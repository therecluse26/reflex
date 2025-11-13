//! CLI argument parsing and command handlers

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::time::Instant;

use crate::cache::CacheManager;
use crate::indexer::Indexer;
use crate::models::{IndexConfig, Language};
use crate::output;
use crate::query::{QueryEngine, QueryFilter};

/// Reflex: Local-first, structure-aware code search for AI agents
#[derive(Parser, Debug)]
#[command(
    name = "rfx",
    version,
    about = "A fast, deterministic code search engine built for AI",
    long_about = "Reflex is a local-first, structure-aware code search engine that returns \
                  structured results (symbols, spans, scopes) with sub-100ms latency. \
                  Designed for AI coding agents and automation.\n\n\
                  Run 'rfx' with no arguments to launch interactive mode."
)]
pub struct Cli {
    /// Enable verbose logging (can be repeated for more verbosity)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Build or update the local code index
    Index {
        /// Directory to index (defaults to current directory)
        #[arg(value_name = "PATH", default_value = ".")]
        path: PathBuf,

        /// Force full rebuild (ignore incremental cache)
        #[arg(short, long)]
        force: bool,

        /// Languages to include (empty = all)
        #[arg(short, long, value_delimiter = ',')]
        languages: Vec<String>,

        /// Suppress all output (no progress bar, no summary)
        #[arg(short, long)]
        quiet: bool,

        /// Show background symbol indexing status
        #[arg(long)]
        status: bool,
    },

    /// Query the code index
    ///
    /// Search modes:
    ///   - Default: Word-boundary matching (precise, finds complete identifiers)
    ///     Example: rfx query "Error" → finds "Error" but not "NetworkError"
    ///     Example: rfx query "test" → finds "test" but not "test_helper"
    ///
    ///   - Symbol search: Word-boundary for text, exact match for symbols
    ///     Example: rfx query "parse" --symbols → finds only "parse" function/class
    ///     Example: rfx query "parse" --kind function → finds only "parse" functions
    ///
    ///   - Substring search: Expansive matching (opt-in with --contains)
    ///     Example: rfx query "mb" --contains → finds "mb", "kmb_dai_ops", "symbol", etc.
    ///
    ///   - Regex search: Pattern-controlled matching (opt-in with --regex)
    ///     Example: rfx query "^mb_.*" --regex → finds "mb_init", "mb_start", etc.
    Query {
        /// Search pattern
        pattern: String,

        /// Search symbol definitions only (functions, classes, etc.)
        #[arg(short, long)]
        symbols: bool,

        /// Filter by language
        /// Supported: rust, python, javascript, typescript, vue, svelte, go, java, php, c, c++, c#, ruby, kotlin, zig
        #[arg(short, long)]
        lang: Option<String>,

        /// Filter by symbol kind (implies --symbols)
        /// Supported: function, class, struct, enum, interface, trait, constant, variable, method, module, namespace, type, macro, property, event, import, export, attribute
        #[arg(short, long)]
        kind: Option<String>,

        /// Use AST pattern matching (SLOW: 500ms-2s+, scans all files)
        ///
        /// WARNING: AST queries bypass trigram optimization and scan the entire codebase.
        /// In 95% of cases, use --symbols instead which is 10-100x faster.
        ///
        /// When --ast is set, the pattern parameter is interpreted as a Tree-sitter
        /// S-expression query instead of text search.
        ///
        /// RECOMMENDED: Always use --glob to limit scope for better performance.
        ///
        /// Examples:
        ///   Fast (2-50ms):    rfx query "fetch" --symbols --kind function --lang python
        ///   Slow (500ms-2s):  rfx query "(function_definition) @fn" --ast --lang python
        ///   Faster with glob: rfx query "(class_declaration) @class" --ast --lang typescript --glob "src/**/*.ts"
        #[arg(long)]
        ast: bool,

        /// Use regex pattern matching
        #[arg(short = 'r', long)]
        regex: bool,

        /// Output format as JSON
        #[arg(long)]
        json: bool,

        /// Pretty-print JSON output (only with --json)
        /// By default, JSON is minified to reduce token usage
        #[arg(long)]
        pretty: bool,

        /// AI-optimized mode: returns JSON with ai_instruction field
        /// Implies --json (minified by default, use --pretty for formatted output)
        /// Provides context-aware guidance to AI agents on response format and next actions
        #[arg(long)]
        ai: bool,

        /// Maximum number of results
        #[arg(short = 'n', long)]
        limit: Option<usize>,

        /// Pagination offset (skip first N results after sorting)
        /// Use with --limit for pagination: --offset 0 --limit 10, then --offset 10 --limit 10
        #[arg(short = 'o', long)]
        offset: Option<usize>,

        /// Show full symbol definition (entire function/class body)
        /// Only applicable to symbol searches
        #[arg(long)]
        expand: bool,

        /// Filter by file path (supports substring matching)
        /// Example: --file math.rs or --file helpers/
        #[arg(short = 'f', long)]
        file: Option<String>,

        /// Exact symbol name match (no substring matching)
        /// Only applicable to symbol searches
        #[arg(long)]
        exact: bool,

        /// Use substring matching for both text and symbols (expansive search)
        /// Default behavior uses word-boundary and exact matching for precision
        #[arg(long)]
        contains: bool,

        /// Only show count and timing, not the actual results
        #[arg(short, long)]
        count: bool,

        /// Query timeout in seconds (0 = no timeout, default: 30)
        #[arg(short = 't', long, default_value = "30")]
        timeout: u64,

        /// Use plain text output (disable colors and syntax highlighting)
        #[arg(long)]
        plain: bool,

        /// Include files matching glob pattern (can be repeated)
        /// Example: --glob "src/**/*.rs" --glob "tests/**/*.rs"
        #[arg(short = 'g', long)]
        glob: Vec<String>,

        /// Exclude files matching glob pattern (can be repeated)
        /// Example: --exclude "target/**" --exclude "*.gen.rs"
        #[arg(short = 'x', long)]
        exclude: Vec<String>,

        /// Return only unique file paths (no line numbers or content)
        /// Compatible with --json to output ["path1", "path2", ...]
        #[arg(short = 'p', long)]
        paths: bool,

        /// Disable smart preview truncation (show full lines)
        /// By default, previews are truncated to ~100 chars to reduce token usage
        #[arg(long)]
        no_truncate: bool,

        /// Return all results (no limit)
        /// Equivalent to --limit 0, convenience flag for getting unlimited results
        #[arg(short = 'a', long)]
        all: bool,

        /// Force execution of potentially expensive queries
        /// Bypasses broad query detection that prevents queries with:
        /// • Short patterns (< 3 characters)
        /// • High candidate counts (> 5,000 files for symbol/AST queries)
        /// • AST queries without --glob restrictions
        #[arg(long)]
        force: bool,

        /// Include dependency information (imports) in results
        /// Currently only available for Rust files
        #[arg(long)]
        dependencies: bool,
    },

    /// Start a local HTTP API server
    Serve {
        /// Port to listen on
        #[arg(short, long, default_value = "7878")]
        port: u16,

        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
    },

    /// Show index statistics and cache information
    Stats {
        /// Output format as JSON
        #[arg(long)]
        json: bool,

        /// Pretty-print JSON output (only with --json)
        #[arg(long)]
        pretty: bool,
    },

    /// Clear the local cache
    Clear {
        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },

    /// List all indexed files
    ListFiles {
        /// Output format as JSON
        #[arg(long)]
        json: bool,

        /// Pretty-print JSON output (only with --json)
        #[arg(long)]
        pretty: bool,
    },

    /// Watch for file changes and auto-reindex
    ///
    /// Continuously monitors the workspace for changes and automatically
    /// triggers incremental reindexing. Useful for IDE integrations and
    /// keeping the index always fresh during active development.
    ///
    /// The debounce timer resets on every file change, batching rapid edits
    /// (e.g., multi-file refactors, format-on-save) into a single reindex.
    Watch {
        /// Directory to watch (defaults to current directory)
        #[arg(value_name = "PATH", default_value = ".")]
        path: PathBuf,

        /// Debounce duration in milliseconds (default: 15000 = 15s)
        /// Waits this long after the last change before reindexing
        /// Valid range: 5000-30000 (5-30 seconds)
        #[arg(short, long, default_value = "15000")]
        debounce: u64,

        /// Suppress output (only log errors)
        #[arg(short, long)]
        quiet: bool,
    },

    /// Start MCP server for AI agent integration
    ///
    /// Runs Reflex as a Model Context Protocol (MCP) server using stdio transport.
    /// This command is automatically invoked by MCP clients like Claude Code and
    /// should not be run manually.
    ///
    /// Configuration example for Claude Code (~/.claude/claude_code_config.json):
    /// {
    ///   "mcpServers": {
    ///     "reflex": {
    ///       "type": "stdio",
    ///       "command": "rfx",
    ///       "args": ["mcp"]
    ///     }
    ///   }
    /// }
    Mcp,

    /// Analyze file dependencies and imports
    ///
    /// Show dependencies, dependents, and perform graph analysis
    /// to understand code relationships and architecture.
    ///
    /// Examples:
    ///   rfx deps src/main.rs                  # Show dependencies
    ///   rfx deps src/config.rs --reverse      # Show dependents
    ///   rfx deps src/api.rs --depth 3         # Transitive deps
    ///   rfx deps --circular                   # Find cycles
    ///   rfx deps --hotspots                   # Most-imported files
    ///   rfx deps --unused                     # Orphaned files
    Deps {
        /// File path to analyze (omit for graph-wide operations)
        file: Option<PathBuf>,

        /// Show files that depend on this file (reverse lookup)
        #[arg(short, long)]
        reverse: bool,

        /// Traversal depth for transitive dependencies (default: 1)
        #[arg(short, long, default_value = "1")]
        depth: usize,

        /// Output format: json (default), tree, table, dot
        #[arg(short = 'f', long, default_value = "json")]
        format: String,

        /// Pretty-print JSON output (only with --format json)
        #[arg(long)]
        pretty: bool,

        /// Filter to internal dependencies only
        #[arg(long)]
        only_internal: bool,

        /// Filter to external dependencies only
        #[arg(long)]
        only_external: bool,

        /// Filter to stdlib dependencies only
        #[arg(long)]
        only_stdlib: bool,

        /// Find circular dependencies (graph-wide)
        #[arg(long, conflicts_with = "file")]
        circular: bool,

        /// Find most-imported files (hotspots)
        #[arg(long, conflicts_with = "file")]
        hotspots: bool,

        /// Find unused files (no incoming dependencies)
        #[arg(long, conflicts_with = "file")]
        unused: bool,

        /// Find disconnected components (islands)
        #[arg(long, conflicts_with = "file")]
        islands: bool,

        /// Full analysis report (runs all analyses)
        #[arg(long, conflicts_with = "file")]
        analyze: bool,

        /// Maximum number of results to return
        #[arg(short = 'n', long)]
        limit: Option<usize>,
    },

    /// Internal command: Run background symbol indexing (hidden from help)
    #[command(hide = true)]
    IndexSymbolsInternal {
        /// Cache directory path
        cache_dir: PathBuf,
    },
}

impl Cli {
    /// Execute the CLI command
    pub fn execute(self) -> Result<()> {
        // Setup logging based on verbosity
        let log_level = match self.verbose {
            0 => "warn",   // Default: only warnings and errors
            1 => "info",   // -v: show info messages
            2 => "debug",  // -vv: show debug messages
            _ => "trace",  // -vvv: show trace messages
        };
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level))
            .init();

        // Execute the subcommand, or launch interactive mode if no command provided
        match self.command {
            None => {
                // No subcommand: launch interactive mode
                handle_interactive()
            }
            Some(Command::Index { path, force, languages, quiet, status }) => {
                handle_index(path, force, languages, quiet, status)
            }
            Some(Command::Query { pattern, symbols, lang, kind, ast, regex, json, pretty, ai, limit, offset, expand, file, exact, contains, count, timeout, plain, glob, exclude, paths, no_truncate, all, force, dependencies }) => {
                handle_query(pattern, symbols, lang, kind, ast, regex, json, pretty, ai, limit, offset, expand, file, exact, contains, count, timeout, plain, glob, exclude, paths, no_truncate, all, force, dependencies)
            }
            Some(Command::Serve { port, host }) => {
                handle_serve(port, host)
            }
            Some(Command::Stats { json, pretty }) => {
                handle_stats(json, pretty)
            }
            Some(Command::Clear { yes }) => {
                handle_clear(yes)
            }
            Some(Command::ListFiles { json, pretty }) => {
                handle_list_files(json, pretty)
            }
            Some(Command::Watch { path, debounce, quiet }) => {
                handle_watch(path, debounce, quiet)
            }
            Some(Command::Mcp) => {
                handle_mcp()
            }
            Some(Command::Deps { file, reverse, depth, format, pretty, only_internal, only_external, only_stdlib, circular, hotspots, unused, islands, analyze, limit }) => {
                handle_deps(file, reverse, depth, format, pretty, only_internal, only_external, only_stdlib, circular, hotspots, unused, islands, analyze, limit)
            }
            Some(Command::IndexSymbolsInternal { cache_dir }) => {
                handle_index_symbols_internal(cache_dir)
            }
        }
    }
}

/// Handle the `index` subcommand
fn handle_index(path: PathBuf, force: bool, languages: Vec<String>, quiet: bool, show_status: bool) -> Result<()> {
    log::info!("Starting index command");

    let cache = CacheManager::new(&path);
    let cache_path = cache.path().to_path_buf();

    // Handle --status flag
    if show_status {
        match crate::background_indexer::BackgroundIndexer::get_status(&cache_path) {
            Ok(Some(status)) => {
                println!("Background Symbol Indexing Status");
                println!("==================================");
                println!("State:           {:?}", status.state);
                println!("Total files:     {}", status.total_files);
                println!("Processed:       {}", status.processed_files);
                println!("Cached:          {}", status.cached_files);
                println!("Parsed:          {}", status.parsed_files);
                println!("Failed:          {}", status.failed_files);
                println!("Started:         {}", status.started_at);
                println!("Last updated:    {}", status.updated_at);

                if let Some(completed_at) = &status.completed_at {
                    println!("Completed:       {}", completed_at);
                }

                if let Some(error) = &status.error {
                    println!("Error:           {}", error);
                }

                // Show progress percentage if running
                if status.state == crate::background_indexer::IndexerState::Running && status.total_files > 0 {
                    let progress = (status.processed_files as f64 / status.total_files as f64) * 100.0;
                    println!("\nProgress:        {:.1}%", progress);
                }

                return Ok(());
            }
            Ok(None) => {
                println!("No background symbol indexing in progress.");
                println!("\nRun 'rfx index' to start background symbol indexing.");
                return Ok(());
            }
            Err(e) => {
                anyhow::bail!("Failed to get indexing status: {}", e);
            }
        }
    }

    if force {
        log::info!("Force rebuild requested, clearing existing cache");
        cache.clear()?;
    }

    // Parse language filters
    let lang_filters: Vec<Language> = languages
        .iter()
        .filter_map(|s| match s.to_lowercase().as_str() {
            "rust" | "rs" => Some(Language::Rust),
            "python" | "py" => Some(Language::Python),
            "javascript" | "js" => Some(Language::JavaScript),
            "typescript" | "ts" => Some(Language::TypeScript),
            "go" => Some(Language::Go),
            "java" => Some(Language::Java),
            "php" => Some(Language::PHP),
            "c" => Some(Language::C),
            "cpp" | "c++" => Some(Language::Cpp),
            _ => {
                output::warn(&format!("Unknown language: {}", s));
                None
            }
        })
        .collect();

    let config = IndexConfig {
        languages: lang_filters,
        ..Default::default()
    };

    let indexer = Indexer::new(cache, config);
    // Show progress by default, unless quiet mode is enabled
    let show_progress = !quiet;
    let stats = indexer.index(&path, show_progress)?;

    // In quiet mode, suppress all output
    if !quiet {
        println!("Indexing complete!");
        println!("  Files indexed: {}", stats.total_files);
        println!("  Cache size: {}", format_bytes(stats.index_size_bytes));
        println!("  Last updated: {}", stats.last_updated);

        // Display language breakdown if we have indexed files
        if !stats.files_by_language.is_empty() {
            println!("\nFiles by language:");

            // Sort languages by count (descending) for consistent output
            let mut lang_vec: Vec<_> = stats.files_by_language.iter().collect();
            lang_vec.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));

            // Calculate column widths
            let max_lang_len = lang_vec.iter().map(|(lang, _)| lang.len()).max().unwrap_or(8);
            let lang_width = max_lang_len.max(8); // At least "Language" header width

            // Print table header
            println!("  {:<width$}  Files  Lines", "Language", width = lang_width);
            println!("  {}  -----  -------", "-".repeat(lang_width));

            // Print rows
            for (language, file_count) in lang_vec {
                let line_count = stats.lines_by_language.get(language).copied().unwrap_or(0);
                println!("  {:<width$}  {:5}  {:7}",
                    language, file_count, line_count,
                    width = lang_width);
            }
        }
    }

    // Start background symbol indexing (if not already running)
    if !crate::background_indexer::BackgroundIndexer::is_running(&cache_path) {
        if !quiet {
            println!("\nStarting background symbol indexing...");
            println!("  Symbols will be cached for faster queries");
            println!("  Check status with: rfx index --status");
        }

        // Spawn detached background process for symbol indexing
        // Pass the workspace root, not the .reflex directory
        let current_exe = std::env::current_exe()
            .context("Failed to get current executable path")?;

        #[cfg(unix)]
        {
            std::process::Command::new(&current_exe)
                .arg("index-symbols-internal")
                .arg(&path)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .context("Failed to spawn background indexing process")?;
        }

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;

            std::process::Command::new(&current_exe)
                .arg("index-symbols-internal")
                .arg(&path)
                .creation_flags(CREATE_NO_WINDOW)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .context("Failed to spawn background indexing process")?;
        }

        log::debug!("Spawned background symbol indexing process");
    } else if !quiet {
        println!("\n⚠️  Background symbol indexing already in progress");
        println!("  Check status with: rfx index --status");
    }

    Ok(())
}

/// Format bytes into human-readable size (KB, MB, GB, etc.)
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

/// Smart truncate preview to reduce token usage
/// Truncates at word boundary if possible, adds ellipsis if truncated
pub fn truncate_preview(preview: &str, max_length: usize) -> String {
    if preview.len() <= max_length {
        return preview.to_string();
    }

    // Find a good break point (prefer word boundary)
    let truncate_at = preview.char_indices()
        .take(max_length)
        .filter(|(_, c)| c.is_whitespace())
        .last()
        .map(|(i, _)| i)
        .unwrap_or(max_length.min(preview.len()));

    let mut truncated = preview[..truncate_at].to_string();
    truncated.push_str("…");
    truncated
}

/// Handle the `query` subcommand
fn handle_query(
    pattern: String,
    symbols_flag: bool,
    lang: Option<String>,
    kind_str: Option<String>,
    use_ast: bool,
    use_regex: bool,
    as_json: bool,
    pretty_json: bool,
    ai_mode: bool,
    limit: Option<usize>,
    offset: Option<usize>,
    expand: bool,
    file_pattern: Option<String>,
    exact: bool,
    use_contains: bool,
    count_only: bool,
    timeout_secs: u64,
    plain: bool,
    glob_patterns: Vec<String>,
    exclude_patterns: Vec<String>,
    paths_only: bool,
    no_truncate: bool,
    all: bool,
    force: bool,
    include_dependencies: bool,
) -> Result<()> {
    log::info!("Starting query command");

    // AI mode implies JSON output
    let as_json = as_json || ai_mode;

    let cache = CacheManager::new(".");
    let engine = QueryEngine::new(cache);

    // Parse and validate language filter
    let language = if let Some(lang_str) = lang.as_deref() {
        match lang_str.to_lowercase().as_str() {
            "rust" | "rs" => Some(Language::Rust),
            "python" | "py" => Some(Language::Python),
            "javascript" | "js" => Some(Language::JavaScript),
            "typescript" | "ts" => Some(Language::TypeScript),
            "vue" => Some(Language::Vue),
            "svelte" => Some(Language::Svelte),
            "go" => Some(Language::Go),
            "java" => Some(Language::Java),
            "php" => Some(Language::PHP),
            "c" => Some(Language::C),
            "cpp" | "c++" => Some(Language::Cpp),
            "csharp" | "cs" | "c#" => Some(Language::CSharp),
            "ruby" | "rb" => Some(Language::Ruby),
            "kotlin" | "kt" => Some(Language::Kotlin),
            "zig" => Some(Language::Zig),
            _ => {
                anyhow::bail!(
                    "Unknown language: '{}'\n\
                     \n\
                     Supported languages:\n\
                     • rust, rs\n\
                     • python, py\n\
                     • javascript, js\n\
                     • typescript, ts\n\
                     • vue\n\
                     • svelte\n\
                     • go\n\
                     • java\n\
                     • php\n\
                     • c\n\
                     • c++, cpp\n\
                     • c#, csharp, cs\n\
                     • ruby, rb\n\
                     • kotlin, kt\n\
                     • zig\n\
                     \n\
                     Example: rfx query \"pattern\" --lang rust",
                    lang_str
                );
            }
        }
    } else {
        None
    };

    // Parse symbol kind - try exact match first (case-insensitive), then treat as Unknown
    let kind = kind_str.as_deref().and_then(|s| {
        // Try parsing with proper case (PascalCase for SymbolKind)
        let capitalized = {
            let mut chars = s.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars.flat_map(|c| c.to_lowercase())).collect(),
            }
        };

        capitalized.parse::<crate::models::SymbolKind>()
            .ok()
            .or_else(|| {
                // If not a known kind, treat as Unknown for flexibility
                log::debug!("Treating '{}' as unknown symbol kind for filtering", s);
                Some(crate::models::SymbolKind::Unknown(s.to_string()))
            })
    });

    // Smart behavior: --kind implies --symbols
    let symbols_mode = symbols_flag || kind.is_some();

    // Smart limit handling:
    // 1. If --count is set: no limit (count should always show total)
    // 2. If --all is set: no limit (None)
    // 3. If --limit 0 is set: no limit (None) - treat 0 as "unlimited"
    // 4. If --paths is set and user didn't specify --limit: no limit (None)
    // 5. If user specified --limit: use that value
    // 6. Otherwise: use default limit of 100
    let final_limit = if count_only {
        None  // --count always shows total count, no pagination
    } else if all {
        None  // --all means no limit
    } else if limit == Some(0) {
        None  // --limit 0 means no limit (unlimited results)
    } else if paths_only && limit.is_none() {
        None  // --paths without explicit --limit means no limit
    } else if let Some(user_limit) = limit {
        Some(user_limit)  // Use user-specified limit
    } else {
        Some(100)  // Default: limit to 100 results for token efficiency
    };

    // Validate AST query requirements
    if use_ast && language.is_none() {
        anyhow::bail!(
            "AST pattern matching requires a language to be specified.\n\
             \n\
             Use --lang to specify the language for tree-sitter parsing.\n\
             \n\
             Supported languages for AST queries:\n\
             • rust, python, go, java, c, c++, c#, php, ruby, kotlin, zig, typescript, javascript\n\
             \n\
             Note: Vue and Svelte use line-based parsing and do not support AST queries.\n\
             \n\
             WARNING: AST queries are SLOW (500ms-2s+). Use --symbols instead for 95% of cases.\n\
             \n\
             Examples:\n\
             • rfx query \"(function_definition) @fn\" --ast --lang python\n\
             • rfx query \"(class_declaration) @class\" --ast --lang typescript --glob \"src/**/*.ts\""
        );
    }

    let filter = QueryFilter {
        language,
        kind,
        use_ast,
        use_regex,
        limit: final_limit,
        symbols_mode,
        expand,
        file_pattern,
        exact,
        use_contains,
        timeout_secs,
        glob_patterns: glob_patterns.clone(),
        exclude_patterns,
        paths_only,
        offset,
        force,
        suppress_output: as_json,  // Suppress warnings in JSON mode
        include_dependencies,
    };

    // Measure query time
    let start = Instant::now();

    // Execute query and get pagination metadata
    // Handle errors specially for JSON output mode
    let query_result = if use_ast {
        // AST query: pattern is the S-expression, scan all files
        engine.search_ast_all_files(&pattern, filter.clone())
            .map(|ast_results| {
                let count = ast_results.len();
                (ast_results, count, false)
            })
    } else {
        // Use metadata-aware search for all queries (to get pagination info)
        engine.search_with_metadata(&pattern, filter.clone())
            .map(|response| {
                let total = response.pagination.total;
                let has_more = response.pagination.has_more;
                (response.results, total, has_more)
            })
    };

    // Handle errors with JSON formatting when --json is set
    let (mut results, total_results, has_more) = match query_result {
        Ok(data) => data,
        Err(e) => {
            if as_json {
                // Output error as JSON
                let error_response = serde_json::json!({
                    "error": e.to_string(),
                    "query_too_broad": e.to_string().contains("Query too broad")
                });
                let json_output = if pretty_json {
                    serde_json::to_string_pretty(&error_response)?
                } else {
                    serde_json::to_string(&error_response)?
                };
                println!("{}", json_output);
                std::process::exit(1);
            } else {
                // Plain text error (default behavior)
                return Err(e);
            }
        }
    };

    // Apply preview truncation unless --no-truncate is set
    if !no_truncate {
        const MAX_PREVIEW_LENGTH: usize = 100;
        for result in &mut results {
            result.preview = truncate_preview(&result.preview, MAX_PREVIEW_LENGTH);
        }
    }

    let elapsed = start.elapsed();

    // Format timing string
    let timing_str = if elapsed.as_millis() < 1 {
        format!("{:.1}ms", elapsed.as_secs_f64() * 1000.0)
    } else {
        format!("{}ms", elapsed.as_millis())
    };

    if as_json {
        if count_only {
            // Count-only JSON mode: output simple count object
            let count_response = serde_json::json!({
                "count": total_results,
                "timing_ms": elapsed.as_millis()
            });
            let json_output = if pretty_json {
                serde_json::to_string_pretty(&count_response)?
            } else {
                serde_json::to_string(&count_response)?
            };
            println!("{}", json_output);
        } else if paths_only {
            // Paths-only JSON mode: output array of {path, line} objects
            let locations: Vec<serde_json::Value> = results.iter()
                .map(|r| serde_json::json!({
                    "path": r.path,
                    "line": r.span.start_line
                }))
                .collect();
            let json_output = if pretty_json {
                serde_json::to_string_pretty(&locations)?
            } else {
                serde_json::to_string(&locations)?
            };
            println!("{}", json_output);
            eprintln!("Found {} unique files in {}", locations.len(), timing_str);
        } else {
            // Get or build QueryResponse for JSON output
            let mut response = if use_ast {
                // For AST queries, build a response with minimal metadata
                use crate::models::{PaginationInfo, IndexStatus};
                crate::models::QueryResponse {
                    ai_instruction: None,  // Will be populated below if ai_mode is true
                    status: IndexStatus::Fresh,
                    can_trust_results: true,
                    warning: None,
                    pagination: PaginationInfo {
                        total: results.len(),
                        count: results.len(),
                        offset: offset.unwrap_or(0),
                        limit,
                        has_more: false, // AST already applied pagination
                    },
                    results,
                }
            } else {
                // Use search_with_metadata which already has pagination info
                let mut response = engine.search_with_metadata(&pattern, filter)?;
                // Replace results with truncated ones (search_with_metadata returns non-truncated)
                response.results = results;
                response
            };

            // Generate AI instruction if in AI mode
            if ai_mode {
                response.ai_instruction = crate::query::generate_ai_instruction(
                    response.results.len(),
                    response.pagination.total,
                    response.pagination.has_more,
                    symbols_mode,
                    paths_only,
                    use_ast,
                    use_regex,
                    language.is_some(),
                    !glob_patterns.is_empty(),
                    exact,
                );
            }

            let json_output = if pretty_json {
                serde_json::to_string_pretty(&response)?
            } else {
                serde_json::to_string(&response)?
            };
            println!("{}", json_output);
            eprintln!("Found {} results in {}", response.results.len(), timing_str);
        }
    } else {
        // Standard output with formatting
        if count_only {
            println!("Found {} results in {}", results.len(), timing_str);
            return Ok(());
        }

        if paths_only {
            // Paths-only plain text mode: output one path per line
            if results.is_empty() {
                eprintln!("No results found (searched in {}).", timing_str);
            } else {
                for result in &results {
                    println!("{}", result.path);
                }
                eprintln!("Found {} unique files in {}", results.len(), timing_str);
            }
        } else {
            // Standard result formatting
            if results.is_empty() {
                println!("No results found (searched in {}).", timing_str);
            } else {
                // Use formatter for pretty output
                let formatter = crate::formatter::OutputFormatter::new(plain);
                formatter.format_results(&results, &pattern)?;

                // Print summary at the bottom with pagination details
                if total_results > results.len() {
                    // Results were paginated - show detailed count
                    println!("\nFound {} results ({} total) in {}", results.len(), total_results, timing_str);
                    // Show pagination hint if there are more results available
                    if has_more {
                        println!("Use --limit and --offset to paginate");
                    }
                } else {
                    // All results shown - simple count
                    println!("\nFound {} results in {}", results.len(), timing_str);
                }
            }
        }
    }

    Ok(())
}

/// Handle the `serve` subcommand
fn handle_serve(port: u16, host: String) -> Result<()> {
    log::info!("Starting HTTP server on {}:{}", host, port);

    println!("Starting Reflex HTTP server...");
    println!("  Address: http://{}:{}", host, port);
    println!("\nEndpoints:");
    println!("  GET  /query?q=<pattern>&lang=<lang>&kind=<kind>&limit=<n>&symbols=true&regex=true&exact=true&contains=true&expand=true&file=<pattern>&timeout=<secs>&glob=<pattern>&exclude=<pattern>&paths=true&dependencies=true");
    println!("  GET  /stats");
    println!("  POST /index");
    println!("\nPress Ctrl+C to stop.");

    // Start the server using tokio runtime
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async {
        run_server(port, host).await
    })
}

/// Run the HTTP server
async fn run_server(port: u16, host: String) -> Result<()> {
    use axum::{
        extract::{Query as AxumQuery, State},
        http::StatusCode,
        response::{IntoResponse, Json},
        routing::{get, post},
        Router,
    };
    use tower_http::cors::{CorsLayer, Any};
    use std::sync::Arc;

    // Server state shared across requests
    #[derive(Clone)]
    struct AppState {
        cache_path: String,
    }

    // Query parameters for GET /query
    #[derive(Debug, serde::Deserialize)]
    struct QueryParams {
        q: String,
        #[serde(default)]
        lang: Option<String>,
        #[serde(default)]
        kind: Option<String>,
        #[serde(default)]
        limit: Option<usize>,
        #[serde(default)]
        offset: Option<usize>,
        #[serde(default)]
        symbols: bool,
        #[serde(default)]
        regex: bool,
        #[serde(default)]
        exact: bool,
        #[serde(default)]
        contains: bool,
        #[serde(default)]
        expand: bool,
        #[serde(default)]
        file: Option<String>,
        #[serde(default = "default_timeout")]
        timeout: u64,
        #[serde(default)]
        glob: Vec<String>,
        #[serde(default)]
        exclude: Vec<String>,
        #[serde(default)]
        paths: bool,
        #[serde(default)]
        force: bool,
        #[serde(default)]
        dependencies: bool,
    }

    // Default timeout for HTTP queries (30 seconds)
    fn default_timeout() -> u64 {
        30
    }

    // Request body for POST /index
    #[derive(Debug, serde::Deserialize)]
    struct IndexRequest {
        #[serde(default)]
        force: bool,
        #[serde(default)]
        languages: Vec<String>,
    }

    // GET /query endpoint
    async fn handle_query_endpoint(
        State(state): State<Arc<AppState>>,
        AxumQuery(params): AxumQuery<QueryParams>,
    ) -> Result<Json<crate::models::QueryResponse>, (StatusCode, String)> {
        log::info!("Query request: pattern={}", params.q);

        let cache = CacheManager::new(&state.cache_path);
        let engine = QueryEngine::new(cache);

        // Parse language filter
        let language = if let Some(lang_str) = params.lang.as_deref() {
            match lang_str.to_lowercase().as_str() {
                "rust" | "rs" => Some(Language::Rust),
                "javascript" | "js" => Some(Language::JavaScript),
                "typescript" | "ts" => Some(Language::TypeScript),
                "vue" => Some(Language::Vue),
                "svelte" => Some(Language::Svelte),
                "php" => Some(Language::PHP),
                "python" | "py" => Some(Language::Python),
                "go" => Some(Language::Go),
                "java" => Some(Language::Java),
                "c" => Some(Language::C),
                "cpp" | "c++" => Some(Language::Cpp),
                _ => {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        format!("Unknown language '{}'. Supported languages: rust, javascript (js), typescript (ts), vue, svelte, php, python (py), go, java, c, cpp (c++)", lang_str)
                    ));
                }
            }
        } else {
            None
        };

        // Parse symbol kind
        let kind = params.kind.as_deref().and_then(|s| {
            let capitalized = {
                let mut chars = s.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().chain(chars.flat_map(|c| c.to_lowercase())).collect(),
                }
            };

            capitalized.parse::<crate::models::SymbolKind>()
                .ok()
                .or_else(|| {
                    log::debug!("Treating '{}' as unknown symbol kind for filtering", s);
                    Some(crate::models::SymbolKind::Unknown(s.to_string()))
                })
        });

        // Smart behavior: --kind implies --symbols
        let symbols_mode = params.symbols || kind.is_some();

        // Smart limit handling (same as CLI and MCP)
        let final_limit = if params.paths && params.limit.is_none() {
            None  // --paths without explicit limit means no limit
        } else if let Some(user_limit) = params.limit {
            Some(user_limit)  // Use user-specified limit
        } else {
            Some(100)  // Default: limit to 100 results for token efficiency
        };

        let filter = QueryFilter {
            language,
            kind,
            use_ast: false,
            use_regex: params.regex,
            limit: final_limit,
            symbols_mode,
            expand: params.expand,
            file_pattern: params.file,
            exact: params.exact,
            use_contains: params.contains,
            timeout_secs: params.timeout,
            glob_patterns: params.glob,
            exclude_patterns: params.exclude,
            paths_only: params.paths,
            offset: params.offset,
            force: params.force,
            suppress_output: true,  // HTTP API always returns JSON, suppress warnings
            include_dependencies: params.dependencies,
        };

        match engine.search_with_metadata(&params.q, filter) {
            Ok(response) => Ok(Json(response)),
            Err(e) => {
                log::error!("Query error: {}", e);
                Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Query failed: {}", e)))
            }
        }
    }

    // GET /stats endpoint
    async fn handle_stats_endpoint(
        State(state): State<Arc<AppState>>,
    ) -> Result<Json<crate::models::IndexStats>, (StatusCode, String)> {
        log::info!("Stats request");

        let cache = CacheManager::new(&state.cache_path);

        if !cache.exists() {
            return Err((StatusCode::NOT_FOUND, "No index found. Run 'rfx index' first.".to_string()));
        }

        match cache.stats() {
            Ok(stats) => Ok(Json(stats)),
            Err(e) => {
                log::error!("Stats error: {}", e);
                Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to get stats: {}", e)))
            }
        }
    }

    // POST /index endpoint
    async fn handle_index_endpoint(
        State(state): State<Arc<AppState>>,
        Json(req): Json<IndexRequest>,
    ) -> Result<Json<crate::models::IndexStats>, (StatusCode, String)> {
        log::info!("Index request: force={}, languages={:?}", req.force, req.languages);

        let cache = CacheManager::new(&state.cache_path);

        if req.force {
            log::info!("Force rebuild requested, clearing existing cache");
            if let Err(e) = cache.clear() {
                return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to clear cache: {}", e)));
            }
        }

        // Parse language filters
        let lang_filters: Vec<Language> = req.languages
            .iter()
            .filter_map(|s| match s.to_lowercase().as_str() {
                "rust" | "rs" => Some(Language::Rust),
                "python" | "py" => Some(Language::Python),
                "javascript" | "js" => Some(Language::JavaScript),
                "typescript" | "ts" => Some(Language::TypeScript),
                "vue" => Some(Language::Vue),
                "svelte" => Some(Language::Svelte),
                "go" => Some(Language::Go),
                "java" => Some(Language::Java),
                "php" => Some(Language::PHP),
                "c" => Some(Language::C),
                "cpp" | "c++" => Some(Language::Cpp),
                _ => {
                    log::warn!("Unknown language: {}", s);
                    None
                }
            })
            .collect();

        let config = IndexConfig {
            languages: lang_filters,
            ..Default::default()
        };

        let indexer = Indexer::new(cache, config);
        let path = std::path::PathBuf::from(&state.cache_path);

        match indexer.index(&path, false) {
            Ok(stats) => Ok(Json(stats)),
            Err(e) => {
                log::error!("Index error: {}", e);
                Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Indexing failed: {}", e)))
            }
        }
    }

    // Health check endpoint
    async fn handle_health() -> impl IntoResponse {
        (StatusCode::OK, "Reflex is running")
    }

    // Create shared state
    let state = Arc::new(AppState {
        cache_path: ".".to_string(),
    });

    // Configure CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build the router
    let app = Router::new()
        .route("/query", get(handle_query_endpoint))
        .route("/stats", get(handle_stats_endpoint))
        .route("/index", post(handle_index_endpoint))
        .route("/health", get(handle_health))
        .layer(cors)
        .with_state(state);

    // Bind to the specified address
    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr).await
        .map_err(|e| anyhow::anyhow!("Failed to bind to {}: {}", addr, e))?;

    log::info!("Server listening on {}", addr);

    // Run the server
    axum::serve(listener, app)
        .await
        .map_err(|e| anyhow::anyhow!("Server error: {}", e))?;

    Ok(())
}

/// Handle the `stats` subcommand
fn handle_stats(as_json: bool, pretty_json: bool) -> Result<()> {
    log::info!("Showing index statistics");

    let cache = CacheManager::new(".");

    if !cache.exists() {
        anyhow::bail!(
            "No index found in current directory.\n\
             \n\
             Run 'rfx index' to build the code search index first.\n\
             This will scan all files in the current directory and create a .reflex/ cache.\n\
             \n\
             Example:\n\
             $ rfx index          # Index current directory\n\
             $ rfx stats          # Show index statistics"
        );
    }

    let stats = cache.stats()?;

    if as_json {
        let json_output = if pretty_json {
            serde_json::to_string_pretty(&stats)?
        } else {
            serde_json::to_string(&stats)?
        };
        println!("{}", json_output);
    } else {
        println!("Reflex Index Statistics");
        println!("=======================");

        // Show git branch info if in git repo, or (None) if not
        let root = std::env::current_dir()?;
        if crate::git::is_git_repo(&root) {
            match crate::git::get_git_state(&root) {
                Ok(git_state) => {
                    let dirty_indicator = if git_state.dirty { " (uncommitted changes)" } else { " (clean)" };
                    println!("Branch:         {}@{}{}",
                             git_state.branch,
                             &git_state.commit[..7],
                             dirty_indicator);

                    // Check if current branch is indexed
                    match cache.get_branch_info(&git_state.branch) {
                        Ok(branch_info) => {
                            if branch_info.commit_sha != git_state.commit {
                                println!("                ⚠️  Index commit mismatch (indexed: {})",
                                         &branch_info.commit_sha[..7]);
                            }
                            if git_state.dirty && !branch_info.is_dirty {
                                println!("                ⚠️  Uncommitted changes not indexed");
                            }
                        }
                        Err(_) => {
                            println!("                ⚠️  Branch not indexed");
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Failed to get git state: {}", e);
                }
            }
        } else {
            // Not a git repository - show (None)
            println!("Branch:         (None)");
        }

        println!("Files indexed:  {}", stats.total_files);
        println!("Index size:     {} bytes", stats.index_size_bytes);
        println!("Last updated:   {}", stats.last_updated);
    }

    Ok(())
}

/// Handle the `clear` subcommand
fn handle_clear(skip_confirm: bool) -> Result<()> {
    let cache = CacheManager::new(".");

    if !cache.exists() {
        println!("No cache to clear.");
        return Ok(());
    }

    if !skip_confirm {
        println!("This will delete the local Reflex cache at: {:?}", cache.path());
        print!("Are you sure? [y/N] ");
        use std::io::{self, Write};
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled.");
            return Ok(());
        }
    }

    cache.clear()?;
    println!("Cache cleared successfully.");

    Ok(())
}

/// Handle the `list-files` subcommand
fn handle_list_files(as_json: bool, pretty_json: bool) -> Result<()> {
    let cache = CacheManager::new(".");

    if !cache.exists() {
        anyhow::bail!(
            "No index found in current directory.\n\
             \n\
             Run 'rfx index' to build the code search index first.\n\
             This will scan all files in the current directory and create a .reflex/ cache.\n\
             \n\
             Example:\n\
             $ rfx index            # Index current directory\n\
             $ rfx list-files       # List indexed files"
        );
    }

    let files = cache.list_files()?;

    if as_json {
        let json_output = if pretty_json {
            serde_json::to_string_pretty(&files)?
        } else {
            serde_json::to_string(&files)?
        };
        println!("{}", json_output);
    } else if files.is_empty() {
        println!("No files indexed yet.");
    } else {
        println!("Indexed Files ({} total):", files.len());
        println!();
        for file in files {
            println!("  {} ({})",
                     file.path,
                     file.language);
        }
    }

    Ok(())
}

/// Handle the `watch` subcommand
fn handle_watch(path: PathBuf, debounce_ms: u64, quiet: bool) -> Result<()> {
    log::info!("Starting watch mode for {:?}", path);

    // Validate debounce range (5s - 30s)
    if !(5000..=30000).contains(&debounce_ms) {
        anyhow::bail!(
            "Debounce must be between 5000ms (5s) and 30000ms (30s). Got: {}ms",
            debounce_ms
        );
    }

    if !quiet {
        println!("Starting Reflex watch mode...");
        println!("  Directory: {}", path.display());
        println!("  Debounce: {}ms ({}s)", debounce_ms, debounce_ms / 1000);
        println!("  Press Ctrl+C to stop.\n");
    }

    // Setup cache
    let cache = CacheManager::new(&path);

    // Initial index if cache doesn't exist
    if !cache.exists() {
        if !quiet {
            println!("No index found, running initial index...");
        }
        let config = IndexConfig::default();
        let indexer = Indexer::new(cache, config);
        indexer.index(&path, !quiet)?;
        if !quiet {
            println!("Initial index complete. Now watching for changes...\n");
        }
    }

    // Create indexer for watcher
    let cache = CacheManager::new(&path);
    let config = IndexConfig::default();
    let indexer = Indexer::new(cache, config);

    // Start watcher
    let watch_config = crate::watcher::WatchConfig {
        debounce_ms,
        quiet,
    };

    crate::watcher::watch(&path, indexer, watch_config)?;

    Ok(())
}

/// Handle interactive mode (default when no command is given)
fn handle_interactive() -> Result<()> {
    log::info!("Launching interactive mode");
    crate::interactive::run_interactive()
}

/// Handle the `mcp` subcommand
fn handle_mcp() -> Result<()> {
    log::info!("Starting MCP server");
    crate::mcp::run_mcp_server()
}

/// Handle the internal `index-symbols-internal` command
fn handle_index_symbols_internal(cache_dir: PathBuf) -> Result<()> {
    let mut indexer = crate::background_indexer::BackgroundIndexer::new(&cache_dir)?;
    indexer.run()?;
    Ok(())
}

/// Handle the `deps` subcommand
#[allow(clippy::too_many_arguments)]
fn handle_deps(
    file: Option<PathBuf>,
    reverse: bool,
    depth: usize,
    format: String,
    pretty_json: bool,
    only_internal: bool,
    only_external: bool,
    only_stdlib: bool,
    circular: bool,
    hotspots: bool,
    unused: bool,
    islands: bool,
    analyze: bool,
    limit: Option<usize>,
) -> Result<()> {
    use crate::dependency::DependencyIndex;
    use crate::models::ImportType;

    log::info!("Starting deps command");

    let cache = CacheManager::new(".");

    if !cache.exists() {
        anyhow::bail!(
            "No index found in current directory.\n\
             \n\
             Run 'rfx index' to build the code search index first.\n\
             \n\
             Example:\n\
             $ rfx index          # Index current directory\n\
             $ rfx deps <file>    # Analyze dependencies"
        );
    }

    let deps_index = DependencyIndex::new(cache);

    // Determine operation mode
    if analyze {
        // Run full analysis
        return handle_deps_analyze(&deps_index, &format, pretty_json, limit);
    } else if circular {
        return handle_deps_circular(&deps_index, &format, pretty_json);
    } else if hotspots {
        return handle_deps_hotspots(&deps_index, &format, pretty_json, limit);
    } else if unused {
        return handle_deps_unused(&deps_index, &format, pretty_json, limit);
    } else if islands {
        return handle_deps_islands(&deps_index, &format, pretty_json);
    }

    // File-based operations require a file path
    let file_path = file.ok_or_else(|| {
        anyhow::anyhow!(
            "No file specified.\n\
             \n\
             Usage:\n\
             $ rfx deps <file>              # Show dependencies\n\
             $ rfx deps <file> --reverse    # Show dependents\n\
             $ rfx deps --circular          # Find cycles\n\
             $ rfx deps --hotspots          # Most-imported files"
        )
    })?;

    // Convert file path to string
    let file_str = file_path.to_string_lossy().to_string();

    // Get file ID
    let file_id = deps_index.get_file_id_by_path(&file_str)?
        .ok_or_else(|| anyhow::anyhow!("File '{}' not found in index", file_str))?;

    // Filter function based on flags
    let import_filter = move |import_type: &ImportType| -> bool {
        if only_internal && *import_type != ImportType::Internal {
            return false;
        }
        if only_external && *import_type != ImportType::External {
            return false;
        }
        if only_stdlib && *import_type != ImportType::Stdlib {
            return false;
        }
        true
    };

    if reverse {
        // Show dependents (who imports this file)
        let dependents = deps_index.get_dependents(file_id)?;
        let paths = deps_index.get_file_paths(&dependents)?;

        match format.as_str() {
            "json" => {
                let output: Vec<_> = dependents.iter()
                    .filter_map(|id| paths.get(id).map(|path| serde_json::json!({
                        "file_id": id,
                        "path": path,
                    })))
                    .collect();

                let json_str = if pretty_json {
                    serde_json::to_string_pretty(&output)?
                } else {
                    serde_json::to_string(&output)?
                };
                println!("{}", json_str);
                eprintln!("Found {} files that import {}", dependents.len(), file_str);
            }
            "tree" => {
                println!("Files that import {}:", file_str);
                for (id, path) in &paths {
                    if dependents.contains(id) {
                        println!("  └─ {}", path);
                    }
                }
                eprintln!("\nFound {} dependents", dependents.len());
            }
            "table" => {
                println!("ID     Path");
                println!("-----  ----");
                for id in &dependents {
                    if let Some(path) = paths.get(id) {
                        println!("{:<5}  {}", id, path);
                    }
                }
                eprintln!("\nFound {} dependents", dependents.len());
            }
            _ => {
                anyhow::bail!("Unknown format '{}'. Supported: json, tree, table, dot", format);
            }
        }
    } else {
        // Show dependencies (what this file imports)
        if depth == 1 {
            // Direct dependencies only
            let deps = deps_index.get_dependencies(file_id)?;
            let filtered_deps: Vec<_> = deps.into_iter()
                .filter(|d| import_filter(&d.import_type))
                .collect();

            match format.as_str() {
                "json" => {
                    let output: Vec<_> = filtered_deps.iter()
                        .map(|dep| serde_json::json!({
                            "imported_path": dep.imported_path,
                            "resolved_file_id": dep.resolved_file_id,
                            "import_type": match dep.import_type {
                                ImportType::Internal => "internal",
                                ImportType::External => "external",
                                ImportType::Stdlib => "stdlib",
                            },
                            "line": dep.line_number,
                            "symbols": dep.imported_symbols,
                        }))
                        .collect();

                    let json_str = if pretty_json {
                        serde_json::to_string_pretty(&output)?
                    } else {
                        serde_json::to_string(&output)?
                    };
                    println!("{}", json_str);
                    eprintln!("Found {} dependencies for {}", filtered_deps.len(), file_str);
                }
                "tree" => {
                    println!("Dependencies of {}:", file_str);
                    for dep in &filtered_deps {
                        let type_label = match dep.import_type {
                            ImportType::Internal => "[internal]",
                            ImportType::External => "[external]",
                            ImportType::Stdlib => "[stdlib]",
                        };
                        println!("  └─ {} {} (line {})", dep.imported_path, type_label, dep.line_number);
                    }
                    eprintln!("\nFound {} dependencies", filtered_deps.len());
                }
                "table" => {
                    println!("Path                          Type       Line");
                    println!("----------------------------  ---------  ----");
                    for dep in &filtered_deps {
                        let type_str = match dep.import_type {
                            ImportType::Internal => "internal",
                            ImportType::External => "external",
                            ImportType::Stdlib => "stdlib",
                        };
                        println!("{:<28}  {:<9}  {}", dep.imported_path, type_str, dep.line_number);
                    }
                    eprintln!("\nFound {} dependencies", filtered_deps.len());
                }
                _ => {
                    anyhow::bail!("Unknown format '{}'. Supported: json, tree, table, dot", format);
                }
            }
        } else {
            // Transitive dependencies (depth > 1)
            let transitive = deps_index.get_transitive_deps(file_id, depth)?;
            let file_ids: Vec<_> = transitive.keys().copied().collect();
            let paths = deps_index.get_file_paths(&file_ids)?;

            match format.as_str() {
                "json" => {
                    let output: Vec<_> = transitive.iter()
                        .filter_map(|(id, d)| {
                            paths.get(id).map(|path| serde_json::json!({
                                "file_id": id,
                                "path": path,
                                "depth": d,
                            }))
                        })
                        .collect();

                    let json_str = if pretty_json {
                        serde_json::to_string_pretty(&output)?
                    } else {
                        serde_json::to_string(&output)?
                    };
                    println!("{}", json_str);
                    eprintln!("Found {} transitive dependencies (depth {})", transitive.len(), depth);
                }
                "tree" => {
                    println!("Transitive dependencies of {} (depth {}):", file_str, depth);
                    // Group by depth for tree display
                    let mut by_depth: std::collections::HashMap<usize, Vec<i64>> = std::collections::HashMap::new();
                    for (id, d) in &transitive {
                        by_depth.entry(*d).or_insert_with(Vec::new).push(*id);
                    }

                    for depth_level in 0..=depth {
                        if let Some(ids) = by_depth.get(&depth_level) {
                            let indent = "  ".repeat(depth_level);
                            for id in ids {
                                if let Some(path) = paths.get(id) {
                                    if depth_level == 0 {
                                        println!("{}{} (self)", indent, path);
                                    } else {
                                        println!("{}└─ {}", indent, path);
                                    }
                                }
                            }
                        }
                    }
                    eprintln!("\nFound {} transitive dependencies", transitive.len());
                }
                "table" => {
                    println!("Depth  File ID  Path");
                    println!("-----  -------  ----");
                    let mut sorted: Vec<_> = transitive.iter().collect();
                    sorted.sort_by_key(|(_, d)| *d);
                    for (id, d) in sorted {
                        if let Some(path) = paths.get(id) {
                            println!("{:<5}  {:<7}  {}", d, id, path);
                        }
                    }
                    eprintln!("\nFound {} transitive dependencies", transitive.len());
                }
                _ => {
                    anyhow::bail!("Unknown format '{}'. Supported: json, tree, table, dot", format);
                }
            }
        }
    }

    Ok(())
}

/// Handle --circular flag (detect cycles)
fn handle_deps_circular(
    deps_index: &crate::dependency::DependencyIndex,
    format: &str,
    pretty_json: bool,
) -> Result<()> {
    let cycles = deps_index.detect_circular_dependencies()?;

    if cycles.is_empty() {
        println!("No circular dependencies found.");
        return Ok(());
    }

    match format {
        "json" => {
            let file_ids: Vec<i64> = cycles.iter().flat_map(|c| c.iter()).copied().collect();
            let paths = deps_index.get_file_paths(&file_ids)?;

            let output: Vec<_> = cycles.iter()
                .map(|cycle| {
                    let cycle_paths: Vec<_> = cycle.iter()
                        .filter_map(|id| paths.get(id).cloned())
                        .collect();
                    serde_json::json!({
                        "file_ids": cycle,
                        "paths": cycle_paths,
                    })
                })
                .collect();

            let json_str = if pretty_json {
                serde_json::to_string_pretty(&output)?
            } else {
                serde_json::to_string(&output)?
            };
            println!("{}", json_str);
            eprintln!("Found {} circular dependencies", cycles.len());
        }
        "tree" => {
            println!("Circular Dependencies Found:");
            let file_ids: Vec<i64> = cycles.iter().flat_map(|c| c.iter()).copied().collect();
            let paths = deps_index.get_file_paths(&file_ids)?;

            for (idx, cycle) in cycles.iter().enumerate() {
                println!("\nCycle {}:", idx + 1);
                for id in cycle {
                    if let Some(path) = paths.get(id) {
                        println!("  → {}", path);
                    }
                }
                // Show cycle completion
                if let Some(first_id) = cycle.first() {
                    if let Some(path) = paths.get(first_id) {
                        println!("  → {} (cycle completes)", path);
                    }
                }
            }
            eprintln!("\nFound {} cycles", cycles.len());
        }
        "table" => {
            println!("Cycle  Files in Cycle");
            println!("-----  --------------");
            let file_ids: Vec<i64> = cycles.iter().flat_map(|c| c.iter()).copied().collect();
            let paths = deps_index.get_file_paths(&file_ids)?;

            for (idx, cycle) in cycles.iter().enumerate() {
                let cycle_str = cycle.iter()
                    .filter_map(|id| paths.get(id).map(|p| p.as_str()))
                    .collect::<Vec<_>>()
                    .join(" → ");
                println!("{:<5}  {}", idx + 1, cycle_str);
            }
            eprintln!("\nFound {} cycles", cycles.len());
        }
        _ => {
            anyhow::bail!("Unknown format '{}'. Supported: json, tree, table", format);
        }
    }

    Ok(())
}

/// Handle --hotspots flag (most-imported files)
fn handle_deps_hotspots(
    deps_index: &crate::dependency::DependencyIndex,
    format: &str,
    pretty_json: bool,
    limit: Option<usize>,
) -> Result<()> {
    let hotspots = deps_index.find_hotspots(limit)?;

    if hotspots.is_empty() {
        println!("No hotspots found.");
        return Ok(());
    }

    let file_ids: Vec<i64> = hotspots.iter().map(|(id, _)| *id).collect();
    let paths = deps_index.get_file_paths(&file_ids)?;

    match format {
        "json" => {
            let output: Vec<_> = hotspots.iter()
                .filter_map(|(id, count)| {
                    paths.get(id).map(|path| serde_json::json!({
                        "file_id": id,
                        "path": path,
                        "import_count": count,
                    }))
                })
                .collect();

            let json_str = if pretty_json {
                serde_json::to_string_pretty(&output)?
            } else {
                serde_json::to_string(&output)?
            };
            println!("{}", json_str);
            eprintln!("Found {} hotspots", hotspots.len());
        }
        "tree" => {
            println!("Hotspots (Most-Imported Files):");
            for (idx, (id, count)) in hotspots.iter().enumerate() {
                if let Some(path) = paths.get(id) {
                    println!("  {}. {} ({} imports)", idx + 1, path, count);
                }
            }
            eprintln!("\nFound {} hotspots", hotspots.len());
        }
        "table" => {
            println!("Rank  Imports  File");
            println!("----  -------  ----");
            for (idx, (id, count)) in hotspots.iter().enumerate() {
                if let Some(path) = paths.get(id) {
                    println!("{:<4}  {:<7}  {}", idx + 1, count, path);
                }
            }
            eprintln!("\nFound {} hotspots", hotspots.len());
        }
        _ => {
            anyhow::bail!("Unknown format '{}'. Supported: json, tree, table", format);
        }
    }

    Ok(())
}

/// Handle --unused flag (orphaned files)
fn handle_deps_unused(
    deps_index: &crate::dependency::DependencyIndex,
    format: &str,
    pretty_json: bool,
    limit: Option<usize>,
) -> Result<()> {
    let mut unused = deps_index.find_unused_files()?;

    // Apply limit if specified
    if let Some(lim) = limit {
        unused.truncate(lim);
    }

    if unused.is_empty() {
        println!("No unused files found (all files have incoming dependencies).");
        return Ok(());
    }

    let paths = deps_index.get_file_paths(&unused)?;

    match format {
        "json" => {
            let output: Vec<_> = unused.iter()
                .filter_map(|id| {
                    paths.get(id).map(|path| serde_json::json!({
                        "file_id": id,
                        "path": path,
                    }))
                })
                .collect();

            let json_str = if pretty_json {
                serde_json::to_string_pretty(&output)?
            } else {
                serde_json::to_string(&output)?
            };
            println!("{}", json_str);
            eprintln!("Found {} unused files", unused.len());
        }
        "tree" => {
            println!("Unused Files (No Incoming Dependencies):");
            for (idx, id) in unused.iter().enumerate() {
                if let Some(path) = paths.get(id) {
                    println!("  {}. {}", idx + 1, path);
                }
            }
            eprintln!("\nFound {} unused files", unused.len());
        }
        "table" => {
            println!("File ID  Path");
            println!("-------  ----");
            for id in &unused {
                if let Some(path) = paths.get(id) {
                    println!("{:<7}  {}", id, path);
                }
            }
            eprintln!("\nFound {} unused files", unused.len());
        }
        _ => {
            anyhow::bail!("Unknown format '{}'. Supported: json, tree, table", format);
        }
    }

    Ok(())
}

/// Handle --islands flag (disconnected components)
fn handle_deps_islands(
    deps_index: &crate::dependency::DependencyIndex,
    format: &str,
    pretty_json: bool,
) -> Result<()> {
    let islands = deps_index.find_islands()?;

    if islands.is_empty() {
        println!("No islands found.");
        return Ok(());
    }

    // Get all file IDs from all islands
    let file_ids: Vec<i64> = islands.iter().flat_map(|island| island.iter()).copied().collect();
    let paths = deps_index.get_file_paths(&file_ids)?;

    match format {
        "json" => {
            let output: Vec<_> = islands.iter()
                .enumerate()
                .map(|(idx, island)| {
                    let island_paths: Vec<_> = island.iter()
                        .filter_map(|id| paths.get(id).cloned())
                        .collect();
                    serde_json::json!({
                        "island_id": idx + 1,
                        "size": island.len(),
                        "file_ids": island,
                        "paths": island_paths,
                    })
                })
                .collect();

            let json_str = if pretty_json {
                serde_json::to_string_pretty(&output)?
            } else {
                serde_json::to_string(&output)?
            };
            println!("{}", json_str);
            eprintln!("Found {} islands (disconnected components)", islands.len());
        }
        "tree" => {
            println!("Islands (Disconnected Components):");
            for (idx, island) in islands.iter().enumerate() {
                println!("\nIsland {} ({} files):", idx + 1, island.len());
                for id in island {
                    if let Some(path) = paths.get(id) {
                        println!("  ├─ {}", path);
                    }
                }
            }
            eprintln!("\nFound {} islands", islands.len());
        }
        "table" => {
            println!("Island  Size  Files");
            println!("------  ----  -----");
            for (idx, island) in islands.iter().enumerate() {
                let island_files = island.iter()
                    .filter_map(|id| paths.get(id).map(|p| p.as_str()))
                    .collect::<Vec<_>>()
                    .join(", ");
                println!("{:<6}  {:<4}  {}", idx + 1, island.len(), island_files);
            }
            eprintln!("\nFound {} islands", islands.len());
        }
        _ => {
            anyhow::bail!("Unknown format '{}'. Supported: json, tree, table", format);
        }
    }

    Ok(())
}

/// Handle --analyze flag (full report)
fn handle_deps_analyze(
    deps_index: &crate::dependency::DependencyIndex,
    format: &str,
    pretty_json: bool,
    limit: Option<usize>,
) -> Result<()> {
    println!("Running comprehensive dependency analysis...\n");

    // Run all analyses
    println!("1. Circular Dependencies:");
    handle_deps_circular(deps_index, format, pretty_json)?;

    println!("\n2. Hotspots (Most-Imported Files):");
    handle_deps_hotspots(deps_index, format, pretty_json, limit)?;

    println!("\n3. Unused Files:");
    handle_deps_unused(deps_index, format, pretty_json, limit)?;

    println!("\nAnalysis complete!");
    Ok(())
}
