//! Indexing engine for parsing source code
//!
//! The indexer scans the project directory, parses source files using Tree-sitter,
//! and builds the symbol/token cache for fast querying.

use anyhow::{Context, Result};
use ignore::WalkBuilder;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::cache::{CacheManager, SymbolReader, SymbolWriter, SYMBOLS_BIN};
use crate::models::{IndexConfig, IndexStats, Language, SearchResult};
use crate::parsers::ParserFactory;

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
    pub fn index(&self, root: impl AsRef<Path>) -> Result<IndexStats> {
        let root = root.as_ref();
        log::info!("Indexing directory: {:?}", root);

        // Ensure cache is initialized
        self.cache.init()?;

        // Load existing hashes for incremental indexing
        let existing_hashes = self.cache.load_hashes()?;
        log::debug!("Loaded {} existing file hashes", existing_hashes.len());

        // Load existing symbols to preserve them during incremental indexing
        let symbols_path = self.cache.path().join(SYMBOLS_BIN);
        let existing_symbols = if symbols_path.exists() {
            match SymbolReader::open(&symbols_path) {
                Ok(reader) => reader.read_all().unwrap_or_else(|e| {
                    log::warn!("Failed to load existing symbols: {}", e);
                    Vec::new()
                }),
                Err(e) => {
                    log::warn!("Failed to open existing symbols: {}", e);
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        };
        log::debug!("Loaded {} existing symbols from cache", existing_symbols.len());

        // Step 1: Walk directory tree and collect files
        let files = self.discover_files(root)?;
        log::info!("Discovered {} files to index", files.len());

        // Step 2: Parse files and extract symbols
        let mut new_hashes = HashMap::new();
        let mut all_symbols = Vec::new();
        let mut files_indexed = 0;

        // Build a map of path -> symbols from existing cache for quick lookup
        let mut existing_symbols_by_path: HashMap<String, Vec<SearchResult>> = HashMap::new();
        for symbol in existing_symbols {
            existing_symbols_by_path
                .entry(symbol.path.clone())
                .or_insert_with(Vec::new)
                .push(symbol);
        }

        for file_path in files {
            // Compute hash for incremental indexing
            let hash = self.hash_file(&file_path)?;
            let path_str = file_path.to_string_lossy().to_string();

            // Check if file changed
            if let Some(existing_hash) = existing_hashes.get(&path_str) {
                if existing_hash == &hash {
                    log::debug!("Skipping unchanged file: {}", path_str);
                    // Preserve existing symbols from this file
                    if let Some(symbols) = existing_symbols_by_path.get(&path_str) {
                        all_symbols.extend(symbols.clone());
                        log::debug!("  Preserved {} symbols from cache", symbols.len());
                    }
                    new_hashes.insert(path_str, hash);
                    continue;
                }
            }

            // File is new or changed - parse it
            log::debug!("Parsing file: {}", path_str);
            match self.parse_file(&file_path) {
                Ok(symbols) => {
                    let symbol_count = symbols.len();

                    // Detect language
                    let ext = file_path.extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("");
                    let language = Language::from_extension(ext);

                    // Update file metadata in database
                    self.cache.update_file(
                        &path_str,
                        &hash,
                        &format!("{:?}", language),
                        symbol_count
                    )?;

                    all_symbols.extend(symbols);
                    new_hashes.insert(path_str.clone(), hash);
                    files_indexed += 1;
                    log::debug!("  Extracted {} symbols from {}", symbol_count, path_str);
                }
                Err(e) => {
                    log::warn!("Failed to parse {}: {}", path_str, e);
                    // Still track the file to avoid retrying
                    new_hashes.insert(path_str, hash);
                }
            }
        }

        log::info!("Parsed {} files, extracted {} symbols", files_indexed, all_symbols.len());

        // Step 3: Write symbols to cache (only if we have new symbols)
        if !all_symbols.is_empty() {
            let mut writer = SymbolWriter::new();
            writer.add_all(&all_symbols);
            let symbols_path = self.cache.path().join(SYMBOLS_BIN);
            writer.write(&symbols_path)
                .context("Failed to write symbols to cache")?;
            log::info!("Wrote {} symbols to {}", writer.len(), SYMBOLS_BIN);
        } else {
            log::info!("No new symbols to write (incremental indexing skipped all files)");
        }

        // Step 4: Save updated hashes
        self.cache.save_hashes(&new_hashes)?;

        // Step 5: Update SQLite statistics from database totals
        self.cache.update_stats()?;

        // Return stats
        let stats = self.cache.stats()?;
        log::info!("Indexing complete: {} files, {} symbols",
                   stats.total_files, stats.total_symbols);

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
        if matches!(lang, Language::Unknown) {
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
        // For now, accept all files with known language extensions

        true
    }

    /// Parse a single file and extract symbols
    fn parse_file(&self, path: &Path) -> Result<Vec<SearchResult>> {
        // Read file contents
        let source = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", path.display()))?;

        // Detect language from extension
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let language = Language::from_extension(ext);

        // Parse with appropriate Tree-sitter grammar
        let path_str = path.to_string_lossy().to_string();
        let symbols = ParserFactory::parse(&path_str, &source, language)
            .with_context(|| format!("Failed to parse file: {}", path.display()))?;

        Ok(symbols)
    }

    /// Compute blake3 hash of a file for change detection
    fn hash_file(&self, path: &Path) -> Result<String> {
        let contents = std::fs::read(path)?;
        let hash = blake3::hash(&contents);
        Ok(hash.to_hex().to_string())
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
