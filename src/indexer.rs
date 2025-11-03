//! Indexing engine for parsing source code
//!
//! The indexer scans the project directory, parses source files using Tree-sitter,
//! and builds the symbol/token cache for fast querying.

use anyhow::{Context, Result};
use ignore::WalkBuilder;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use crate::cache::CacheManager;
use crate::content_store::ContentWriter;
use crate::models::{IndexConfig, IndexStats, Language};
use crate::trigram::TrigramIndex;

/// Result of processing a single file (used for parallel processing)
struct FileProcessingResult {
    path: PathBuf,
    path_str: String,
    hash: String,
    content: String,
    language: Language,
    line_count: usize,
}

/// Manages the indexing process
pub struct Indexer {
    cache: CacheManager,
    config: IndexConfig,
}

impl Indexer {
    /// Create a new indexer with the given cache manager and config
    pub fn new(cache: CacheManager, config: IndexConfig) -> Self {
        Self { cache, config }
    }

    /// Build or update the index for the given root directory
    pub fn index(&self, root: impl AsRef<Path>, show_progress: bool) -> Result<IndexStats> {
        let root = root.as_ref();
        log::info!("Indexing directory: {:?}", root);

        // Get git state (if in git repo)
        let git_state = crate::git::get_git_state_optional(root)?;
        let branch = git_state
            .as_ref()
            .map(|s| s.branch.clone())
            .unwrap_or_else(|| "_default".to_string());

        if let Some(ref state) = git_state {
            log::info!(
                "Git state: branch='{}', commit='{}', dirty={}",
                state.branch,
                state.commit,
                state.dirty
            );
        } else {
            log::info!("Not a git repository, using default branch");
        }

        // Configure thread pool for parallel processing
        // 0 = auto (use 80% of available cores to avoid locking the system)
        let num_threads = if self.config.parallel_threads == 0 {
            let available_cores = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4);
            // Use 80% of available cores (minimum 1)
            ((available_cores as f64 * 0.8).ceil() as usize).max(1)
        } else {
            self.config.parallel_threads
        };

        log::info!("Using {} threads for parallel indexing (out of {} available)",
                   num_threads,
                   std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4));

        // Ensure cache is initialized
        self.cache.init()?;

        // Load existing hashes for incremental indexing
        let existing_hashes = self.cache.load_hashes()?;
        log::debug!("Loaded {} existing file hashes", existing_hashes.len());

        // Step 1: Walk directory tree and collect files
        let files = self.discover_files(root)?;
        let total_files = files.len();
        log::info!("Discovered {} files to index", total_files);

        // Step 2: Build trigram index + content store
        let mut new_hashes = HashMap::new();
        let mut files_indexed = 0;
        let mut file_metadata: Vec<(String, String, String, usize, usize)> = Vec::new(); // For batch SQLite update

        // Initialize trigram index and content store
        let mut trigram_index = TrigramIndex::new();
        let mut content_writer = ContentWriter::new();

        // Enable batch-flush mode for trigram index if we have lots of files
        if total_files > 10000 {
            let temp_dir = self.cache.path().join("trigram_temp");
            trigram_index.enable_batch_flush(temp_dir)
                .context("Failed to enable batch-flush mode for trigram index")?;
            log::info!("Enabled batch-flush mode for {} files", total_files);
        }

        // Initialize content writer to start streaming writes immediately
        let content_path = self.cache.path().join("content.bin");
        content_writer.init(content_path.clone())
            .context("Failed to initialize content writer")?;

        // Create progress bar (only if requested via --progress flag)
        let pb = if show_progress {
            let pb = ProgressBar::new(total_files as u64);
            pb.set_draw_target(ProgressDrawTarget::stderr());
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} files ({percent}%) {msg}")
                    .unwrap()
                    .progress_chars("=>-")
            );
            // Force updates every 100ms to ensure progress is visible
            pb.enable_steady_tick(std::time::Duration::from_millis(100));
            pb
        } else {
            ProgressBar::hidden()
        };

        // Atomic counter for thread-safe progress updates
        let progress_counter = Arc::new(AtomicU64::new(0));

        let _start_time = Instant::now();

        // Spawn a background thread to update progress bar during parallel processing
        let counter_for_thread = Arc::clone(&progress_counter);
        let pb_clone = pb.clone();
        let progress_thread = if show_progress {
            Some(std::thread::spawn(move || {
                loop {
                    let count = counter_for_thread.load(Ordering::Relaxed);
                    pb_clone.set_position(count);
                    if count >= total_files as u64 {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            }))
        } else {
            None
        };

        // Build a custom thread pool with limited threads
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()
            .context("Failed to create thread pool")?;

        // Process files in batches to avoid OOM on huge codebases
        // Batch size: process 5000 files at a time to limit memory usage
        const BATCH_SIZE: usize = 5000;
        let num_batches = (total_files + BATCH_SIZE - 1) / BATCH_SIZE;
        log::info!("Processing {} files in {} batches of up to {} files",
                   total_files, num_batches, BATCH_SIZE);

        for (batch_idx, batch_files) in files.chunks(BATCH_SIZE).enumerate() {
            log::info!("Processing batch {}/{} ({} files)",
                       batch_idx + 1, num_batches, batch_files.len());

            // Process files in parallel using rayon with custom thread pool
            let counter_clone = Arc::clone(&progress_counter);
            let results: Vec<Option<FileProcessingResult>> = pool.install(|| {
                batch_files
                    .par_iter()
                    .map(|file_path| {
                let path_str = file_path.to_string_lossy().to_string();

                // Read file content once (used for hashing, trigrams, and parsing)
                let content = match std::fs::read_to_string(&file_path) {
                    Ok(c) => c,
                    Err(e) => {
                        log::warn!("Failed to read {}: {}", path_str, e);
                        // Update progress
                        counter_clone.fetch_add(1, Ordering::Relaxed);
                        return None;
                    }
                };

                // Compute hash from content (no duplicate file read!)
                let hash = self.hash_content(content.as_bytes());

                // Detect language
                let ext = file_path.extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");
                let language = Language::from_extension(ext);

                // Count lines in the file
                let line_count = content.lines().count();

                // Update progress atomically
                counter_clone.fetch_add(1, Ordering::Relaxed);

                Some(FileProcessingResult {
                    path: file_path.clone(),
                    path_str,
                    hash,
                    content,
                    language,
                    line_count,
                })
                })
                .collect()
            });

            // Process batch results immediately (streaming approach to minimize memory)
            for result in results.into_iter().flatten() {
                // Add file to trigram index (get file_id)
                let file_id = trigram_index.add_file(result.path.clone());

                // Index file content directly (avoid accumulating all trigrams)
                trigram_index.index_file(file_id, &result.content);

                // Add to content store
                content_writer.add_file(result.path.clone(), &result.content);

                files_indexed += 1;

                // Prepare file metadata for batch database update
                file_metadata.push((
                    result.path_str.clone(),
                    result.hash.clone(),
                    format!("{:?}", result.language),
                    0, // symbol_count is always 0 now
                    result.line_count
                ));

                new_hashes.insert(result.path_str, result.hash);
            }

            // Flush trigram index batch to disk if batch-flush mode is enabled
            if total_files > 10000 {
                if show_progress {
                    pb.set_message(format!("Flushing batch {}/{}...", batch_idx + 1, num_batches));
                }
                trigram_index.flush_batch()
                    .context("Failed to flush trigram batch")?;
            }
        }

        // Wait for progress thread to finish
        if let Some(thread) = progress_thread {
            let _ = thread.join();
        }

        // Update progress bar to final count
        if show_progress {
            let final_count = progress_counter.load(Ordering::Relaxed);
            pb.set_position(final_count);
        }

        // Finalize trigram index (sort and deduplicate posting lists)
        if show_progress {
            pb.set_message("Finalizing trigram index...".to_string());
        }
        trigram_index.finalize();

        // Update progress bar message for post-processing
        if show_progress {
            pb.set_message("Writing file metadata to database...".to_string());
        }

        // Batch write all file metadata to SQLite in one transaction
        if !file_metadata.is_empty() {
            self.cache.batch_update_files(&file_metadata)
                .context("Failed to batch update file metadata")?;
            log::info!("Wrote metadata for {} files to database", file_metadata.len());
        }

        // Record files for this branch (for branch-aware indexing)
        if show_progress {
            pb.set_message("Recording branch files...".to_string());
        }

        // Prepare data for batch recording
        let branch_files: Vec<(String, String)> = file_metadata
            .iter()
            .map(|(path, hash, _, _, _)| (path.clone(), hash.clone()))
            .collect();

        // Batch record all files in a single transaction
        if !branch_files.is_empty() {
            self.cache.batch_record_branch_files(
                &branch_files,
                &branch,
                git_state.as_ref().map(|s| s.commit.as_str()),
            )?;
        }

        // Update branch metadata
        self.cache.update_branch_metadata(
            &branch,
            git_state.as_ref().map(|s| s.commit.as_str()),
            file_metadata.len(),
            git_state.as_ref().map(|s| s.dirty).unwrap_or(false),
        )?;

        log::info!("Indexed {} files", files_indexed);

        // Step 3: Write trigram index
        if show_progress {
            pb.set_message("Writing trigram index...".to_string());
        }
        let trigrams_path = self.cache.path().join("trigrams.bin");
        log::info!("Writing trigram index with {} trigrams to trigrams.bin",
                   trigram_index.trigram_count());

        if show_progress {
            pb.set_message("Writing trigram index...".to_string());
        }
        trigram_index.write(&trigrams_path)
            .context("Failed to write trigram index")?;
        log::info!("Wrote {} files to trigrams.bin", trigram_index.file_count());

        // Step 4: Finalize content store (already been writing incrementally)
        if show_progress {
            pb.set_message("Finalizing content store...".to_string());
        }
        content_writer.finalize_if_needed()
            .context("Failed to finalize content store")?;
        log::info!("Wrote {} files ({} bytes) to content.bin",
                   content_writer.file_count(), content_writer.content_size());

        // Step 5: Update SQLite statistics from database totals
        if show_progress {
            pb.set_message("Updating statistics...".to_string());
        }
        // Note: Hashes are already persisted to SQLite via cache.update_file() in the loop above
        self.cache.update_stats()?;

        pb.finish_with_message("Indexing complete");

        // Return stats
        let stats = self.cache.stats()?;
        log::info!("Indexing complete: {} files",
                   stats.total_files);

        Ok(stats)
    }

    /// Discover all indexable files in the directory tree
    fn discover_files(&self, root: &Path) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        let walker = WalkBuilder::new(root)
            .follow_links(self.config.follow_symlinks)
            .build();

        for entry in walker {
            let entry = entry?;
            let path = entry.path();

            // Only process files (not directories)
            if !entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                continue;
            }

            // Check if should be indexed
            if self.should_index(path) {
                files.push(path.to_path_buf());
            }
        }

        Ok(files)
    }

    /// Check if a file should be indexed based on config
    fn should_index(&self, path: &Path) -> bool {
        // Check file extension for supported languages
        let ext = match path.extension() {
            Some(ext) => ext.to_string_lossy(),
            None => return false,
        };

        let lang = Language::from_extension(&ext);

        // Only index files for languages with parser implementations
        if !lang.is_supported() {
            if !matches!(lang, Language::Unknown) {
                log::debug!("Skipping {} ({:?} parser not yet implemented)",
                           path.display(), lang);
            }
            return false;
        }

        // Check file size limits
        if let Ok(metadata) = std::fs::metadata(path) {
            if metadata.len() > self.config.max_file_size as u64 {
                log::debug!("Skipping {} (too large: {} bytes)",
                           path.display(), metadata.len());
                return false;
            }
        }

        // TODO: Check include/exclude patterns when glob support is added
        // For now, accept all files with supported language extensions

        true
    }

    /// Compute blake3 hash from file contents for change detection
    fn hash_content(&self, content: &[u8]) -> String {
        let hash = blake3::hash(content);
        hash.to_hex().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_indexer_creation() {
        let temp = TempDir::new().unwrap();
        let cache = CacheManager::new(temp.path());
        let config = IndexConfig::default();
        let indexer = Indexer::new(cache, config);

        assert!(indexer.cache.path().ends_with(".reflex"));
    }
}
