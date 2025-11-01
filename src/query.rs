//! Query engine for searching indexed code
//!
//! The query engine loads the memory-mapped cache and executes
//! deterministic searches based on lexical, structural, or symbol patterns.

use anyhow::{Context, Result};

use crate::cache::{CacheManager, SymbolReader, SYMBOLS_BIN};
use crate::content_store::ContentReader;
use crate::models::{Language, SearchResult, Span, SymbolKind};
use crate::trigram::TrigramIndex;

/// Query filter options
#[derive(Debug, Clone)]
pub struct QueryFilter {
    /// Language filter (None = all languages)
    pub language: Option<Language>,
    /// Symbol kind filter (None = all kinds)
    pub kind: Option<SymbolKind>,
    /// Use AST pattern matching (vs lexical search)
    pub use_ast: bool,
    /// Maximum number of results
    pub limit: Option<usize>,
    /// Search symbol definitions only (vs full-text)
    pub symbols_mode: bool,
    /// Show full symbol body (from span.start_line to span.end_line)
    pub expand: bool,
    /// File path filter (substring match)
    pub file_pattern: Option<String>,
    /// Exact symbol name match (no substring matching)
    pub exact: bool,
}

impl Default for QueryFilter {
    fn default() -> Self {
        Self {
            language: None,
            kind: None,
            use_ast: false,
            limit: None,
            symbols_mode: false,
            expand: false,
            file_pattern: None,
            exact: false,
        }
    }
}

/// Manages query execution against the index
pub struct QueryEngine {
    cache: CacheManager,
}

impl QueryEngine {
    /// Create a new query engine with the given cache manager
    pub fn new(cache: CacheManager) -> Self {
        Self { cache }
    }

    /// Execute a query and return matching results
    pub fn search(&self, pattern: &str, filter: QueryFilter) -> Result<Vec<SearchResult>> {
        log::info!("Executing query: pattern='{}', filter={:?}", pattern, filter);

        // Ensure cache exists
        if !self.cache.exists() {
            anyhow::bail!(
                "Index not found. Run 'reflex index' to build the cache first."
            );
        }

        // Step 1: Load symbol reader
        let symbols_path = self.cache.path().join(SYMBOLS_BIN);
        let reader = SymbolReader::open(&symbols_path)
            .context("Failed to open symbols cache")?;

        // Step 2: Execute search based on mode
        let mut results = if filter.symbols_mode {
            // Symbol name search (filtered to symbol definitions only)
            // Searches Tree-sitter parsed symbols: functions, classes, structs, etc.
            if pattern.ends_with('*') {
                // Prefix match: "get_*"
                let prefix = pattern.trim_end_matches('*');
                reader.find_by_prefix(prefix)?
            } else if pattern == "*" {
                // List all symbols
                reader.read_all()?
            } else if pattern.contains('*') {
                // Wildcard match - treat as substring but symbol-only
                let substring = pattern.replace('*', "");
                reader.find_by_symbol_name_only(&substring)?
            } else {
                // Substring match in symbol names only
                reader.find_by_symbol_name_only(pattern)?
            }
        } else {
            // Trigram-based full-text search
            // Searches all file content for any occurrence of the pattern
            self.search_with_trigrams(pattern)?
        };

        // Step 3: Apply filters
        if let Some(lang) = filter.language {
            results.retain(|r| r.lang == lang);
        }

        // Apply kind filter (only relevant for symbol searches)
        // Special case: --kind function also includes methods (methods are functions in classes)
        if let Some(ref kind) = filter.kind {
            results.retain(|r| {
                if matches!(kind, SymbolKind::Function) {
                    // When searching for functions, also include methods
                    matches!(r.kind, SymbolKind::Function | SymbolKind::Method)
                } else {
                    r.kind == *kind
                }
            });
        }

        // Apply file path filter (substring match)
        if let Some(ref file_pattern) = filter.file_pattern {
            results.retain(|r| r.path.contains(file_pattern));
        }

        // Apply exact name filter (only for symbol searches)
        if filter.exact && filter.symbols_mode {
            results.retain(|r| r.symbol == pattern);
        }

        // Expand symbol bodies if requested
        if filter.expand && filter.symbols_mode {
            // Load content store to fetch full symbol bodies
            let content_path = self.cache.path().join("content.bin");
            if let Ok(content_reader) = ContentReader::open(&content_path) {
                for result in &mut results {
                    // Find the file_id for this result's path
                    if let Some(file_id) = Self::find_file_id(&content_reader, &result.path) {
                        // Fetch the full span content
                        if let Ok(content) = content_reader.get_file_content(file_id) {
                            let lines: Vec<&str> = content.lines().collect();
                            let start_idx = (result.span.start_line as usize).saturating_sub(1);
                            let end_idx = (result.span.end_line as usize).min(lines.len());

                            if start_idx < end_idx {
                                let full_body = lines[start_idx..end_idx].join("\n");
                                result.preview = full_body;
                            }
                        }
                    }
                }
            }
        }

        // Step 4: Sort results deterministically (by path, then line number)
        results.sort_by(|a, b| {
            a.path.cmp(&b.path)
                .then_with(|| a.span.start_line.cmp(&b.span.start_line))
        });

        // Step 5: Apply limit
        if let Some(limit) = filter.limit {
            results.truncate(limit);
        }

        log::info!("Query returned {} results", results.len());

        Ok(results)
    }

    /// Search for symbols by exact name match
    pub fn find_symbol(&self, name: &str) -> Result<Vec<SearchResult>> {
        let filter = QueryFilter {
            symbols_mode: true,
            ..Default::default()
        };
        self.search(name, filter)
    }

    /// Search using a Tree-sitter AST pattern
    pub fn search_ast(&self, pattern: &str, lang: Option<Language>) -> Result<Vec<SearchResult>> {
        let filter = QueryFilter {
            language: lang,
            use_ast: true,
            ..Default::default()
        };

        self.search(pattern, filter)
    }

    /// List all symbols of a specific kind
    pub fn list_by_kind(&self, kind: SymbolKind) -> Result<Vec<SearchResult>> {
        let filter = QueryFilter {
            kind: Some(kind),
            symbols_mode: true,
            ..Default::default()
        };

        self.search("*", filter)
    }

    /// Search using trigram-based full-text search
    fn search_with_trigrams(&self, pattern: &str) -> Result<Vec<SearchResult>> {
        // Load content store
        let content_path = self.cache.path().join("content.bin");
        let content_reader = ContentReader::open(&content_path)
            .context("Failed to open content store")?;

        // Load symbol reader to get actual symbol kinds
        let symbols_path = self.cache.path().join(SYMBOLS_BIN);
        let symbol_reader = SymbolReader::open(&symbols_path)
            .context("Failed to open symbols cache")?;

        // Load trigram index from disk (or rebuild if missing)
        let trigrams_path = self.cache.path().join("trigrams.bin");
        let trigram_index = if trigrams_path.exists() {
            match TrigramIndex::load(&trigrams_path) {
                Ok(index) => {
                    log::debug!("Loaded trigram index from disk: {} trigrams, {} files",
                               index.trigram_count(), index.file_count());
                    index
                }
                Err(e) => {
                    log::warn!("Failed to load trigram index from disk: {}", e);
                    log::warn!("Rebuilding trigram index from content store...");
                    Self::rebuild_trigram_index(&content_reader)?
                }
            }
        } else {
            log::debug!("trigrams.bin not found, rebuilding from content store");
            Self::rebuild_trigram_index(&content_reader)?
        };

        // Search using trigrams
        let candidates = trigram_index.search(pattern);
        log::debug!("Found {} candidate locations", candidates.len());

        // Get all symbols for matching against locations
        let all_symbols = symbol_reader.read_all()?;

        // Verify matches and build results
        let mut results = Vec::new();

        for loc in candidates {
            let file_path = trigram_index.get_file(loc.file_id)
                .context("Invalid file_id from trigram search")?;
            let content = content_reader.get_file_content(loc.file_id)?;

            // Find all occurrences of the pattern in this file
            for (line_idx, line) in content.lines().enumerate() {
                if line.contains(pattern) {
                    let line_no = line_idx + 1;
                    let file_path_str = file_path.to_string_lossy().to_string();

                    // Detect language from file extension
                    let ext = file_path.extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("");
                    let lang = Language::from_extension(ext);

                    // Try to find a matching symbol at this location
                    let matching_symbol = all_symbols.iter().find(|sym| {
                        sym.path == file_path_str &&
                        line_no >= sym.span.start_line &&
                        line_no <= sym.span.end_line &&
                        line.contains(&sym.symbol)
                    });

                    if let Some(symbol) = matching_symbol {
                        // Use the actual symbol information
                        results.push(symbol.clone());
                    } else {
                        // Fallback: create a generic text match result
                        results.push(SearchResult {
                            path: file_path_str,
                            lang,
                            kind: SymbolKind::Unknown("text_match".to_string()),
                            symbol: pattern.to_string(),
                            span: Span {
                                start_line: line_no,
                                end_line: line_no,
                                start_col: 0,
                                end_col: 0,
                            },
                            scope: None,
                            preview: line.to_string(),
                        });
                    }
                }
            }
        }

        Ok(results)
    }

    /// Helper function to find file_id in ContentReader by matching path
    fn find_file_id(content_reader: &ContentReader, target_path: &str) -> Option<u32> {
        for file_id in 0..content_reader.file_count() {
            if let Some(path) = content_reader.get_file_path(file_id as u32) {
                if path.to_string_lossy() == target_path {
                    return Some(file_id as u32);
                }
            }
        }
        None
    }

    /// Rebuild trigram index from content store (fallback when trigrams.bin is missing)
    fn rebuild_trigram_index(content_reader: &ContentReader) -> Result<TrigramIndex> {
        log::debug!("Rebuilding trigram index from {} files", content_reader.file_count());
        let mut trigram_index = TrigramIndex::new();

        for file_id in 0..content_reader.file_count() {
            let file_path = content_reader.get_file_path(file_id as u32)
                .context("Invalid file_id")?
                .to_path_buf();
            let content = content_reader.get_file_content(file_id as u32)?;

            let idx = trigram_index.add_file(file_path);
            trigram_index.index_file(idx, content);
        }

        trigram_index.finalize();
        log::debug!("Trigram index rebuilt with {} trigrams", trigram_index.trigram_count());

        Ok(trigram_index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_query_engine_creation() {
        let temp = TempDir::new().unwrap();
        let cache = CacheManager::new(temp.path());
        let engine = QueryEngine::new(cache);

        assert!(engine.cache.path().ends_with(".reflex"));
    }

    #[test]
    fn test_filter_modes() {
        // Test that symbols_mode works as expected
        let filter_fulltext = QueryFilter::default();
        assert!(!filter_fulltext.symbols_mode);

        let filter_symbols = QueryFilter {
            symbols_mode: true,
            ..Default::default()
        };
        assert!(filter_symbols.symbols_mode);

        // Test that kind implies symbols_mode (handled in CLI layer)
        let filter_with_kind = QueryFilter {
            kind: Some(SymbolKind::Function),
            symbols_mode: true,
            ..Default::default()
        };
        assert!(filter_with_kind.symbols_mode);
    }
}
