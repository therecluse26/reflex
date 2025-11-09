//! Background symbol indexer for transparent caching
//!
//! This module provides background processing to parse symbols from all indexed
//! files and populate the symbol cache. It runs as a separate process spawned by
//! `rfx index`, allowing users to continue working while symbols are being indexed.

use anyhow::{Context, Result};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::cache::CacheManager;
use crate::parsers::ParserFactory;
use crate::symbol_cache::SymbolCache;

/// Lock file name to prevent concurrent indexing
const LOCK_FILE: &str = "indexing.lock";

/// Status file name for progress tracking
const STATUS_FILE: &str = "indexing.status";

/// Indexing progress status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingStatus {
    /// Current state of the indexer
    pub state: IndexerState,
    /// Total files to process
    pub total_files: usize,
    /// Files processed so far
    pub processed_files: usize,
    /// Files that had symbols cached
    pub cached_files: usize,
    /// Files that were newly parsed
    pub parsed_files: usize,
    /// Files that failed to parse
    pub failed_files: usize,
    /// Start time (ISO 8601)
    pub started_at: String,
    /// Last update time (ISO 8601)
    pub updated_at: String,
    /// Completion time (ISO 8601, None if not finished)
    pub completed_at: Option<String>,
    /// Error message if failed
    pub error: Option<String>,
}

/// Indexer state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum IndexerState {
    /// Indexer is currently running
    Running,
    /// Indexer completed successfully
    Completed,
    /// Indexer failed with error
    Failed,
}

/// Background symbol indexer
pub struct BackgroundIndexer {
    workspace_path: PathBuf,
    cache_path: PathBuf,
    status: IndexingStatus,
    batch_size: usize,
}

impl BackgroundIndexer {
    /// Create a new background indexer
    ///
    /// # Arguments
    /// * `workspace_path` - Path to the workspace root (e.g., ".")
    pub fn new(workspace_path: &Path) -> Result<Self> {
        let now = chrono::Utc::now().to_rfc3339();

        // Create CacheManager to get the cache directory path
        let cache_mgr = CacheManager::new(workspace_path);
        let cache_path = cache_mgr.path().to_path_buf();

        Ok(Self {
            workspace_path: workspace_path.to_path_buf(),
            cache_path,
            status: IndexingStatus {
                state: IndexerState::Running,
                total_files: 0,
                processed_files: 0,
                cached_files: 0,
                parsed_files: 0,
                failed_files: 0,
                started_at: now.clone(),
                updated_at: now,
                completed_at: None,
                error: None,
            },
            batch_size: 500, // Batch symbol writes for performance (increased for better throughput)
        })
    }

    /// Check if an indexing process is already running
    pub fn is_running(cache_dir: &Path) -> bool {
        cache_dir.join(LOCK_FILE).exists()
    }

    /// Get the current indexing status (if available)
    pub fn get_status(cache_dir: &Path) -> Result<Option<IndexingStatus>> {
        let status_path = cache_dir.join(STATUS_FILE);

        if !status_path.exists() {
            return Ok(None);
        }

        let status_json = std::fs::read_to_string(&status_path)
            .context("Failed to read indexing status")?;

        let status: IndexingStatus = serde_json::from_str(&status_json)
            .context("Failed to parse indexing status")?;

        Ok(Some(status))
    }

    /// Acquire lock file (returns error if already locked)
    fn acquire_lock(&self) -> Result<File> {
        let lock_path = self.cache_path.join(LOCK_FILE);

        if lock_path.exists() {
            anyhow::bail!("Indexing already in progress (lock file exists)");
        }

        let mut lock_file = File::create(&lock_path)
            .context("Failed to create lock file")?;

        // Write PID to lock file for debugging
        let pid = std::process::id();
        writeln!(lock_file, "{}", pid)?;

        log::debug!("Acquired indexing lock (PID: {})", pid);
        Ok(lock_file)
    }

    /// Release lock file
    fn release_lock(&self) -> Result<()> {
        let lock_path = self.cache_path.join(LOCK_FILE);

        if lock_path.exists() {
            std::fs::remove_file(&lock_path)
                .context("Failed to remove lock file")?;
            log::debug!("Released indexing lock");
        }

        Ok(())
    }

    /// Write current status to status file
    fn write_status(&mut self) -> Result<()> {
        self.status.updated_at = chrono::Utc::now().to_rfc3339();

        let status_path = self.cache_path.join(STATUS_FILE);
        let status_json = serde_json::to_string_pretty(&self.status)
            .context("Failed to serialize status")?;

        std::fs::write(&status_path, status_json)
            .context("Failed to write status file")?;

        Ok(())
    }

    /// Run the background indexer
    ///
    /// This processes all indexed files, parsing symbols and caching them.
    /// Progress is written to `.reflex/indexing.status` and can be monitored.
    pub fn run(&mut self) -> Result<()> {
        let start_time = Instant::now();

        // Acquire lock (fails if already running)
        let _lock_file = self.acquire_lock()
            .context("Failed to acquire indexing lock")?;

        // Ensure lock is released even on panic
        let cache_path = self.cache_path.clone();
        let _guard = scopeguard::guard((), move |_| {
            let _ = std::fs::remove_file(cache_path.join(LOCK_FILE));
        });

        // Run indexing
        let result = self.run_internal();

        // Update status based on result
        match result {
            Ok(()) => {
                self.status.state = IndexerState::Completed;
                self.status.completed_at = Some(chrono::Utc::now().to_rfc3339());
                log::info!(
                    "Symbol indexing completed: {} files processed ({} cached, {} parsed, {} failed) in {:.2}s",
                    self.status.processed_files,
                    self.status.cached_files,
                    self.status.parsed_files,
                    self.status.failed_files,
                    start_time.elapsed().as_secs_f64()
                );
            }
            Err(ref e) => {
                self.status.state = IndexerState::Failed;
                self.status.error = Some(format!("{:#}", e));
                self.status.completed_at = Some(chrono::Utc::now().to_rfc3339());
                log::error!("Symbol indexing failed: {:#}", e);
            }
        }

        // Write final status
        self.write_status()?;

        // Release lock
        self.release_lock()?;

        result
    }

    /// Internal indexing implementation with parallel processing
    fn run_internal(&mut self) -> Result<()> {
        log::info!("Starting background symbol indexing");

        // Calculate thread pool size (25-30% of available CPUs)
        let num_cpus = num_cpus::get();
        let num_threads = ((num_cpus as f32 * 0.275).ceil() as usize).max(1);

        log::info!(
            "Using {} threads for background indexing ({} CPUs available, ~27.5% utilization)",
            num_threads,
            num_cpus
        );

        // Create custom thread pool with limited threads
        let thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()
            .context("Failed to create thread pool")?;

        // Open cache manager and symbol cache
        let cache_mgr = CacheManager::new(&self.workspace_path);
        let symbol_cache = SymbolCache::open(&self.cache_path)
            .context("Failed to open symbol cache")?;

        // Get all indexed files
        let files = cache_mgr.list_files()
            .context("Failed to list indexed files")?;

        // Get file hashes across all branches (background indexer processes all files)
        let file_hashes = cache_mgr.load_all_hashes()
            .context("Failed to load file hashes")?;

        self.status.total_files = files.len();
        log::info!("Found {} indexed files to process", files.len());

        // Write initial status
        self.write_status()?;

        // Shared state for status tracking
        let status_mutex = Arc::new(Mutex::new((0usize, 0usize, 0usize))); // (cached, parsed, failed)

        // Process files in batches
        let batch_size = self.batch_size;
        let mut processed = 0;

        for chunk in files.chunks(batch_size) {
            // Filter out cached files (sequential check is fast)
            let files_to_parse: Vec<_> = chunk
                .iter()
                .filter_map(|file| {
                    let path = &file.path;
                    let file_hash = file_hashes.get(path)?;

                    // Check if already cached
                    if symbol_cache.get(path, file_hash).ok().flatten().is_some() {
                        // Update cached count
                        let mut status = status_mutex.lock().unwrap();
                        status.0 += 1;
                        None
                    } else {
                        Some((path.clone(), file_hash.clone(), file.language.clone()))
                    }
                })
                .collect();

            // Parse files in parallel using custom thread pool
            let parsed_results: Vec<_> = thread_pool.install(|| {
                files_to_parse
                    .par_iter()
                    .map(|(path, file_hash, _language)| {
                        match self.parse_symbols(path, _language) {
                            Ok(symbols) => {
                                // Update parsed count
                                let mut status = status_mutex.lock().unwrap();
                                status.1 += 1;
                                Some((path.clone(), file_hash.clone(), symbols))
                            }
                            Err(e) => {
                                log::warn!("Failed to parse symbols from {}: {}", path, e);
                                // Update failed count
                                let mut status = status_mutex.lock().unwrap();
                                status.2 += 1;
                                None
                            }
                        }
                    })
                    .flatten()
                    .collect()
            });

            // Write batch to cache (sequential - SQLite limitation)
            if !parsed_results.is_empty() {
                if let Err(e) = symbol_cache.batch_set(&parsed_results) {
                    log::error!("Failed to write symbol batch: {}", e);
                    let mut status = status_mutex.lock().unwrap();
                    status.2 += parsed_results.len();
                }
            }

            // Update status counters
            processed += chunk.len();
            {
                let status = status_mutex.lock().unwrap();
                self.status.cached_files = status.0;
                self.status.parsed_files = status.1;
                self.status.failed_files = status.2;
                self.status.processed_files = processed;
            }

            // Write status every batch
            if processed % 500 < batch_size {
                if let Err(e) = self.write_status() {
                    log::warn!("Failed to write status: {}", e);
                }
            }
        }

        // Final status update
        self.status.processed_files = files.len();
        self.write_status()?;

        // Cleanup stale entries
        let removed = symbol_cache.cleanup_stale()
            .context("Failed to cleanup stale symbols")?;

        if removed > 0 {
            log::info!("Cleaned up {} stale symbol entries", removed);
        }

        Ok(())
    }

    /// Parse symbols from a file
    fn parse_symbols(&self, path: &str, _language: &str) -> Result<Vec<crate::models::SearchResult>> {
        // Read file contents
        let source = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", path))?;

        // Detect language from file extension
        let extension = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let language = crate::models::Language::from_extension(extension);

        // Parse with appropriate parser
        let symbols = ParserFactory::parse(path, &source, language)
            .with_context(|| format!("Failed to parse symbols from: {}", path))?;

        Ok(symbols)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::CacheManager;
    use tempfile::TempDir;

    #[test]
    fn test_indexer_lock() {
        let temp = TempDir::new().unwrap();
        let cache_mgr = CacheManager::new(temp.path());
        cache_mgr.init().unwrap();

        assert!(!BackgroundIndexer::is_running(cache_mgr.path()));

        let mut indexer = BackgroundIndexer::new(temp.path()).unwrap();
        let _lock = indexer.acquire_lock().unwrap();

        assert!(BackgroundIndexer::is_running(cache_mgr.path()));

        indexer.release_lock().unwrap();
        assert!(!BackgroundIndexer::is_running(cache_mgr.path()));
    }

    #[test]
    fn test_indexer_lock_prevents_concurrent() {
        let temp = TempDir::new().unwrap();
        let cache_mgr = CacheManager::new(temp.path());
        cache_mgr.init().unwrap();

        let mut indexer1 = BackgroundIndexer::new(temp.path()).unwrap();
        let _lock1 = indexer1.acquire_lock().unwrap();

        let mut indexer2 = BackgroundIndexer::new(temp.path()).unwrap();
        let result = indexer2.acquire_lock();

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already in progress"));
    }

    #[test]
    fn test_indexer_status_write() {
        let temp = TempDir::new().unwrap();
        let cache_mgr = CacheManager::new(temp.path());
        cache_mgr.init().unwrap();

        let mut indexer = BackgroundIndexer::new(temp.path()).unwrap();
        indexer.status.total_files = 100;
        indexer.status.processed_files = 50;

        indexer.write_status().unwrap();

        let status = BackgroundIndexer::get_status(cache_mgr.path()).unwrap();
        assert!(status.is_some());

        let status = status.unwrap();
        assert_eq!(status.total_files, 100);
        assert_eq!(status.processed_files, 50);
        assert_eq!(status.state, IndexerState::Running);
    }

    #[test]
    fn test_indexer_status_read_nonexistent() {
        let temp = TempDir::new().unwrap();
        let cache_mgr = CacheManager::new(temp.path());
        cache_mgr.init().unwrap();

        let status = BackgroundIndexer::get_status(cache_mgr.path()).unwrap();
        assert!(status.is_none());
    }

    #[test]
    fn test_indexer_run_empty_index() {
        let temp = TempDir::new().unwrap();
        let cache_mgr = CacheManager::new(temp.path());
        cache_mgr.init().unwrap();

        let mut indexer = BackgroundIndexer::new(temp.path()).unwrap();
        let result = indexer.run();

        assert!(result.is_ok());
        assert_eq!(indexer.status.state, IndexerState::Completed);
        assert_eq!(indexer.status.processed_files, 0);
        assert_eq!(indexer.status.total_files, 0);
    }
}
