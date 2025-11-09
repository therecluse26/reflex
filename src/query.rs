//! Query engine for searching indexed code
//!
//! The query engine loads the memory-mapped cache and executes
//! deterministic searches based on lexical, structural, or symbol patterns.

use anyhow::{Context, Result};
use regex::Regex;

use crate::cache::CacheManager;
use crate::content_store::ContentReader;
use crate::models::{
    IndexStatus, IndexWarning, IndexWarningDetails, Language, QueryResponse, SearchResult, Span,
    SymbolKind,
};
use crate::parsers::ParserFactory;
use crate::regex_trigrams::extract_trigrams_from_regex;
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
    /// Use regex pattern matching
    pub use_regex: bool,
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
    /// Use substring matching instead of word-boundary matching (opt-in, expansive)
    pub use_contains: bool,
    /// Query timeout in seconds (0 = no timeout)
    pub timeout_secs: u64,
    /// Glob patterns to include (empty = all files)
    pub glob_patterns: Vec<String>,
    /// Glob patterns to exclude (applied after includes)
    pub exclude_patterns: Vec<String>,
    /// Return only unique file paths (deduplicated)
    pub paths_only: bool,
    /// Pagination offset (skip first N results after sorting)
    pub offset: Option<usize>,
}

impl Default for QueryFilter {
    fn default() -> Self {
        Self {
            language: None,
            kind: None,
            use_ast: false,
            use_regex: false,
            limit: Some(100),  // Default: limit to 100 results for token efficiency
            symbols_mode: false,
            expand: false,
            file_pattern: None,
            exact: false,
            use_contains: false,  // Default: word-boundary matching
            timeout_secs: 30, // 30 seconds default timeout
            glob_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
            paths_only: false,
            offset: None,
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

    /// Execute a query and return matching results with index metadata
    ///
    /// This is the preferred method for programmatic/JSON output as it includes
    /// index freshness information that AI agents can use to decide whether to re-index.
    pub fn search_with_metadata(&self, pattern: &str, filter: QueryFilter) -> Result<QueryResponse> {
        log::info!("Executing query with metadata: pattern='{}', filter={:?}", pattern, filter);

        // Ensure cache exists
        if !self.cache.exists() {
            anyhow::bail!(
                "Index not found. Run 'rfx index' to build the cache first."
            );
        }

        // Validate cache integrity
        if let Err(e) = self.cache.validate() {
            anyhow::bail!(
                "Cache appears to be corrupted: {}. Run 'rfx clear' followed by 'rfx index' to rebuild.",
                e
            );
        }

        // Get index status and warning (without printing warnings to stderr)
        let (status, can_trust_results, warning) = self.get_index_status()?;

        // Execute the search
        let (results, total) = self.search_internal(pattern, filter.clone())?;

        // Build pagination metadata
        use crate::models::PaginationInfo;
        let pagination = PaginationInfo {
            total,
            count: results.len(),
            offset: filter.offset.unwrap_or(0),
            limit: filter.limit,
            has_more: total > filter.offset.unwrap_or(0) + results.len(),
        };

        Ok(QueryResponse {
            status,
            can_trust_results,
            warning,
            pagination,
            results,
        })
    }

    /// Execute a query and return matching results (legacy method)
    ///
    /// This method prints warnings to stderr and returns just the results.
    /// For programmatic use, prefer `search_with_metadata()`.
    pub fn search(&self, pattern: &str, filter: QueryFilter) -> Result<Vec<SearchResult>> {
        log::info!("Executing query: pattern='{}', filter={:?}", pattern, filter);

        // Ensure cache exists
        if !self.cache.exists() {
            anyhow::bail!(
                "Index not found. Run 'rfx index' to build the cache first."
            );
        }

        // Validate cache integrity
        if let Err(e) = self.cache.validate() {
            anyhow::bail!(
                "Cache appears to be corrupted: {}. Run 'rfx clear' followed by 'rfx index' to rebuild.",
                e
            );
        }

        // Show non-blocking warnings about branch state and staleness
        self.check_index_freshness()?;

        // Execute the search (discard total count - legacy method doesn't use it)
        let (results, _total_count) = self.search_internal(pattern, filter)?;
        Ok(results)
    }

    /// Internal search implementation (used by both search methods)
    /// Returns (results, total_count) where total_count is the count before offset/limit
    fn search_internal(&self, pattern: &str, filter: QueryFilter) -> Result<(Vec<SearchResult>, usize)> {
        use std::time::{Duration, Instant};

        // Start timeout timer if configured
        let start_time = Instant::now();
        let timeout = if filter.timeout_secs > 0 {
            Some(Duration::from_secs(filter.timeout_secs))
        } else {
            None
        };

        // KEYWORD DETECTION (early): Check if this is a keyword query that should scan ALL files
        // When a user searches for a language keyword (like "class", "function") with --symbols or --kind,
        // we interpret it as "list all symbols of that type" and should scan ALL files,
        // not just the first 100 candidates from trigram search.
        //
        // Requirements for keyword query mode:
        // 1. Symbol mode active (--symbols or --kind)
        // 2. Pattern matches a keyword in ANY supported language
        //
        // Note: --lang is optional. If specified, language filtering happens naturally in Phase 2/3.
        let is_keyword_query = if filter.symbols_mode || filter.kind.is_some() {
            ParserFactory::get_all_keywords().contains(&pattern)
        } else {
            false
        };

        // KEYWORD-TO-KIND MAPPING: If user searches for a keyword without --kind, infer the kind
        // Example: "class" → SymbolKind::Class, "function" → SymbolKind::Function
        // This ensures keyword queries return only the relevant symbol type
        let mut filter = filter.clone();  // Clone so we can modify it
        if is_keyword_query && filter.kind.is_none() {
            if let Some(inferred_kind) = Self::keyword_to_kind(pattern) {
                log::info!("Keyword '{}' mapped to kind {:?} (auto-inferred)", pattern, inferred_kind);
                filter.kind = Some(inferred_kind);
            }
        }

        // PHASE 1: Get initial candidates (choose search strategy)
        let mut results = if is_keyword_query {
            // KEYWORD QUERY MODE: Scan all files (or files of target language if --lang specified)
            // This ensures we find ALL classes/functions/etc, not just those in the first 100 trigram matches
            if let Some(lang) = filter.language {
                log::info!("Keyword query detected for '{}' - scanning all {:?} files (bypassing trigram search)",
                          pattern, lang);
            } else {
                log::info!("Keyword query detected for '{}' - scanning all files (bypassing trigram search)", pattern);
            }
            self.get_all_language_files(&filter)?
        } else if filter.use_regex {
            // Regex pattern search with trigram optimization
            self.get_regex_candidates(pattern, timeout.as_ref(), &start_time)?
        } else {
            // Standard trigram-based full-text search
            self.get_trigram_candidates(pattern, &filter)?
        };


        // Check timeout after Phase 1
        if let Some(timeout_duration) = timeout {
            if start_time.elapsed() > timeout_duration {
                anyhow::bail!(
                    "Query timeout exceeded ({} seconds).\n\
                     \n\
                     The query took too long to complete. Try one of these approaches:\n\
                     • Use a more specific search pattern (longer patterns = faster search)\n\
                     • Add a language filter with --lang to narrow the search space\n\
                     • Add a file filter with --file to search specific directories\n\
                     • Increase the timeout with --timeout <seconds>\n\
                     \n\
                     Example: rfx query \"{}\" --lang rust --timeout 60",
                    filter.timeout_secs,
                    pattern
                );
            }
        }

        // EARLY LANGUAGE FILTER: Apply language filtering BEFORE early limiting
        // This ensures we only parse files matching the language filter in Phase 2
        // Critical for non-keyword queries to work correctly
        //
        // Skip for keyword queries - those candidates are already pre-filtered by language
        if !is_keyword_query {
            if let Some(lang) = filter.language {
                let before_count = results.len();
                results.retain(|r| r.lang == lang);
                log::debug!(
                    "Language filter ({:?}): reduced {} candidates to {} candidates",
                    lang,
                    before_count,
                    results.len()
                );
            }
        }

        // DETERMINISTIC SORTING: Sort candidates early for deterministic results
        // This ensures results are always returned in the same order
        if filter.symbols_mode || filter.kind.is_some() || filter.use_ast {
            results.sort_by(|a, b| {
                a.path.cmp(&b.path)
                    .then_with(|| a.span.start_line.cmp(&b.span.start_line))
            });

            // Warn if many candidates need parsing (helps users refine queries)
            let candidate_count = results.len();
            if candidate_count > 1000 {
                log::warn!(
                    "Pattern '{}' matched {} files - parsing may take some time. Consider using --file, --glob, or a more specific pattern to narrow the search.",
                    pattern,
                    candidate_count
                );
            } else if candidate_count > 100 {
                log::info!("Parsing {} candidate files for symbol extraction", candidate_count);
            }
        }

        // PHASE 2: Enrich with symbol information or AST pattern matching (if needed)
        if filter.use_ast {
            // AST pattern matching: Execute Tree-sitter query on candidate files
            results = self.enrich_with_ast(results, pattern, filter.language)?;
        } else if filter.symbols_mode || filter.kind.is_some() {
            // Symbol enrichment: Parse candidate files and extract symbol definitions
            results = self.enrich_with_symbols(results, pattern, &filter)?;
        }

        // PHASE 3: Apply filters
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

        // Apply glob pattern filters
        if !filter.glob_patterns.is_empty() || !filter.exclude_patterns.is_empty() {
            use globset::{Glob, GlobSetBuilder};

            // Build include matcher (if patterns specified)
            let include_matcher = if !filter.glob_patterns.is_empty() {
                let mut builder = GlobSetBuilder::new();
                for pattern in &filter.glob_patterns {
                    // Normalize pattern to ensure LLM-generated patterns work correctly
                    let normalized = Self::normalize_glob_pattern(pattern);
                    match Glob::new(&normalized) {
                        Ok(glob) => {
                            builder.add(glob);
                        }
                        Err(e) => {
                            log::warn!("Invalid glob pattern '{}': {}", pattern, e);
                        }
                    }
                }
                match builder.build() {
                    Ok(matcher) => Some(matcher),
                    Err(e) => {
                        log::warn!("Failed to build glob matcher: {}", e);
                        None
                    }
                }
            } else {
                None
            };

            // Build exclude matcher (if patterns specified)
            let exclude_matcher = if !filter.exclude_patterns.is_empty() {
                let mut builder = GlobSetBuilder::new();
                for pattern in &filter.exclude_patterns {
                    // Normalize pattern to ensure LLM-generated patterns work correctly
                    let normalized = Self::normalize_glob_pattern(pattern);
                    match Glob::new(&normalized) {
                        Ok(glob) => {
                            builder.add(glob);
                        }
                        Err(e) => {
                            log::warn!("Invalid exclude pattern '{}': {}", pattern, e);
                        }
                    }
                }
                match builder.build() {
                    Ok(matcher) => Some(matcher),
                    Err(e) => {
                        log::warn!("Failed to build exclude matcher: {}", e);
                        None
                    }
                }
            } else {
                None
            };

            // Apply filters
            results.retain(|r| {
                // If include patterns specified, path must match at least one
                let included = if let Some(ref matcher) = include_matcher {
                    matcher.is_match(&r.path)
                } else {
                    true // No include patterns = include all
                };

                // If exclude patterns specified, path must NOT match any
                let excluded = if let Some(ref matcher) = exclude_matcher {
                    matcher.is_match(&r.path)
                } else {
                    false // No exclude patterns = exclude none
                };

                included && !excluded
            });
        }

        // Apply exact name filter (only for symbol searches)
        if filter.exact && filter.symbols_mode {
            results.retain(|r| r.symbol.as_deref() == Some(pattern));
        }

        // Expand symbol bodies if requested
        // Works for both symbol-mode and regex searches (if regex matched a symbol definition)
        if filter.expand {
            // Load content store to fetch full symbol bodies
            let content_path = self.cache.path().join("content.bin");
            if let Ok(content_reader) = ContentReader::open(&content_path) {
                for result in &mut results {
                    // Only expand if the result has a meaningful span (not just a single line)
                    if result.span.start_line < result.span.end_line {
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
        }

        // Step 4: Deduplicate by path if paths-only mode
        if filter.paths_only {
            use std::collections::HashSet;
            let mut seen_paths = HashSet::new();
            results.retain(|r| seen_paths.insert(r.path.clone()));
        }

        // Step 5: Sort results deterministically (by path, then line number)
        results.sort_by(|a, b| {
            a.path.cmp(&b.path)
                .then_with(|| a.span.start_line.cmp(&b.span.start_line))
        });

        // Capture total count AFTER all filtering but BEFORE pagination (offset/limit)
        // This is the total number of results the user can paginate through
        let total_count = results.len();

        // Step 5.5: Apply offset (pagination)
        if let Some(offset) = filter.offset {
            if offset < results.len() {
                results = results.into_iter().skip(offset).collect();
            } else {
                // Offset beyond results - return empty
                results.clear();
            }
        }

        // Step 6: Apply limit
        if let Some(limit) = filter.limit {
            results.truncate(limit);
        }

        log::info!("Query returned {} results (total before pagination: {})", results.len(), total_count);

        Ok((results, total_count))
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

    /// Execute AST query on all indexed files (no trigram filtering)
    ///
    /// WARNING: This method scans the entire codebase (500ms-2s+).
    /// In 95% of cases, use --symbols instead which is 10-100x faster.
    ///
    /// # Algorithm
    /// 1. Get all indexed files for the specified language
    /// 2. Apply glob/exclude filters to reduce file set
    /// 3. Load file contents for all matching files
    /// 4. Execute AST query pattern using Tree-sitter
    /// 5. Apply remaining filters and return results
    ///
    /// # Performance
    /// - Parses entire codebase (not just trigram candidates)
    /// - Expected: 500ms-2s for medium codebases, 2-10s for large codebases
    /// - Use --glob to limit scope for better performance
    ///
    /// # Requirements
    /// - Language must be specified (AST queries are language-specific)
    /// - AST pattern must be valid S-expression syntax
    pub fn search_ast_all_files(&self, ast_pattern: &str, filter: QueryFilter) -> Result<Vec<SearchResult>> {
        log::info!("Executing AST query on all files: pattern='{}', filter={:?}", ast_pattern, filter);

        // Require language for AST queries
        let lang = filter.language.ok_or_else(|| anyhow::anyhow!(
            "Language must be specified for AST pattern matching. Use --lang to specify the language.\n\
             \n\
             Example: rfx query \"(function_definition) @fn\" --ast --lang python"
        ))?;

        // Ensure cache exists
        if !self.cache.exists() {
            anyhow::bail!(
                "Index not found. Run 'rfx index' to build the cache first."
            );
        }

        // Show non-blocking warnings about branch state and staleness
        self.check_index_freshness()?;

        // Load content store
        let content_path = self.cache.path().join("content.bin");
        let content_reader = ContentReader::open(&content_path)
            .context("Failed to open content store")?;

        // Build glob matchers ONCE before file iteration (performance optimization)
        use globset::{Glob, GlobSetBuilder};

        let include_matcher = if !filter.glob_patterns.is_empty() {
            let mut builder = GlobSetBuilder::new();
            for pattern in &filter.glob_patterns {
                // Normalize pattern to ensure LLM-generated patterns work correctly
                let normalized = Self::normalize_glob_pattern(pattern);
                if let Ok(glob) = Glob::new(&normalized) {
                    builder.add(glob);
                }
            }
            builder.build().ok()
        } else {
            None
        };

        let exclude_matcher = if !filter.exclude_patterns.is_empty() {
            let mut builder = GlobSetBuilder::new();
            for pattern in &filter.exclude_patterns {
                // Normalize pattern to ensure LLM-generated patterns work correctly
                let normalized = Self::normalize_glob_pattern(pattern);
                if let Ok(glob) = Glob::new(&normalized) {
                    builder.add(glob);
                }
            }
            builder.build().ok()
        } else {
            None
        };

        // Get all files matching the language and glob filters
        let mut candidates: Vec<SearchResult> = Vec::new();

        for file_id in 0..content_reader.file_count() {
            let file_path = match content_reader.get_file_path(file_id as u32) {
                Some(p) => p,
                None => continue,
            };

            // Detect language from file extension
            let ext = file_path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            let detected_lang = Language::from_extension(ext);

            // Filter by language
            if detected_lang != lang {
                continue;
            }

            let file_path_str = file_path.to_string_lossy().to_string();

            // Apply glob/exclude filters BEFORE loading content (performance optimization)
            let included = include_matcher.as_ref().map_or(true, |m| m.is_match(&file_path_str));
            let excluded = exclude_matcher.as_ref().map_or(false, |m| m.is_match(&file_path_str));

            if !included || excluded {
                continue;
            }

            // Create a dummy candidate for this file (AST query will replace it)
            candidates.push(SearchResult {
                path: file_path_str,
                lang: detected_lang,
                span: Span { start_line: 1, end_line: 1 },
                symbol: None,
                kind: SymbolKind::Unknown("ast_query".to_string()),
                preview: String::new(),
            });
        }

        log::info!("AST query scanning {} files for language {:?}", candidates.len(), lang);

        if candidates.is_empty() {
            log::warn!("No files found for language {:?}. Check your language filter or glob patterns.", lang);
            return Ok(Vec::new());
        }

        // Execute the AST query on all candidate files
        // This will load file contents and parse them with tree-sitter
        let mut results = self.enrich_with_ast(candidates, ast_pattern, filter.language)?;

        log::debug!("AST query found {} matches before filtering", results.len());

        // Apply remaining filters (same as search_internal Phase 3)

        // Apply kind filter
        if let Some(ref kind) = filter.kind {
            results.retain(|r| {
                if matches!(kind, SymbolKind::Function) {
                    matches!(r.kind, SymbolKind::Function | SymbolKind::Method)
                } else {
                    r.kind == *kind
                }
            });
        }

        // Note: exact filter doesn't make sense for AST queries (pattern is S-expression, not symbol name)

        // Expand symbol bodies if requested
        if filter.expand {
            let content_path = self.cache.path().join("content.bin");
            if let Ok(content_reader) = ContentReader::open(&content_path) {
                for result in &mut results {
                    if result.span.start_line < result.span.end_line {
                        if let Some(file_id) = Self::find_file_id(&content_reader, &result.path) {
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
        }

        // Deduplicate by path if paths-only mode
        if filter.paths_only {
            use std::collections::HashSet;
            let mut seen_paths = HashSet::new();
            results.retain(|r| seen_paths.insert(r.path.clone()));
        }

        // Sort results deterministically
        results.sort_by(|a, b| {
            a.path.cmp(&b.path)
                .then_with(|| a.span.start_line.cmp(&b.span.start_line))
        });

        // Apply offset (pagination)
        if let Some(offset) = filter.offset {
            if offset < results.len() {
                results = results.into_iter().skip(offset).collect();
            } else {
                results.clear();
            }
        }

        // Apply limit
        if let Some(limit) = filter.limit {
            results.truncate(limit);
        }

        log::info!("AST query returned {} results", results.len());

        Ok(results)
    }

    /// Search using AST pattern with separate text pattern for trigram filtering
    ///
    /// This allows efficient AST queries by:
    /// 1. Using text_pattern for Phase 1 trigram filtering (narrows to candidate files)
    /// 2. Using ast_pattern for Phase 2 AST matching (structure-aware filtering)
    ///
    /// # Example
    /// ```ignore
    /// // Find async functions: trigram search for "fn ", AST match for function_item
    /// engine.search_ast_with_text_filter("fn ", "(function_item (async))", filter)?;
    /// ```
    pub fn search_ast_with_text_filter(
        &self,
        text_pattern: &str,
        ast_pattern: &str,
        filter: QueryFilter,
    ) -> Result<Vec<SearchResult>> {
        log::info!("Executing AST query with text filter: text='{}', ast='{}', filter={:?}",
                   text_pattern, ast_pattern, filter);

        // Ensure cache exists
        if !self.cache.exists() {
            anyhow::bail!(
                "Index not found. Run 'rfx index' to build the cache first."
            );
        }

        // Show non-blocking warnings about branch state and staleness
        self.check_index_freshness()?;

        // Start timeout timer if configured
        use std::time::{Duration, Instant};
        let start_time = Instant::now();
        let timeout = if filter.timeout_secs > 0 {
            Some(Duration::from_secs(filter.timeout_secs))
        } else {
            None
        };

        // PHASE 1: Get initial candidates using text pattern (trigram search)
        let candidates = if filter.use_regex {
            self.get_regex_candidates(text_pattern, timeout.as_ref(), &start_time)?
        } else {
            self.get_trigram_candidates(text_pattern, &filter)?
        };

        log::debug!("Phase 1 found {} candidate locations", candidates.len());

        // PHASE 2: Execute AST query on candidates
        let mut results = self.enrich_with_ast(candidates, ast_pattern, filter.language)?;

        log::debug!("Phase 2 AST matching found {} results", results.len());

        // PHASE 3: Apply filters
        if let Some(lang) = filter.language {
            results.retain(|r| r.lang == lang);
        }

        if let Some(ref kind) = filter.kind {
            results.retain(|r| {
                if matches!(kind, SymbolKind::Function) {
                    matches!(r.kind, SymbolKind::Function | SymbolKind::Method)
                } else {
                    r.kind == *kind
                }
            });
        }

        if let Some(ref file_pattern) = filter.file_pattern {
            results.retain(|r| r.path.contains(file_pattern));
        }

        // Apply glob pattern filters (same logic as in search_internal)
        if !filter.glob_patterns.is_empty() || !filter.exclude_patterns.is_empty() {
            use globset::{Glob, GlobSetBuilder};

            let include_matcher = if !filter.glob_patterns.is_empty() {
                let mut builder = GlobSetBuilder::new();
                for pattern in &filter.glob_patterns {
                    // Normalize pattern to ensure LLM-generated patterns work correctly
                    let normalized = Self::normalize_glob_pattern(pattern);
                    if let Ok(glob) = Glob::new(&normalized) {
                        builder.add(glob);
                    }
                }
                builder.build().ok()
            } else {
                None
            };

            let exclude_matcher = if !filter.exclude_patterns.is_empty() {
                let mut builder = GlobSetBuilder::new();
                for pattern in &filter.exclude_patterns {
                    // Normalize pattern to ensure LLM-generated patterns work correctly
                    let normalized = Self::normalize_glob_pattern(pattern);
                    if let Ok(glob) = Glob::new(&normalized) {
                        builder.add(glob);
                    }
                }
                builder.build().ok()
            } else {
                None
            };

            results.retain(|r| {
                let included = include_matcher.as_ref().map_or(true, |m| m.is_match(&r.path));
                let excluded = exclude_matcher.as_ref().map_or(false, |m| m.is_match(&r.path));
                included && !excluded
            });
        }

        if filter.exact && filter.symbols_mode {
            results.retain(|r| r.symbol.as_deref() == Some(text_pattern));
        }

        // Expand symbol bodies if requested
        if filter.expand {
            let content_path = self.cache.path().join("content.bin");
            if let Ok(content_reader) = ContentReader::open(&content_path) {
                for result in &mut results {
                    if result.span.start_line < result.span.end_line {
                        if let Some(file_id) = Self::find_file_id(&content_reader, &result.path) {
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
        }

        // Sort results deterministically
        results.sort_by(|a, b| {
            a.path.cmp(&b.path)
                .then_with(|| a.span.start_line.cmp(&b.span.start_line))
        });

        // Apply offset (pagination)
        if let Some(offset) = filter.offset {
            if offset < results.len() {
                results = results.into_iter().skip(offset).collect();
            } else {
                results.clear();
            }
        }

        // Apply limit
        if let Some(limit) = filter.limit {
            results.truncate(limit);
        }

        log::info!("AST query returned {} results", results.len());

        Ok(results)
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

    /// Enrich text match candidates with symbol information by parsing files
    ///
    /// Takes a list of text match candidates and replaces them with actual symbol
    /// definitions where the symbol name matches the pattern.
    ///
    /// # Algorithm
    /// 1. Group candidates by file_id for efficient processing
    /// 2. Parse each file with tree-sitter to extract symbols
    /// 3. For each symbol, check if its name matches the pattern
    ///    - If use_regex=true: match symbol name against regex pattern
    ///    - If use_contains=true: substring match (contains)
    ///    - Default: exact match
    /// 4. Return symbol results (not the original text matches)
    ///
    /// # Performance
    /// Only parses files that have text matches, so typically 10-100 files
    /// instead of the entire codebase (62K+ files).
    ///
    /// # Optimizations
    /// 1. Language filtering: Skips files with unsupported languages (no parsers)
    /// 2. Parallel processing: Uses Rayon to parse files concurrently across CPU cores
    fn enrich_with_symbols(&self, candidates: Vec<SearchResult>, pattern: &str, filter: &QueryFilter) -> Result<Vec<SearchResult>> {
        // Load content store for file reading
        let content_path = self.cache.path().join("content.bin");
        let content_reader = ContentReader::open(&content_path)
            .context("Failed to open content store")?;

        // Load trigram index for file path lookups
        let trigrams_path = self.cache.path().join("trigrams.bin");
        let trigram_index = if trigrams_path.exists() {
            TrigramIndex::load(&trigrams_path)?
        } else {
            Self::rebuild_trigram_index(&content_reader)?
        };

        // Open symbol cache for reading cached symbols
        let symbol_cache = crate::symbol_cache::SymbolCache::open(self.cache.path())
            .context("Failed to open symbol cache")?;

        // Load file hashes for current branch for cache lookups
        let root = self.cache.workspace_root();
        let branch = crate::git::get_current_branch(&root)
            .unwrap_or_else(|_| "_default".to_string());
        let file_hashes = self.cache.load_hashes_for_branch(&branch)
            .context("Failed to load file hashes")?;
        log::debug!("Loaded {} file hashes for branch '{}' for symbol cache lookups", file_hashes.len(), branch);

        // Group candidates by file, filtering out unsupported languages
        use std::collections::HashMap;
        let mut files_by_path: HashMap<String, Vec<SearchResult>> = HashMap::new();
        let mut skipped_unsupported = 0;

        for candidate in candidates {
            // Skip files with unsupported languages (no parser available)
            if !candidate.lang.is_supported() {
                skipped_unsupported += 1;
                continue;
            }

            files_by_path
                .entry(candidate.path.clone())
                .or_insert_with(Vec::new)
                .push(candidate);
        }

        let total_files = files_by_path.len();
        log::debug!("Processing {} candidate files for symbol enrichment (skipped {} unsupported language files)",
                   total_files, skipped_unsupported);

        // Warn if pattern is very broad (may take time to parse all files)
        if total_files > 1000 {
            log::warn!(
                "Pattern '{}' matched {} files. This may take some time to parse.",
                pattern,
                total_files
            );
            log::warn!("Consider using a more specific pattern or adding --lang/--file filters to narrow the search.");
        }

        // Convert to vec for parallel processing
        let mut files_to_process: Vec<String> = files_by_path.keys().cloned().collect();

        // PHASE 2a: Line-based pre-filtering (skip files where ALL matches are in comments/strings)
        // This reduces tree-sitter parsing workload by 2-5x for most queries
        let mut files_to_skip: std::collections::HashSet<String> = std::collections::HashSet::new();

        for file_path in &files_to_process {
            // Get the language for this file
            let ext = std::path::Path::new(file_path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            let lang = Language::from_extension(ext);

            // Get line filter for this language (if available)
            if let Some(line_filter) = crate::line_filter::get_filter(lang) {
                // Find file_id for this path
                let file_id = match Self::find_file_id_by_path(&content_reader, &trigram_index, file_path) {
                    Some(id) => id,
                    None => continue,
                };

                // Load file content
                let content = match content_reader.get_file_content(file_id) {
                    Ok(c) => c,
                    Err(_) => continue,
                };

                // Check if ALL pattern occurrences are in comments/strings
                let mut all_in_non_code = true;
                for line in content.lines() {
                    // Find all occurrences of the pattern in this line
                    let mut search_start = 0;
                    while let Some(pos) = line[search_start..].find(pattern) {
                        let absolute_pos = search_start + pos;

                        // Check if this occurrence is in code (not comment/string)
                        let in_comment = line_filter.is_in_comment(line, absolute_pos);
                        let in_string = line_filter.is_in_string(line, absolute_pos);

                        if !in_comment && !in_string {
                            // Found at least one occurrence in actual code
                            all_in_non_code = false;
                            break;
                        }

                        search_start = absolute_pos + pattern.len();
                    }

                    if !all_in_non_code {
                        break;
                    }
                }

                // If ALL occurrences are in comments/strings, skip this file
                if all_in_non_code {
                    // Double-check: make sure there was at least one occurrence
                    if content.contains(pattern) {
                        files_to_skip.insert(file_path.clone());
                        log::debug!("Pre-filter: Skipping {} (all matches in comments/strings)", file_path);
                    }
                }
            }
        }

        // Filter out files we're skipping
        files_to_process.retain(|path| !files_to_skip.contains(path));

        log::debug!("Pre-filter: Skipped {} files where all matches are in comments/strings (parsing {} files)",
                   files_to_skip.len(), files_to_process.len());

        // Configure thread pool for parallel processing (use 80% of available cores, capped at 8)
        let num_threads = {
            let available_cores = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4);
            // Use 80% of available cores (minimum 1, maximum 8) to avoid locking the system
            // Cap at 8 to prevent diminishing returns from cache contention on high-core systems
            ((available_cores as f64 * 0.8).ceil() as usize).max(1).min(8)
        };

        log::debug!("Using {} threads for parallel symbol extraction (out of {} available cores)",
                   num_threads,
                   std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4));

        // Build a custom thread pool with limited threads
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()
            .context("Failed to create thread pool for symbol extraction")?;

        // OPTIMIZATION: Batch read all cached symbols in ONE database transaction
        // This is 10-30x faster than calling get() individually for each file

        // Step 1: Collect file paths that have hashes
        let files_with_hashes: Vec<String> = files_to_process
            .iter()
            .filter(|path| file_hashes.contains_key(path.as_str()))
            .cloned()
            .collect();

        // Step 2: Batch lookup file_ids for all paths
        let file_id_map = self.cache.batch_get_file_ids(&files_with_hashes)
            .context("Failed to batch lookup file IDs")?;

        // Step 3: Build (file_id, hash, path) tuples for batch_get_with_kind
        let file_lookup_tuples: Vec<(i64, String, String)> = files_with_hashes
            .iter()
            .filter_map(|path| {
                let file_id = file_id_map.get(path)?;
                let hash = file_hashes.get(path.as_str())?;
                Some((*file_id, hash.clone(), path.clone()))
            })
            .collect();

        // Step 4: Batch read symbols with kind filtering (uses junction table + integer joins)
        let batch_results = symbol_cache.batch_get_with_kind(&file_lookup_tuples, filter.kind.clone())
            .context("Failed to batch read symbol cache")?;

        // Step 5: Separate files into cached vs need-to-parse
        let mut cached_symbols: HashMap<String, Vec<SearchResult>> = HashMap::new();
        let mut files_needing_parse: Vec<String> = Vec::new();

        // Build path lookup from file_id
        let id_to_path: HashMap<i64, String> = file_id_map
            .iter()
            .map(|(path, id)| (*id, path.clone()))
            .collect();

        // Process cached results
        for (file_id, symbols) in batch_results {
            if let Some(file_path) = id_to_path.get(&file_id) {
                cached_symbols.insert(file_path.clone(), symbols);
            }
        }

        // Files with hashes but not in cache results need parsing
        for path in &files_with_hashes {
            if file_id_map.contains_key(path) && !cached_symbols.contains_key(path) {
                files_needing_parse.push(path.clone());
            }
        }

        // Add files without hashes to parse list
        for file_path in &files_to_process {
            if !file_hashes.contains_key(file_path.as_str()) {
                files_needing_parse.push(file_path.clone());
            }
        }

        log::debug!(
            "Symbol cache: {} hits, {} need parsing",
            cached_symbols.len(),
            files_needing_parse.len()
        );

        // Parse files in parallel using custom thread pool (only cache misses)
        use rayon::prelude::*;

        let parsed_symbols: Vec<SearchResult> = pool.install(|| {
            files_needing_parse
                .par_iter()
                .flat_map(|file_path| {
                // Find file_id for this path
                let file_id = match Self::find_file_id_by_path(&content_reader, &trigram_index, file_path) {
                    Some(id) => id,
                    None => {
                        log::warn!("Could not find file_id for path: {}", file_path);
                        return Vec::new();
                    }
                };

                let content = match content_reader.get_file_content(file_id) {
                    Ok(c) => c,
                    Err(e) => {
                        log::warn!("Failed to read file {}: {}", file_path, e);
                        return Vec::new();
                    }
                };

                // Detect language
                let ext = std::path::Path::new(file_path)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");
                let lang = Language::from_extension(ext);

                // Parse file to extract symbols
                let symbols = match ParserFactory::parse(file_path, content, lang) {
                    Ok(symbols) => {
                        log::debug!("Parsed {} symbols from {}", symbols.len(), file_path);
                        symbols
                    }
                    Err(e) => {
                        log::debug!("Failed to parse {}: {}", file_path, e);
                        Vec::new()
                    }
                };

                // Cache the parsed symbols (ignore errors - caching is best-effort)
                if let Some(file_hash) = file_hashes.get(file_path.as_str()) {
                    if let Err(e) = symbol_cache.set(file_path, file_hash, &symbols) {
                        log::debug!("Failed to cache symbols for {}: {}", file_path, e);
                    }
                }

                symbols
            })
            .collect()
        });

        // Combine cached and parsed symbols
        let mut all_symbols: Vec<SearchResult> = Vec::new();

        // Add all cached symbols
        for symbols in cached_symbols.values() {
            all_symbols.extend_from_slice(symbols);
        }

        // Add all parsed symbols
        all_symbols.extend(parsed_symbols);

        // KEYWORD DETECTION: Check if pattern is a language keyword (e.g., "class", "function")
        // If it matches a keyword AND symbols_mode is true, interpret as "list all symbols of that type"
        // rather than looking for a symbol literally named "class" or "function"
        //
        // IMPORTANT: Only check keywords for languages that will pass Phase 3 filtering.
        // If a language filter is specified, only check that language's keywords.
        // Otherwise, check all languages present in the symbol results.
        let is_keyword_query = {
            // Determine which language to check keywords for
            let lang_to_check = if let Some(lang) = filter.language {
                // Language filter specified - check that language only
                // This ensures keyword detection aligns with Phase 3 language filtering
                vec![lang]
            } else {
                // No language filter - check all languages that appear in the actual symbols
                // (not candidates, but the parsed symbols that made it through)
                // This handles mixed-language codebases correctly
                let mut langs: Vec<Language> = all_symbols.iter()
                    .map(|s| s.lang)
                    .collect::<Vec<_>>();
                langs.sort_by(|a, b| format!("{:?}", a).cmp(&format!("{:?}", b))); // Deterministic ordering
                langs.dedup(); // Remove duplicates after sorting
                langs
            };

            // Check if pattern matches a keyword in any of the relevant languages
            lang_to_check.iter().any(|lang| {
                ParserFactory::get_keywords(*lang).contains(&pattern)
            })
        };

        // If pattern is a keyword (like "class" or "function"), skip name-based filtering
        // and return all symbols (kind filtering happens in Phase 3)
        let filtered: Vec<SearchResult> = if is_keyword_query {
            log::info!("Pattern '{}' is a language keyword - listing all symbols (kind filtering will be applied in Phase 3)", pattern);
            all_symbols
        } else if filter.use_regex {
            // Compile regex for symbol name matching
            let regex = Regex::new(pattern)
                .with_context(|| format!("Invalid regex pattern: {}", pattern))?;

            all_symbols
                .into_iter()
                .filter(|sym| {
                    sym.symbol.as_deref().map_or(false, |s| regex.is_match(s))
                })
                .collect()
        } else if filter.use_contains {
            // Substring match (opt-in with --contains)
            all_symbols
                .into_iter()
                .filter(|sym| sym.symbol.as_deref().map_or(false, |s| s.contains(pattern)))
                .collect()
        } else {
            // Exact match (default)
            all_symbols
                .into_iter()
                .filter(|sym| sym.symbol.as_deref().map_or(false, |s| s == pattern))
                .collect()
        };

        log::info!("Symbol enrichment found {} matches for pattern '{}'", filtered.len(), pattern);

        Ok(filtered)
    }

    /// Enrich text match candidates with AST pattern matching
    ///
    /// Takes a list of text match candidates and executes a Tree-sitter AST query
    /// on the candidate files, returning only matches that satisfy the AST pattern.
    ///
    /// # Algorithm
    /// 1. Extract unique file paths from candidates
    /// 2. Load file contents for each candidate file
    /// 3. Execute AST query pattern using Tree-sitter
    /// 4. Return AST matches
    ///
    /// # Performance
    /// Only parses files that have text matches, so typically 10-100 files
    /// instead of the entire codebase (62K+ files).
    ///
    /// # Requirements
    /// - Language must be specified (AST queries are language-specific)
    /// - AST pattern must be valid S-expression syntax
    fn enrich_with_ast(&self, candidates: Vec<SearchResult>, ast_pattern: &str, language: Option<Language>) -> Result<Vec<SearchResult>> {
        // Require language for AST queries
        let lang = language.ok_or_else(|| anyhow::anyhow!(
            "Language must be specified for AST pattern matching. Use --lang to specify the language."
        ))?;

        // Load content store for file reading
        let content_path = self.cache.path().join("content.bin");
        let content_reader = ContentReader::open(&content_path)
            .context("Failed to open content store")?;

        // Load trigram index for file path lookups
        let trigrams_path = self.cache.path().join("trigrams.bin");
        let trigram_index = if trigrams_path.exists() {
            TrigramIndex::load(&trigrams_path)?
        } else {
            Self::rebuild_trigram_index(&content_reader)?
        };

        // Collect unique file paths from candidates and load their contents
        use std::collections::HashMap;
        let mut file_contents: HashMap<String, String> = HashMap::new();

        for candidate in &candidates {
            if file_contents.contains_key(&candidate.path) {
                continue;
            }

            // Find file_id for this path
            let file_id = match Self::find_file_id_by_path(&content_reader, &trigram_index, &candidate.path) {
                Some(id) => id,
                None => {
                    log::warn!("Could not find file_id for path: {}", candidate.path);
                    continue;
                }
            };

            // Load file content
            let content = match content_reader.get_file_content(file_id) {
                Ok(c) => c,
                Err(e) => {
                    log::warn!("Failed to read file {}: {}", candidate.path, e);
                    continue;
                }
            };

            file_contents.insert(candidate.path.clone(), content.to_string());
        }

        log::debug!("Executing AST query on {} candidate files with language {:?}", file_contents.len(), lang);

        // Execute AST query using the ast_query module
        let results = crate::ast_query::execute_ast_query(candidates, ast_pattern, lang, &file_contents)?;

        log::info!("AST query found {} matches for pattern '{}'", results.len(), ast_pattern);

        Ok(results)
    }

    /// Helper to find file_id by path string
    fn find_file_id_by_path(
        content_reader: &ContentReader,
        trigram_index: &TrigramIndex,
        target_path: &str,
    ) -> Option<u32> {
        // Try trigram index first (faster)
        for file_id in 0..trigram_index.file_count() {
            if let Some(path) = trigram_index.get_file(file_id as u32) {
                if path.to_string_lossy() == target_path {
                    return Some(file_id as u32);
                }
            }
        }

        // Fallback to content reader
        for file_id in 0..content_reader.file_count() {
            if let Some(path) = content_reader.get_file_path(file_id as u32) {
                if path.to_string_lossy() == target_path {
                    return Some(file_id as u32);
                }
            }
        }

        None
    }

    /// Map keyword patterns to SymbolKind for auto-inference
    ///
    /// When users search for keywords like "class" or "function" with --symbols,
    /// automatically infer the kind filter to return only symbols of that type.
    ///
    /// This makes keyword queries more intuitive: searching for "class" returns
    /// only classes, not all symbols.
    fn keyword_to_kind(keyword: &str) -> Option<SymbolKind> {
        match keyword {
            // Classes and types
            "class" => Some(SymbolKind::Class),
            "struct" => Some(SymbolKind::Struct),
            "enum" => Some(SymbolKind::Enum),
            "interface" => Some(SymbolKind::Interface),
            "trait" => Some(SymbolKind::Trait),
            "type" => Some(SymbolKind::Type),
            "record" => Some(SymbolKind::Struct),  // C# record types

            // Functions and methods
            "function" | "fn" | "def" | "func" => Some(SymbolKind::Function),

            // Variables and constants
            "const" | "static" => Some(SymbolKind::Constant),
            "var" | "let" => Some(SymbolKind::Variable),

            // Modules and namespaces
            "mod" | "module" | "namespace" => Some(SymbolKind::Module),

            // Other constructs
            "impl" => None,  // impl blocks don't have a direct SymbolKind
            "async" => None, // async is a modifier, not a symbol type

            // Default: no mapping (return all symbols)
            _ => None,
        }
    }

    /// Get all files matching the language filter (for keyword queries)
    ///
    /// This method bypasses trigram search and returns ALL files of the specified language.
    /// Used for keyword queries like "list all classes" where we need complete coverage,
    /// not just the first 100 candidates from a trigram search.
    ///
    /// Similar to `search_ast_all_files()` but works for symbol queries instead of AST queries.
    fn get_all_language_files(&self, filter: &QueryFilter) -> Result<Vec<SearchResult>> {
        // Language filter is optional - if not specified, scan all files
        // If specified, only scan files of that language

        // Load content store
        let content_path = self.cache.path().join("content.bin");
        let content_reader = ContentReader::open(&content_path)
            .context("Failed to open content store")?;

        // Build glob matchers if specified (for filtering)
        use globset::{Glob, GlobSetBuilder};

        let include_matcher = if !filter.glob_patterns.is_empty() {
            let mut builder = GlobSetBuilder::new();
            for pattern in &filter.glob_patterns {
                let normalized = Self::normalize_glob_pattern(pattern);
                if let Ok(glob) = Glob::new(&normalized) {
                    builder.add(glob);
                }
            }
            builder.build().ok()
        } else {
            None
        };

        let exclude_matcher = if !filter.exclude_patterns.is_empty() {
            let mut builder = GlobSetBuilder::new();
            for pattern in &filter.exclude_patterns {
                let normalized = Self::normalize_glob_pattern(pattern);
                if let Ok(glob) = Glob::new(&normalized) {
                    builder.add(glob);
                }
            }
            builder.build().ok()
        } else {
            None
        };

        // Scan all files and filter by language + glob patterns
        let mut candidates: Vec<SearchResult> = Vec::new();

        for file_id in 0..content_reader.file_count() {
            let file_path = match content_reader.get_file_path(file_id as u32) {
                Some(p) => p,
                None => continue,
            };

            // Detect language from file extension
            let ext = file_path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            let detected_lang = Language::from_extension(ext);

            // Filter by language (if specified)
            if let Some(lang) = filter.language {
                if detected_lang != lang {
                    continue;
                }
            }

            let file_path_str = file_path.to_string_lossy().to_string();

            // Apply glob/exclude filters
            let included = include_matcher.as_ref().map_or(true, |m| m.is_match(&file_path_str));
            let excluded = exclude_matcher.as_ref().map_or(false, |m| m.is_match(&file_path_str));

            if !included || excluded {
                continue;
            }

            // Apply file path filter if specified
            if let Some(ref file_pattern) = filter.file_pattern {
                if !file_path_str.contains(file_pattern) {
                    continue;
                }
            }

            // Create a dummy candidate for this file
            // Phase 2 (symbol enrichment) will parse it and extract actual symbols
            candidates.push(SearchResult {
                path: file_path_str,
                lang: detected_lang,
                span: Span { start_line: 1, end_line: 1 },
                symbol: None,
                kind: SymbolKind::Unknown("keyword_query".to_string()),
                preview: String::new(),
            });
        }

        if let Some(lang) = filter.language {
            log::info!("Keyword query will scan {} {:?} files for symbol extraction", candidates.len(), lang);
        } else {
            log::info!("Keyword query will scan {} files (all languages) for symbol extraction", candidates.len());
        }

        Ok(candidates)
    }

    /// Get candidate results using trigram-based full-text search
    fn get_trigram_candidates(&self, pattern: &str, filter: &QueryFilter) -> Result<Vec<SearchResult>> {
        // Load content store
        let content_path = self.cache.path().join("content.bin");
        let content_reader = ContentReader::open(&content_path)
            .context("Failed to open content store")?;

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
        log::debug!("Found {} candidate locations from trigram search", candidates.len());

        // Clone pattern to owned String for thread safety
        let pattern_owned = pattern.to_string();

        // Group candidates by file for efficient processing
        use std::collections::HashMap;
        let mut candidates_by_file: HashMap<u32, Vec<crate::trigram::FileLocation>> = HashMap::new();
        for loc in candidates {
            candidates_by_file
                .entry(loc.file_id)
                .or_insert_with(Vec::new)
                .push(loc);
        }

        log::debug!("Scanning {} files with trigram matches", candidates_by_file.len());

        // Process files in parallel using rayon
        use rayon::prelude::*;

        let results: Vec<SearchResult> = candidates_by_file
            .par_iter()
            .flat_map(|(file_id, locations)| {
                // Get file metadata
                let file_path = match trigram_index.get_file(*file_id) {
                    Some(p) => p,
                    None => return Vec::new(),
                };

                let content = match content_reader.get_file_content(*file_id) {
                    Ok(c) => c,
                    Err(_) => return Vec::new(),
                };

                let file_path_str = file_path.to_string_lossy().to_string();

                // Detect language once per file
                let ext = file_path.extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");
                let lang = Language::from_extension(ext);

                // Split content into lines once
                let lines: Vec<&str> = content.lines().collect();

                // Use a HashSet to deduplicate results by line number
                let mut seen_lines: std::collections::HashSet<usize> = std::collections::HashSet::new();
                let mut file_results = Vec::new();

                // Only check the specific lines indicated by trigram posting lists
                for loc in locations {
                    let line_no = loc.line_no as usize;

                    // Skip if we've already processed this line
                    if seen_lines.contains(&line_no) {
                        continue;
                    }

                    // Bounds check
                    if line_no == 0 || line_no > lines.len() {
                        log::debug!("Line {} out of bounds (file has {} lines)", line_no, lines.len());
                        continue;
                    }

                    let line = lines[line_no - 1];

                    // Apply word-boundary or substring matching based on filter
                    // - Default (not contains, not regex): Word-boundary matching (restrictive)
                    // - --contains or --regex: Substring matching (expansive)
                    let line_matches = if filter.use_contains || filter.use_regex {
                        // Substring matching (expansive)
                        line.contains(&pattern_owned)
                    } else {
                        // Word-boundary matching (restrictive, default)
                        Self::has_word_boundary_match(line, &pattern_owned)
                    };

                    if !line_matches {
                        continue;
                    }

                    seen_lines.insert(line_no);

                    // Create a text match result (no symbol lookup for performance)
                    file_results.push(SearchResult {
                        path: file_path_str.clone(),
                        lang: lang.clone(),
                        kind: SymbolKind::Unknown("text_match".to_string()),
                        symbol: None,  // No symbol name for text matches (avoid duplication)
                        span: Span {
                            start_line: line_no,
                            end_line: line_no,
                        },
                        preview: line.to_string(),
                    });
                }

                file_results
            })
            .collect();

        Ok(results)
    }

    /// Get candidate results using regex patterns with trigram optimization
    ///
    /// # Algorithm
    ///
    /// 1. Extract literal sequences from the regex pattern (≥3 chars)
    /// 2. If literals found: search for files containing ANY of the literals (UNION)
    /// 3. If no literals: fall back to full content scan
    /// 4. Compile regex and verify matches in candidate files
    /// 5. Return matching results with context
    ///
    /// # File Selection Strategy
    ///
    /// Uses UNION of files containing any literal (conservative approach):
    /// - For alternation patterns `(a|b)`: Correctly searches files with a OR b
    /// - For sequential patterns `a.*b`: Searches files with a OR b (may include extra files)
    /// - Trade-off: Ensures correctness at the cost of scanning 2-3x more files for sequential patterns
    /// - Performance impact is minimal due to memory-mapped I/O (<5ms overhead typically)
    ///
    /// # Performance
    ///
    /// - Best case (pattern with literals): <20ms (trigram optimization)
    /// - Typical case (alternation/sequential): 5-15ms on small codebases (<100 files)
    /// - Worst case (no literals like `.*`): ~100ms (full scan)
    fn get_regex_candidates(&self, pattern: &str, timeout: Option<&std::time::Duration>, start_time: &std::time::Instant) -> Result<Vec<SearchResult>> {
        // Step 1: Compile the regex
        let regex = Regex::new(pattern)
            .with_context(|| format!("Invalid regex pattern: {}", pattern))?;

        // Check timeout before expensive operations
        if let Some(timeout_duration) = timeout {
            if start_time.elapsed() > *timeout_duration {
                anyhow::bail!(
                    "Query timeout exceeded ({} seconds) during regex compilation",
                    timeout_duration.as_secs()
                );
            }
        }

        // Step 2: Extract trigrams from regex
        let trigrams = extract_trigrams_from_regex(pattern);

        // Load content store
        let content_path = self.cache.path().join("content.bin");
        let content_reader = ContentReader::open(&content_path)
            .context("Failed to open content store")?;

        let mut results = Vec::new();

        if trigrams.is_empty() {
            // No trigrams - fall back to full scan
            log::warn!("Regex pattern '{}' has no literals (≥3 chars), falling back to full content scan", pattern);
            log::warn!("This may be slow on large codebases. Consider using patterns with literal text.");

            // Scan all files
            for file_id in 0..content_reader.file_count() {
                let file_path = content_reader.get_file_path(file_id as u32)
                    .context("Invalid file_id")?;
                let content = content_reader.get_file_content(file_id as u32)?;

                self.find_regex_matches_in_file(
                    &regex,
                    file_path,
                    content,
                    &mut results,
                )?;
            }
        } else {
            // Use trigrams to narrow down candidates
            log::debug!("Using {} trigrams to narrow regex search candidates", trigrams.len());

            // Load trigram index
            let trigrams_path = self.cache.path().join("trigrams.bin");
            let trigram_index = if trigrams_path.exists() {
                TrigramIndex::load(&trigrams_path)?
            } else {
                Self::rebuild_trigram_index(&content_reader)?
            };

            // Extract the literal sequences from the regex pattern
            use crate::regex_trigrams::extract_literal_sequences;
            let literals = extract_literal_sequences(pattern);

            if literals.is_empty() {
                log::warn!("Regex extraction found trigrams but no literal sequences - this shouldn't happen");
                // Fall back to full scan
                for file_id in 0..content_reader.file_count() {
                    let file_path = content_reader.get_file_path(file_id as u32)
                        .context("Invalid file_id")?;
                    let content = content_reader.get_file_content(file_id as u32)?;
                    self.find_regex_matches_in_file(&regex, file_path, content, &mut results)?;
                }
            } else {
                // Search for each literal sequence and union the results
                // This ensures we find matches for ANY literal (important for alternation patterns like (a|b))
                // Trade-off: May scan more files than necessary for sequential patterns (a.*b),
                // but ensures correctness for all regex patterns
                use std::collections::HashSet;
                let mut candidate_files: HashSet<u32> = HashSet::new();

                for literal in &literals {
                    // Search for this literal in the trigram index
                    let candidates = trigram_index.search(literal);
                    let file_ids: HashSet<u32> = candidates.iter().map(|loc| loc.file_id).collect();

                    log::debug!("Literal '{}' found in {} files", literal, file_ids.len());

                    // Union with existing candidate files (not intersection)
                    // This ensures we search files containing ANY of the literals
                    candidate_files.extend(file_ids);
                }

                let final_candidates = candidate_files;
                log::debug!("After union: searching {} files that contain any literal", final_candidates.len());

                // Verify regex matches in candidate files only
                for &file_id in &final_candidates {
                    let file_path = trigram_index.get_file(file_id)
                        .context("Invalid file_id from trigram search")?;
                    let content = content_reader.get_file_content(file_id)?;

                    self.find_regex_matches_in_file(
                        &regex,
                        file_path,
                        content,
                        &mut results,
                    )?;
                }
            }
        }

        log::info!("Regex search found {} matches for pattern '{}'", results.len(), pattern);
        Ok(results)
    }

    /// Find all regex matches in a single file
    fn find_regex_matches_in_file(
        &self,
        regex: &Regex,
        file_path: &std::path::Path,
        content: &str,
        results: &mut Vec<SearchResult>,
    ) -> Result<()> {
        let file_path_str = file_path.to_string_lossy().to_string();

        // Detect language from file extension
        let ext = file_path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let lang = Language::from_extension(ext);

        // Find all regex matches line by line
        for (line_idx, line) in content.lines().enumerate() {
            if regex.is_match(line) {
                let line_no = line_idx + 1;

                // Create text match result
                // Note: We don't extract symbol names from regex matches because:
                // 1. Regex might match partial identifiers (e.g., "UserController" in "ListUserController")
                // 2. Regex might match across language-specific delimiters (namespaces, scopes, etc.)
                // 3. Accurate symbol extraction requires tree-sitter parsing (expensive)
                // The user can see the full context in the 'preview' field
                results.push(SearchResult {
                    path: file_path_str.clone(),
                    lang: lang.clone(),
                    kind: SymbolKind::Unknown("regex_match".to_string()),
                    symbol: None,  // No symbol name for regex matches
                    span: Span {
                        start_line: line_no,
                        end_line: line_no,
                    },
                    preview: line.to_string(),
                });
            }
        }

        Ok(())
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

    /// Normalize glob patterns for consistent matching
    ///
    /// Ensures glob patterns work correctly by auto-prepending "./" to relative paths
    /// that don't already start with ".", "/", or "*". This fixes LLM-generated patterns
    /// that omit the explicit relative path prefix.
    ///
    /// # Examples
    /// - "services/**/*.php" → "./services/**/*.php"
    /// - "./services/**/*.php" → "./services/**/*.php" (unchanged)
    /// - "**/services/**/*.php" → "**/services/**/*.php" (unchanged)
    /// - "/absolute/path/**" → "/absolute/path/**" (unchanged)
    fn normalize_glob_pattern(pattern: &str) -> String {
        if pattern.starts_with('.') || pattern.starts_with('/') || pattern.starts_with('*') {
            // Already has a prefix that works - don't modify
            pattern.to_string()
        } else {
            // Relative path without explicit prefix - add "./"
            format!("./{}", pattern)
        }
    }

    /// Check if pattern appears at word boundaries in a line
    ///
    /// Word boundary is defined as:
    /// - Start/end of string
    /// - Transition between word characters (\w) and non-word characters (\W)
    ///
    /// This is used for default (restrictive) matching to find complete identifiers
    /// rather than substrings. For example:
    /// - "Error" matches "Error" but not "NetworkError"
    /// - "parse" matches "parse()" but not "parseUser()"
    fn has_word_boundary_match(line: &str, pattern: &str) -> bool {
        // Build regex: \bpattern\b
        let escaped_pattern = regex::escape(pattern);
        let pattern_with_boundaries = format!(r"\b{}\b", escaped_pattern);

        if let Ok(re) = Regex::new(&pattern_with_boundaries) {
            re.is_match(line)
        } else {
            // If regex fails (shouldn't happen with escaped pattern), fall back to substring
            log::debug!("Word boundary regex failed for pattern '{}', falling back to substring", pattern);
            line.contains(pattern)
        }
    }

    /// Get index status for programmatic use (doesn't print warnings)
    ///
    /// Returns (status, can_trust_results, warning) tuple for JSON output.
    /// This is optimized for AI agents to detect staleness and auto-reindex.
    fn get_index_status(&self) -> Result<(IndexStatus, bool, Option<IndexWarning>)> {
        let root = std::env::current_dir()?;

        // Check git state if in a git repo
        if crate::git::is_git_repo(&root) {
            if let Ok(current_branch) = crate::git::get_current_branch(&root) {
                // Check if we're on a different branch than what was indexed
                if !self.cache.branch_exists(&current_branch).unwrap_or(false) {
                    let warning = IndexWarning {
                        reason: format!("Branch '{}' has not been indexed", current_branch),
                        action_required: "rfx index".to_string(),
                        details: Some(IndexWarningDetails {
                            current_branch: Some(current_branch),
                            indexed_branch: None,
                            current_commit: None,
                            indexed_commit: None,
                        }),
                    };
                    return Ok((IndexStatus::Stale, false, Some(warning)));
                }

                // Branch exists - check if commit changed
                if let (Ok(current_commit), Ok(branch_info)) =
                    (crate::git::get_current_commit(&root), self.cache.get_branch_info(&current_branch)) {

                    if branch_info.commit_sha != current_commit {
                        let warning = IndexWarning {
                            reason: format!(
                                "Commit changed from {} to {}",
                                &branch_info.commit_sha[..7],
                                &current_commit[..7]
                            ),
                            action_required: "rfx index".to_string(),
                            details: Some(IndexWarningDetails {
                                current_branch: Some(current_branch.clone()),
                                indexed_branch: Some(current_branch.clone()),
                                current_commit: Some(current_commit.clone()),
                                indexed_commit: Some(branch_info.commit_sha.clone()),
                            }),
                        };
                        return Ok((IndexStatus::Stale, false, Some(warning)));
                    }

                    // If commits match, do a quick file freshness check
                    if let Ok(branch_files) = self.cache.get_branch_files(&current_branch) {
                        let mut checked = 0;
                        let mut changed = 0;
                        const SAMPLE_SIZE: usize = 10;

                        for (path, _indexed_hash) in branch_files.iter().take(SAMPLE_SIZE) {
                            checked += 1;
                            let file_path = std::path::Path::new(path);

                            if let Ok(metadata) = std::fs::metadata(file_path) {
                                if let Ok(modified) = metadata.modified() {
                                    let indexed_time = branch_info.last_indexed;
                                    let file_time = modified.duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_secs() as i64;

                                    if file_time > indexed_time {
                                        // File modified after indexing - likely stale
                                        // Note: We skip hash verification for performance (mtime check is sufficient)
                                        changed += 1;
                                    }
                                }
                            }
                        }

                        if changed > 0 {
                            let warning = IndexWarning {
                                reason: format!("{} of {} sampled files modified", changed, checked),
                                action_required: "rfx index".to_string(),
                                details: Some(IndexWarningDetails {
                                    current_branch: Some(current_branch.clone()),
                                    indexed_branch: Some(branch_info.branch.clone()),
                                    current_commit: Some(current_commit.clone()),
                                    indexed_commit: Some(branch_info.commit_sha.clone()),
                                }),
                            };
                            return Ok((IndexStatus::Stale, false, Some(warning)));
                        }
                    }

                    // All checks passed - index is fresh
                    return Ok((IndexStatus::Fresh, true, None));
                }
            }
        }

        // Not in a git repo or couldn't get git info - assume fresh
        Ok((IndexStatus::Fresh, true, None))
    }

    /// Check index freshness and show non-blocking warnings
    ///
    /// This performs lightweight checks to warn users if their index might be stale:
    /// 1. Branch mismatch: indexed different branch
    /// 2. Commit changed: HEAD moved since indexing
    /// 3. File changes: quick mtime check on sample of files (if available)
    fn check_index_freshness(&self) -> Result<()> {
        let root = std::env::current_dir()?;

        // Check git state if in a git repo
        if crate::git::is_git_repo(&root) {
            if let Ok(current_branch) = crate::git::get_current_branch(&root) {
                // Check if we're on a different branch than what was indexed
                if !self.cache.branch_exists(&current_branch).unwrap_or(false) {
                    eprintln!("⚠️  WARNING: Index not found for branch '{}'. Run 'rfx index' to index this branch.", current_branch);
                    return Ok(());
                }

                // Branch exists - check if commit changed
                if let (Ok(current_commit), Ok(branch_info)) =
                    (crate::git::get_current_commit(&root), self.cache.get_branch_info(&current_branch)) {

                    if branch_info.commit_sha != current_commit {
                        eprintln!("⚠️  WARNING: Index may be stale (commit changed: {} → {}). Consider running 'rfx index'.",
                                 &branch_info.commit_sha[..7], &current_commit[..7]);
                        return Ok(());
                    }

                    // If commits match, do a quick file freshness check
                    // Sample up to 10 files to check for modifications (cheap mtime check)
                    if let Ok(branch_files) = self.cache.get_branch_files(&current_branch) {
                        let mut checked = 0;
                        let mut changed = 0;
                        const SAMPLE_SIZE: usize = 10;

                        for (path, _indexed_hash) in branch_files.iter().take(SAMPLE_SIZE) {
                            checked += 1;
                            let file_path = std::path::Path::new(path);

                            // Check if file exists and has been modified (mtime/size heuristic)
                            if let Ok(metadata) = std::fs::metadata(file_path) {
                                if let Ok(modified) = metadata.modified() {
                                    let indexed_time = branch_info.last_indexed;
                                    let file_time = modified.duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_secs() as i64;

                                    // If file modified after indexing, it might be stale
                                    if file_time > indexed_time {
                                        // File modified after indexing - likely stale
                                        // Note: We skip hash verification for performance (mtime check is sufficient)
                                        // This may cause false positives if files were touched without changes,
                                        // but the warning is non-blocking and vastly better than slow queries
                                        changed += 1;
                                    }
                                }
                            }
                        }

                        if changed > 0 {
                            eprintln!("⚠️  WARNING: {} of {} sampled files changed since indexing. Consider running 'rfx index'.", changed, checked);
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indexer::Indexer;
    use crate::models::IndexConfig;
    use std::fs;
    use tempfile::TempDir;

    // ==================== Basic Tests ====================

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

    // ==================== Search Mode Tests ====================

    #[test]
    fn test_fulltext_search() {
        let temp = TempDir::new().unwrap();
        let project = temp.path().join("project");
        fs::create_dir(&project).unwrap();

        // Create test files
        fs::write(project.join("main.rs"), "fn main() {\n    println!(\"hello\");\n}").unwrap();
        fs::write(project.join("lib.rs"), "pub fn hello() {}").unwrap();

        // Index the project
        let cache = CacheManager::new(&project);
        let indexer = Indexer::new(cache, IndexConfig::default());
        indexer.index(&project, false).unwrap();

        // Search for "hello"
        let cache = CacheManager::new(&project);
        let engine = QueryEngine::new(cache);
        let filter = QueryFilter::default(); // full-text mode
        let results = engine.search("hello", filter).unwrap();

        // Should find both occurrences (println and function name)
        assert!(results.len() >= 2);
        assert!(results.iter().any(|r| r.path.contains("main.rs")));
        assert!(results.iter().any(|r| r.path.contains("lib.rs")));
    }

    #[test]
    fn test_symbol_search() {
        let temp = TempDir::new().unwrap();
        let project = temp.path().join("project");
        fs::create_dir(&project).unwrap();

        // Create test file with function definition and call
        fs::write(
            project.join("main.rs"),
            "fn greet() {}\nfn main() {\n    greet();\n}"
        ).unwrap();

        // Index
        let cache = CacheManager::new(&project);
        let indexer = Indexer::new(cache, IndexConfig::default());
        indexer.index(&project, false).unwrap();

        let cache = CacheManager::new(&project);

        // Symbol search (definitions only)
        let engine = QueryEngine::new(cache);
        let filter = QueryFilter {
            symbols_mode: true,
            ..Default::default()
        };
        let results = engine.search("greet", filter).unwrap();

        // Should find only the definition, not the call
        assert!(results.len() >= 1);
        assert!(results.iter().any(|r| r.kind == SymbolKind::Function));
    }

    #[test]
    fn test_regex_search() {
        let temp = TempDir::new().unwrap();
        let project = temp.path().join("project");
        fs::create_dir(&project).unwrap();

        fs::write(
            project.join("main.rs"),
            "fn test1() {}\nfn test2() {}\nfn other() {}"
        ).unwrap();

        let cache = CacheManager::new(&project);
        let indexer = Indexer::new(cache, IndexConfig::default());
        indexer.index(&project, false).unwrap();

        let cache = CacheManager::new(&project);

        let engine = QueryEngine::new(cache);
        let filter = QueryFilter {
            use_regex: true,
            ..Default::default()
        };
        let results = engine.search(r"fn test\d", filter).unwrap();

        // Should match test1 and test2 but not other
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.preview.contains("test")));
    }

    // ==================== Filter Tests ====================

    #[test]
    fn test_language_filter() {
        let temp = TempDir::new().unwrap();
        let project = temp.path().join("project");
        fs::create_dir(&project).unwrap();

        fs::write(project.join("main.rs"), "fn main() {}").unwrap();
        fs::write(project.join("main.js"), "function main() {}").unwrap();

        let cache = CacheManager::new(&project);
        let indexer = Indexer::new(cache, IndexConfig::default());
        indexer.index(&project, false).unwrap();

        let cache = CacheManager::new(&project);

        let engine = QueryEngine::new(cache);

        // Filter to Rust only
        let filter = QueryFilter {
            language: Some(Language::Rust),
            ..Default::default()
        };
        let results = engine.search("main", filter).unwrap();

        assert!(results.iter().all(|r| r.lang == Language::Rust));
        assert!(results.iter().all(|r| r.path.ends_with(".rs")));
    }

    #[test]
    fn test_kind_filter() {
        let temp = TempDir::new().unwrap();
        let project = temp.path().join("project");
        fs::create_dir(&project).unwrap();

        fs::write(
            project.join("main.rs"),
            "struct Point {}\nfn main() {}\nimpl Point { fn new() {} }"
        ).unwrap();

        let cache = CacheManager::new(&project);
        let indexer = Indexer::new(cache, IndexConfig::default());
        indexer.index(&project, false).unwrap();

        let cache = CacheManager::new(&project);

        let engine = QueryEngine::new(cache);

        // Filter to functions only (includes methods)
        let filter = QueryFilter {
            symbols_mode: true,
            kind: Some(SymbolKind::Function),
            use_contains: true,  // "mai" is substring of "main"
            ..Default::default()
        };
        // Search for "mai" which should match "main" (tri gram pattern will def be in index)
        let results = engine.search("mai", filter).unwrap();

        // Should find main function
        assert!(results.len() > 0, "Should find at least one result");
        assert!(results.iter().any(|r| r.symbol.as_deref() == Some("main")), "Should find 'main' function");
    }

    #[test]
    fn test_file_pattern_filter() {
        let temp = TempDir::new().unwrap();
        let project = temp.path().join("project");
        fs::create_dir_all(project.join("src")).unwrap();
        fs::create_dir_all(project.join("tests")).unwrap();

        fs::write(project.join("src/lib.rs"), "fn foo() {}").unwrap();
        fs::write(project.join("tests/test.rs"), "fn foo() {}").unwrap();

        let cache = CacheManager::new(&project);
        let indexer = Indexer::new(cache, IndexConfig::default());
        indexer.index(&project, false).unwrap();

        let cache = CacheManager::new(&project);

        let engine = QueryEngine::new(cache);

        // Filter to src/ only
        let filter = QueryFilter {
            file_pattern: Some("src/".to_string()),
            ..Default::default()
        };
        let results = engine.search("foo", filter).unwrap();

        assert!(results.iter().all(|r| r.path.contains("src/")));
        assert!(!results.iter().any(|r| r.path.contains("tests/")));
    }

    #[test]
    fn test_limit_filter() {
        let temp = TempDir::new().unwrap();
        let project = temp.path().join("project");
        fs::create_dir(&project).unwrap();

        // Create file with many matches
        let content = (0..20).map(|i| format!("fn test{}() {{}}", i)).collect::<Vec<_>>().join("\n");
        fs::write(project.join("main.rs"), content).unwrap();

        let cache = CacheManager::new(&project);
        let indexer = Indexer::new(cache, IndexConfig::default());
        indexer.index(&project, false).unwrap();

        let cache = CacheManager::new(&project);

        let engine = QueryEngine::new(cache);

        // Limit to 5 results
        let filter = QueryFilter {
            limit: Some(5),
            use_contains: true,  // "test" is substring of "test0", "test1", etc.
            ..Default::default()
        };
        let results = engine.search("test", filter).unwrap();

        assert_eq!(results.len(), 5);
    }

    #[test]
    fn test_exact_match_filter() {
        let temp = TempDir::new().unwrap();
        let project = temp.path().join("project");
        fs::create_dir(&project).unwrap();

        fs::write(
            project.join("main.rs"),
            "fn test() {}\nfn test_helper() {}\nfn other_test() {}"
        ).unwrap();

        let cache = CacheManager::new(&project);
        let indexer = Indexer::new(cache, IndexConfig::default());
        indexer.index(&project, false).unwrap();

        let cache = CacheManager::new(&project);

        let engine = QueryEngine::new(cache);

        // Exact match for "test"
        let filter = QueryFilter {
            symbols_mode: true,
            exact: true,
            ..Default::default()
        };
        let results = engine.search("test", filter).unwrap();

        // Should only match exactly "test", not "test_helper" or "other_test"
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].symbol.as_deref(), Some("test"));
    }

    // ==================== Expand Mode Tests ====================

    #[test]
    fn test_expand_mode() {
        let temp = TempDir::new().unwrap();
        let project = temp.path().join("project");
        fs::create_dir(&project).unwrap();

        fs::write(
            project.join("main.rs"),
            "fn greet() {\n    println!(\"Hello\");\n    println!(\"World\");\n}"
        ).unwrap();

        let cache = CacheManager::new(&project);
        let indexer = Indexer::new(cache, IndexConfig::default());
        indexer.index(&project, false).unwrap();

        let cache = CacheManager::new(&project);

        let engine = QueryEngine::new(cache);

        // Search with expand mode
        let filter = QueryFilter {
            symbols_mode: true,
            expand: true,
            ..Default::default()
        };
        let results = engine.search("greet", filter).unwrap();

        // Should have full function body in preview
        assert!(results.len() >= 1);
        let result = &results[0];
        assert!(result.preview.contains("println"));
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_search_empty_index() {
        let temp = TempDir::new().unwrap();
        let project = temp.path().join("project");
        fs::create_dir(&project).unwrap();

        let cache = CacheManager::new(&project);
        let indexer = Indexer::new(cache, IndexConfig::default());
        indexer.index(&project, false).unwrap();

        let cache = CacheManager::new(&project);

        let engine = QueryEngine::new(cache);
        let filter = QueryFilter::default();
        let results = engine.search("nonexistent", filter).unwrap();

        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_search_no_index() {
        let temp = TempDir::new().unwrap();
        let project = temp.path().join("project");
        fs::create_dir(&project).unwrap();

        let cache = CacheManager::new(&project);
        let engine = QueryEngine::new(cache);
        let filter = QueryFilter::default();

        // Should fail when index doesn't exist
        assert!(engine.search("test", filter).is_err());
    }

    #[test]
    fn test_search_special_characters() {
        let temp = TempDir::new().unwrap();
        let project = temp.path().join("project");
        fs::create_dir(&project).unwrap();

        fs::write(project.join("main.rs"), "let x = 42;\nlet y = x + 1;").unwrap();

        let cache = CacheManager::new(&project);
        let indexer = Indexer::new(cache, IndexConfig::default());
        indexer.index(&project, false).unwrap();

        let cache = CacheManager::new(&project);

        let engine = QueryEngine::new(cache);
        let filter = QueryFilter::default();

        // Search for special characters
        let results = engine.search("x + ", filter).unwrap();
        assert!(results.len() >= 1);
    }

    #[test]
    fn test_search_unicode() {
        let temp = TempDir::new().unwrap();
        let project = temp.path().join("project");
        fs::create_dir(&project).unwrap();

        fs::write(project.join("main.rs"), "// 你好世界\nfn main() {}").unwrap();

        let cache = CacheManager::new(&project);
        let indexer = Indexer::new(cache, IndexConfig::default());
        indexer.index(&project, false).unwrap();

        let cache = CacheManager::new(&project);

        let engine = QueryEngine::new(cache);
        let filter = QueryFilter {
            use_contains: true,  // Unicode word boundaries may not work as expected
            ..Default::default()
        };

        // Search for unicode characters
        let results = engine.search("你好", filter).unwrap();
        assert!(results.len() >= 1);
    }

    #[test]
    fn test_case_sensitive_search() {
        let temp = TempDir::new().unwrap();
        let project = temp.path().join("project");
        fs::create_dir(&project).unwrap();

        fs::write(project.join("main.rs"), "fn Test() {}\nfn test() {}").unwrap();

        let cache = CacheManager::new(&project);
        let indexer = Indexer::new(cache, IndexConfig::default());
        indexer.index(&project, false).unwrap();

        let cache = CacheManager::new(&project);

        let engine = QueryEngine::new(cache);
        let filter = QueryFilter::default();

        // Search is case-sensitive
        let results = engine.search("Test", filter).unwrap();
        assert!(results.iter().any(|r| r.preview.contains("Test()")));
    }

    // ==================== Determinism Tests ====================

    #[test]
    fn test_results_sorted_deterministically() {
        let temp = TempDir::new().unwrap();
        let project = temp.path().join("project");
        fs::create_dir(&project).unwrap();

        fs::write(project.join("a.rs"), "fn test() {}").unwrap();
        fs::write(project.join("z.rs"), "fn test() {}").unwrap();
        fs::write(project.join("m.rs"), "fn test() {}\nfn test2() {}").unwrap();

        let cache = CacheManager::new(&project);
        let indexer = Indexer::new(cache, IndexConfig::default());
        indexer.index(&project, false).unwrap();

        let cache = CacheManager::new(&project);

        let engine = QueryEngine::new(cache);
        let filter = QueryFilter::default();

        // Run search multiple times
        let results1 = engine.search("test", filter.clone()).unwrap();
        let results2 = engine.search("test", filter.clone()).unwrap();
        let results3 = engine.search("test", filter).unwrap();

        // Results should be identical and sorted by path then line
        assert_eq!(results1.len(), results2.len());
        assert_eq!(results1.len(), results3.len());

        for i in 0..results1.len() {
            assert_eq!(results1[i].path, results2[i].path);
            assert_eq!(results1[i].path, results3[i].path);
            assert_eq!(results1[i].span.start_line, results2[i].span.start_line);
            assert_eq!(results1[i].span.start_line, results3[i].span.start_line);
        }

        // Verify sorting (path ascending, then line ascending)
        for i in 0..results1.len().saturating_sub(1) {
            let curr = &results1[i];
            let next = &results1[i + 1];
            assert!(
                curr.path < next.path ||
                (curr.path == next.path && curr.span.start_line <= next.span.start_line)
            );
        }
    }

    // ==================== Combined Filter Tests ====================

    #[test]
    fn test_multiple_filters_combined() {
        let temp = TempDir::new().unwrap();
        let project = temp.path().join("project");
        fs::create_dir_all(project.join("src")).unwrap();

        fs::write(project.join("src/main.rs"), "fn test() {}\nstruct Test {}").unwrap();
        fs::write(project.join("src/lib.rs"), "fn test() {}").unwrap();
        fs::write(project.join("test.js"), "function test() {}").unwrap();

        let cache = CacheManager::new(&project);
        let indexer = Indexer::new(cache, IndexConfig::default());
        indexer.index(&project, false).unwrap();

        let cache = CacheManager::new(&project);

        let engine = QueryEngine::new(cache);

        // Combine language, kind, and file pattern filters
        let filter = QueryFilter {
            language: Some(Language::Rust),
            kind: Some(SymbolKind::Function),
            file_pattern: Some("src/main".to_string()),
            symbols_mode: true,
            ..Default::default()
        };
        let results = engine.search("test", filter).unwrap();

        // Should only find the function in src/main.rs
        assert_eq!(results.len(), 1);
        assert!(results[0].path.contains("src/main.rs"));
        assert_eq!(results[0].kind, SymbolKind::Function);
    }

    // ==================== Helper Method Tests ====================

    #[test]
    fn test_find_symbol_helper() {
        let temp = TempDir::new().unwrap();
        let project = temp.path().join("project");
        fs::create_dir(&project).unwrap();

        fs::write(project.join("main.rs"), "fn greet() {}").unwrap();

        let cache = CacheManager::new(&project);
        let indexer = Indexer::new(cache, IndexConfig::default());
        indexer.index(&project, false).unwrap();

        let cache = CacheManager::new(&project);

        let engine = QueryEngine::new(cache);
        let results = engine.find_symbol("greet").unwrap();

        assert!(results.len() >= 1);
        assert_eq!(results[0].kind, SymbolKind::Function);
    }

    #[test]
    fn test_list_by_kind_helper() {
        let temp = TempDir::new().unwrap();
        let project = temp.path().join("project");
        fs::create_dir(&project).unwrap();

        fs::write(
            project.join("main.rs"),
            "struct Point {}\nfn test() {}\nstruct Line {}"
        ).unwrap();

        let cache = CacheManager::new(&project);
        let indexer = Indexer::new(cache, IndexConfig::default());
        indexer.index(&project, false).unwrap();

        let cache = CacheManager::new(&project);

        let engine = QueryEngine::new(cache);

        // Search for structs that contain "oin" (Point contains it, Line doesn't)
        let filter = QueryFilter {
            kind: Some(SymbolKind::Struct),
            symbols_mode: true,
            use_contains: true,  // "oin" is substring of "Point"
            ..Default::default()
        };
        let results = engine.search("oin", filter).unwrap();

        // Should find Point struct
        assert!(results.len() >= 1, "Should find at least Point struct");
        assert!(results.iter().all(|r| r.kind == SymbolKind::Struct));
        assert!(results.iter().any(|r| r.symbol.as_deref() == Some("Point")));
    }

    // ==================== Metadata Tests ====================

    #[test]
    fn test_search_with_metadata() {
        let temp = TempDir::new().unwrap();
        let project = temp.path().join("project");
        fs::create_dir(&project).unwrap();

        fs::write(project.join("main.rs"), "fn test() {}").unwrap();

        let cache = CacheManager::new(&project);
        let indexer = Indexer::new(cache, IndexConfig::default());
        indexer.index(&project, false).unwrap();

        let cache = CacheManager::new(&project);

        let engine = QueryEngine::new(cache);
        let filter = QueryFilter::default();
        let response = engine.search_with_metadata("test", filter).unwrap();

        // Check metadata is present (status might be stale if run inside git repo)
        assert!(response.results.len() >= 1);
        // Note: can_trust_results may be false if running in a git repo without branch index
    }

    // ==================== Multi-language Tests ====================

    #[test]
    fn test_search_across_languages() {
        let temp = TempDir::new().unwrap();
        let project = temp.path().join("project");
        fs::create_dir(&project).unwrap();

        fs::write(project.join("main.rs"), "fn greet() {}").unwrap();
        fs::write(project.join("main.ts"), "function greet() {}").unwrap();
        fs::write(project.join("main.py"), "def greet(): pass").unwrap();

        let cache = CacheManager::new(&project);
        let indexer = Indexer::new(cache, IndexConfig::default());
        indexer.index(&project, false).unwrap();

        let cache = CacheManager::new(&project);

        let engine = QueryEngine::new(cache);
        let filter = QueryFilter::default();
        let results = engine.search("greet", filter).unwrap();

        // Should find greet in all three languages
        assert!(results.len() >= 3);
        assert!(results.iter().any(|r| r.lang == Language::Rust));
        assert!(results.iter().any(|r| r.lang == Language::TypeScript));
        assert!(results.iter().any(|r| r.lang == Language::Python));
    }
}
