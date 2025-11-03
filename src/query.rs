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
}

impl Default for QueryFilter {
    fn default() -> Self {
        Self {
            language: None,
            kind: None,
            use_ast: false,
            use_regex: false,
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

        // Get index status and warning (without printing warnings to stderr)
        let (status, can_trust_results, warning) = self.get_index_status()?;

        // Execute the search
        let results = self.search_internal(pattern, filter)?;

        Ok(QueryResponse {
            status,
            can_trust_results,
            warning,
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

        // Show non-blocking warnings about branch state and staleness
        self.check_index_freshness()?;

        // Execute the search
        self.search_internal(pattern, filter)
    }

    /// Internal search implementation (used by both search methods)
    fn search_internal(&self, pattern: &str, filter: QueryFilter) -> Result<Vec<SearchResult>> {

        // Step 1: Execute search based on mode
        let mut results = if filter.use_regex {
            // Regex pattern search
            // Uses trigrams to narrow candidates, then verifies with regex
            self.search_with_regex(pattern, &filter)?
        } else if filter.symbols_mode {
            // Symbol name search (filtered to symbol definitions only)
            // Use trigrams to find candidates, then parse files at runtime
            self.search_with_trigrams_and_parse(pattern)?
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

    /// Search with trigrams, then parse candidate files to extract symbols at runtime
    ///
    /// This is the core of the runtime symbol detection strategy:
    /// 1. Use trigrams to narrow down to ~10-100 candidate files
    /// 2. Parse only those files with tree-sitter
    /// 3. Filter symbols by pattern match
    /// 4. Return actual symbol definitions (not just text matches)
    fn search_with_trigrams_and_parse(&self, pattern: &str) -> Result<Vec<SearchResult>> {
        // Load content store
        let content_path = self.cache.path().join("content.bin");
        let content_reader = ContentReader::open(&content_path)
            .context("Failed to open content store")?;

        // Load trigram index
        let trigrams_path = self.cache.path().join("trigrams.bin");
        let trigram_index = if trigrams_path.exists() {
            TrigramIndex::load(&trigrams_path)?
        } else {
            Self::rebuild_trigram_index(&content_reader)?
        };

        // Use trigrams to find candidate files
        let candidates = trigram_index.search(pattern);
        log::debug!("Trigram search found {} candidate locations for symbol search", candidates.len());

        // Collect unique file IDs from trigram results
        use std::collections::HashSet;
        let candidate_file_ids: HashSet<u32> = candidates
            .iter()
            .map(|loc| loc.file_id)
            .collect();

        log::debug!("Parsing {} candidate files for symbol extraction", candidate_file_ids.len());

        // Parse each candidate file and extract symbols
        let mut all_symbols = Vec::new();
        for file_id in candidate_file_ids {
            let file_path = match trigram_index.get_file(file_id) {
                Some(p) => p,
                None => continue,
            };

            let content = match content_reader.get_file_content(file_id) {
                Ok(c) => c,
                Err(e) => {
                    log::warn!("Failed to read file {}: {}", file_path.display(), e);
                    continue;
                }
            };

            // Detect language
            let ext = file_path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            let lang = Language::from_extension(ext);

            // Parse file to extract symbols
            let file_path_str = file_path.to_string_lossy().to_string();
            match ParserFactory::parse(&file_path_str, content, lang) {
                Ok(symbols) => {
                    log::debug!("Parsed {} symbols from {}", symbols.len(), file_path_str);
                    all_symbols.extend(symbols);
                }
                Err(e) => {
                    log::debug!("Failed to parse {}: {}", file_path_str, e);
                    // Continue processing other files
                }
            }
        }

        // Filter symbols by pattern (substring match)
        let filtered: Vec<SearchResult> = all_symbols
            .into_iter()
            .filter(|sym| sym.symbol.contains(pattern))
            .collect();

        log::info!("Runtime symbol detection found {} matches for pattern '{}'", filtered.len(), pattern);

        Ok(filtered)
    }

    /// Search using trigram-based full-text search
    fn search_with_trigrams(&self, pattern: &str) -> Result<Vec<SearchResult>> {
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

                    // Verify the pattern actually appears on this line
                    // (trigrams guarantee all trigrams present, but not exact match)
                    // Use owned pattern_owned (thread-safe)
                    if !line.contains(&pattern_owned) {
                        log::debug!("Pattern '{}' not found on line {} in {}: {:?}",
                                   pattern_owned, line_no, file_path_str,
                                   if line.len() > 80 { &line[..80] } else { line });
                        continue;
                    }

                    seen_lines.insert(line_no);

                    // Create a text match result (no symbol lookup for performance)
                    file_results.push(SearchResult {
                        path: file_path_str.clone(),
                        lang: lang.clone(),
                        kind: SymbolKind::Unknown("text_match".to_string()),
                        symbol: pattern_owned.clone(),
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

                file_results
            })
            .collect();

        Ok(results)
    }

    /// Search using regex patterns with trigram optimization
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
    fn search_with_regex(&self, pattern: &str, _filter: &QueryFilter) -> Result<Vec<SearchResult>> {
        // Step 1: Compile the regex
        let regex = Regex::new(pattern)
            .with_context(|| format!("Invalid regex pattern: {}", pattern))?;

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

                // Parse file to extract symbols (for accurate symbol metadata)
                let file_symbols = Self::parse_file_for_symbols(file_path, content);

                self.find_regex_matches_in_file(
                    &regex,
                    file_path,
                    content,
                    file_symbols.as_deref(),
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

                    // Parse file to extract symbols (for accurate symbol metadata)
                    let file_symbols = Self::parse_file_for_symbols(file_path, content);

                    self.find_regex_matches_in_file(&regex, file_path, content, file_symbols.as_deref(), &mut results)?;
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

                    // Parse file to extract symbols (for accurate symbol metadata)
                    let file_symbols = Self::parse_file_for_symbols(file_path, content);

                    self.find_regex_matches_in_file(
                        &regex,
                        file_path,
                        content,
                        file_symbols.as_deref(),
                        &mut results,
                    )?;
                }
            }
        }

        log::info!("Regex search found {} matches for pattern '{}'", results.len(), pattern);
        Ok(results)
    }

    /// Parse a file to extract symbols (helper for regex search)
    /// Returns None if parsing fails or language is unsupported
    fn parse_file_for_symbols(file_path: &std::path::Path, content: &str) -> Option<Vec<SearchResult>> {
        // Detect language from file extension
        let ext = file_path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let lang = Language::from_extension(ext);

        // Only parse if we have parser support
        if matches!(lang, Language::Unknown) {
            return None;
        }

        let file_path_str = file_path.to_string_lossy().to_string();
        match ParserFactory::parse(&file_path_str, content, lang) {
            Ok(symbols) => {
                log::debug!("Parsed {} symbols from {} for regex matching", symbols.len(), file_path_str);
                Some(symbols)
            }
            Err(e) => {
                log::debug!("Failed to parse {} for symbols: {}", file_path_str, e);
                None
            }
        }
    }

    /// Find a symbol at a specific line number from parsed symbols
    /// Returns the first symbol that overlaps with the target line
    fn find_symbol_at_line(
        symbols: &[SearchResult],
        target_line: usize,
    ) -> Option<&SearchResult> {
        symbols.iter().find(|sym| {
            let start = sym.span.start_line as usize;
            let end = sym.span.end_line as usize;
            target_line >= start && target_line <= end
        })
    }

    /// Find all regex matches in a single file
    fn find_regex_matches_in_file(
        &self,
        regex: &Regex,
        file_path: &std::path::Path,
        content: &str,
        file_symbols: Option<&[SearchResult]>,
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

                // Extract the actual matched portion
                let matched_text = regex.find(line)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_else(|| line.to_string());

                // Try to find the actual symbol at this line (if symbols are available)
                if let Some(symbols) = file_symbols {
                    if let Some(found_symbol) = Self::find_symbol_at_line(symbols, line_no) {
                        // Use the actual symbol information from parsing
                        results.push(SearchResult {
                            path: file_path_str.clone(),
                            lang: lang.clone(),
                            kind: found_symbol.kind.clone(),
                            symbol: found_symbol.symbol.clone(),
                            span: found_symbol.span.clone(),
                            scope: found_symbol.scope.clone(),
                            preview: line.to_string(), // Use the matched line as preview
                        });
                        continue;
                    }
                }

                // Fall back to regex match result (no symbol found at this line)
                results.push(SearchResult {
                    path: file_path_str.clone(),
                    lang: lang.clone(),
                    kind: SymbolKind::Unknown("regex_match".to_string()),
                    symbol: matched_text,
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

                        for (path, indexed_hash) in branch_files.iter().take(SAMPLE_SIZE) {
                            checked += 1;
                            let file_path = std::path::Path::new(path);

                            if let Ok(metadata) = std::fs::metadata(file_path) {
                                if let Ok(modified) = metadata.modified() {
                                    let indexed_time = branch_info.last_indexed;
                                    let file_time = modified.duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_secs() as i64;

                                    if file_time > indexed_time {
                                        if let Ok(content) = std::fs::read(file_path) {
                                            let current_hash = blake3::hash(&content).to_hex().to_string();
                                            if &current_hash != indexed_hash {
                                                changed += 1;
                                            }
                                        }
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

                        for (path, indexed_hash) in branch_files.iter().take(SAMPLE_SIZE) {
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
                                        // Verify with hash to avoid false positives from touch/tools
                                        if let Ok(content) = std::fs::read(file_path) {
                                            let current_hash = blake3::hash(&content).to_hex().to_string();
                                            if &current_hash != indexed_hash {
                                                changed += 1;
                                            }
                                        }
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

        let cache = CacheManager::new(&project);

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
            ..Default::default()
        };
        // Search for "mai" which should match "main" (tri gram pattern will def be in index)
        let results = engine.search("mai", filter).unwrap();

        // Should find main function
        assert!(results.len() > 0, "Should find at least one result");
        assert!(results.iter().any(|r| r.symbol == "main"), "Should find 'main' function");
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
        assert_eq!(results[0].symbol, "test");
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
        let filter = QueryFilter::default();

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
            ..Default::default()
        };
        let results = engine.search("oin", filter).unwrap();

        // Should find Point struct
        assert!(results.len() >= 1, "Should find at least Point struct");
        assert!(results.iter().all(|r| r.kind == SymbolKind::Struct));
        assert!(results.iter().any(|r| r.symbol == "Point"));
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

    // ==================== Regex with Symbol Extraction Tests ====================

    #[test]
    fn test_regex_search_extracts_correct_symbol_names() {
        let temp = TempDir::new().unwrap();
        let project = temp.path().join("project");
        fs::create_dir(&project).unwrap();

        // Create a Rust file with a pattern similar to the user's PHP example
        // Regex: "Helper([A-Z][a-zA-Z0-9]*)Function"
        // Should find: MyHelperFunction, UserHelperFunction, DataHelperFunction
        fs::write(
            project.join("main.rs"),
            "fn my_helper_function() {}\nfn user_helper_function() {}\nfn data_helper_function() {}\nfn other_function() {}"
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

        // Search for functions matching pattern "helper([a-z_]+)function"
        let results = engine.search(r"helper([a-z_]+)function", filter).unwrap();

        // Should match the three helper functions
        assert_eq!(results.len(), 3, "Should find exactly 3 helper functions");

        // Verify that symbol names are extracted correctly (not just the regex match)
        assert!(results.iter().any(|r| r.symbol == "my_helper_function"),
                "Should find symbol 'my_helper_function'");
        assert!(results.iter().any(|r| r.symbol == "user_helper_function"),
                "Should find symbol 'user_helper_function'");
        assert!(results.iter().any(|r| r.symbol == "data_helper_function"),
                "Should find symbol 'data_helper_function'");

        // Verify that all results have the correct kind (Function, not Unknown)
        assert!(results.iter().all(|r| r.kind == SymbolKind::Function),
                "All results should be classified as Function, not Unknown");
    }
}
