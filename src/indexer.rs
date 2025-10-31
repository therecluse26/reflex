//! Indexing engine for parsing source code
//!
//! The indexer scans the project directory, parses source files using Tree-sitter,
//! and builds the symbol/token cache for fast querying.

use anyhow::Result;
use std::path::Path;

use crate::cache::CacheManager;
use crate::models::{IndexConfig, IndexStats, Language};

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

        // TODO: Implement the actual indexing logic:
        // 1. Walk the directory tree (respecting .gitignore and config patterns)
        // 2. For each source file:
        //    a. Compute blake3 hash
        //    b. Compare with existing hash to check if reindex needed
        //    c. Parse with Tree-sitter if changed
        //    d. Extract symbols (functions, classes, etc.)
        //    e. Extract tokens for lexical search
        //    f. Write to cache files (symbols.bin, tokens.bin)
        // 3. Update hashes.json
        // 4. Update meta.db with statistics

        // Placeholder: return empty stats
        let stats = IndexStats {
            total_files: 0,
            total_symbols: 0,
            index_size_bytes: 0,
            last_updated: chrono::Utc::now().to_rfc3339(),
        };

        log::info!("Indexing complete: {} files, {} symbols",
                   stats.total_files, stats.total_symbols);

        Ok(stats)
    }

    /// Check if a file should be indexed based on config
    fn should_index(&self, path: &Path) -> bool {
        // TODO: Implement filtering logic:
        // - Check against include/exclude patterns
        // - Check file extension against supported languages
        // - Check file size limits
        // - Respect .gitignore

        if let Some(ext) = path.extension() {
            let ext = ext.to_string_lossy();
            let lang = Language::from_extension(&ext);

            // For now, only index known languages
            !matches!(lang, Language::Unknown)
        } else {
            false
        }
    }

    /// Parse a single file and extract symbols
    fn parse_file(&self, _path: &Path) -> Result<Vec<crate::models::SearchResult>> {
        // TODO: Implement Tree-sitter parsing:
        // 1. Detect language from file extension
        // 2. Load appropriate Tree-sitter grammar
        // 3. Parse file into AST
        // 4. Walk AST to extract:
        //    - Function definitions
        //    - Class/struct definitions
        //    - Constants
        //    - Imports/exports
        //    - etc.
        // 5. Return SearchResults with spans and context

        Ok(vec![])
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
