//! Query engine for searching indexed code
//!
//! The query engine loads the memory-mapped cache and executes
//! deterministic searches based on lexical, structural, or symbol patterns.

use anyhow::{Context, Result};
use regex::Regex;

use crate::cache::{CacheManager, SymbolReader, SYMBOLS_BIN};
use crate::content_store::ContentReader;
use crate::models::{
    IndexStatus, IndexWarning, IndexWarningDetails, Language, QueryResponse, SearchResult, Span,
    SymbolKind,
};
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
                "Index not found. Run 'reflex index' to build the cache first."
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
                "Index not found. Run 'reflex index' to build the cache first."
            );
        }

        // Show non-blocking warnings about branch state and staleness
        self.check_index_freshness()?;

        // Execute the search
        self.search_internal(pattern, filter)
    }

    /// Internal search implementation (used by both search methods)
    fn search_internal(&self, pattern: &str, filter: QueryFilter) -> Result<Vec<SearchResult>> {

        // Step 1: Load symbol reader
        let symbols_path = self.cache.path().join(SYMBOLS_BIN);
        let reader = SymbolReader::open(&symbols_path)
            .context("Failed to open symbols cache")?;

        // Step 2: Execute search based on mode
        let mut results = if filter.use_regex {
            // Regex pattern search
            // Uses trigrams to narrow candidates, then verifies with regex
            self.search_with_regex(pattern, &filter)?
        } else if filter.symbols_mode {
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

        // Get all symbols and build lookup index by path for O(1) access
        let all_symbols = symbol_reader.read_all()?;

        // Build a HashMap: file_path -> Vec<Symbol> for faster lookup
        use std::collections::HashMap;
        let mut symbols_by_path: HashMap<String, Vec<&SearchResult>> = HashMap::new();
        for symbol in &all_symbols {
            symbols_by_path
                .entry(symbol.path.clone())
                .or_insert_with(Vec::new)
                .push(symbol);
        }

        // Verify matches and build results
        let mut results = Vec::new();

        for loc in candidates {
            let file_path = trigram_index.get_file(loc.file_id)
                .context("Invalid file_id from trigram search")?;
            let content = content_reader.get_file_content(loc.file_id)?;
            let file_path_str = file_path.to_string_lossy().to_string();

            // Detect language from file extension (once per file)
            let ext = file_path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            let lang = Language::from_extension(ext);

            // Get symbols for this file (if any)
            let file_symbols = symbols_by_path.get(&file_path_str);

            // Find all occurrences of the pattern in this file
            for (line_idx, line) in content.lines().enumerate() {
                if line.contains(pattern) {
                    let line_no = line_idx + 1;

                    // Try to find a matching symbol at this location (using pre-filtered file symbols)
                    let matching_symbol = file_symbols.and_then(|syms| {
                        syms.iter().find(|sym| {
                            line_no >= sym.span.start_line &&
                            line_no <= sym.span.end_line &&
                            line.contains(&sym.symbol)
                        })
                    });

                    if let Some(symbol) = matching_symbol {
                        // Use the actual symbol information
                        results.push((*symbol).clone());
                    } else {
                        // Fallback: create a generic text match result
                        results.push(SearchResult {
                            path: file_path_str.clone(),
                            lang: lang.clone(),
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

        // Load symbol reader to get actual symbol kinds
        let symbols_path = self.cache.path().join(SYMBOLS_BIN);
        let symbol_reader = SymbolReader::open(&symbols_path)
            .context("Failed to open symbols cache")?;
        let all_symbols = symbol_reader.read_all()?;

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
                    &all_symbols,
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
                    self.find_regex_matches_in_file(&regex, file_path, content, &all_symbols, &mut results)?;
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
                        &all_symbols,
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
        all_symbols: &[SearchResult],
        results: &mut Vec<SearchResult>,
    ) -> Result<()> {
        let file_path_str = file_path.to_string_lossy().to_string();

        // Detect language from file extension
        let ext = file_path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let lang = Language::from_extension(ext);

        // Pre-filter symbols for this file only (optimization)
        let file_symbols: Vec<&SearchResult> = all_symbols.iter()
            .filter(|sym| sym.path == file_path_str)
            .collect();

        // Find all regex matches line by line
        for (line_idx, line) in content.lines().enumerate() {
            if regex.is_match(line) {
                let line_no = line_idx + 1;

                // Try to find a symbol whose definition is on this exact line
                // AND whose symbol name matches the regex (not just the line content)
                // This ensures --kind function returns only function definitions, not calls
                // and that we only match symbols whose names match the pattern
                let matching_symbol = file_symbols.iter()
                    .find(|sym| {
                        sym.span.start_line == line_no &&
                        regex.is_match(&sym.symbol)  // Symbol name must match regex
                    });

                // Extract the actual matched portion
                let matched_text = regex.find(line)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_else(|| line.to_string());

                if let Some(symbol) = matching_symbol {
                    // Found a symbol - use its kind and full span (for --expand support)
                    results.push(SearchResult {
                        path: file_path_str.clone(),
                        lang: lang.clone(),
                        kind: symbol.kind.clone(),
                        symbol: matched_text,
                        span: symbol.span.clone(),  // Use symbol's full span
                        scope: symbol.scope.clone(),
                        preview: line.to_string(),
                    });
                } else {
                    // No symbol found - create generic text match
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
                        action_required: "reflex index".to_string(),
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
                            action_required: "reflex index".to_string(),
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
                                action_required: "reflex index".to_string(),
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
                    eprintln!("⚠️  WARNING: Index not found for branch '{}'. Run 'reflex index' to index this branch.", current_branch);
                    return Ok(());
                }

                // Branch exists - check if commit changed
                if let (Ok(current_commit), Ok(branch_info)) =
                    (crate::git::get_current_commit(&root), self.cache.get_branch_info(&current_branch)) {

                    if branch_info.commit_sha != current_commit {
                        eprintln!("⚠️  WARNING: Index may be stale (commit changed: {} → {}). Consider running 'reflex index'.",
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
                            eprintln!("⚠️  WARNING: {} of {} sampled files changed since indexing. Consider running 'reflex index'.", changed, checked);
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
