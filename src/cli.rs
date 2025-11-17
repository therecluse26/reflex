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
pub enum IndexSubcommand {
    /// Show background symbol indexing status
    Status,

    /// Compact the cache by removing deleted files
    ///
    /// Removes files from the cache that no longer exist on disk and reclaims
    /// disk space using SQLite VACUUM. This operation is also performed automatically
    /// in the background every 24 hours during normal usage.
    ///
    /// Examples:
    ///   rfx index compact                # Show compaction results
    ///   rfx index compact --json         # JSON output
    Compact {
        /// Output format as JSON
        #[arg(long)]
        json: bool,

        /// Pretty-print JSON output (only with --json)
        #[arg(long)]
        pretty: bool,
    },
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

        /// Subcommand (status, compact)
        #[command(subcommand)]
        command: Option<IndexSubcommand>,
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

    /// Analyze codebase structure and dependencies
    ///
    /// Perform graph-wide dependency analysis to understand code architecture.
    /// By default, shows a summary report with counts. Use specific flags for
    /// detailed results.
    ///
    /// Examples:
    ///   rfx analyze                                # Summary report
    ///   rfx analyze --circular                     # Find cycles
    ///   rfx analyze --hotspots                     # Most-imported files
    ///   rfx analyze --hotspots --min-dependents 5  # Filter by minimum
    ///   rfx analyze --unused                       # Orphaned files
    ///   rfx analyze --islands                      # Disconnected components
    ///   rfx analyze --hotspots --count             # Just show count
    ///   rfx analyze --circular --glob "src/**"     # Limit to src/
    Analyze {
        /// Show circular dependencies
        #[arg(long)]
        circular: bool,

        /// Show most-imported files (hotspots)
        #[arg(long)]
        hotspots: bool,

        /// Minimum number of dependents for hotspots (default: 2)
        #[arg(long, default_value = "2", requires = "hotspots")]
        min_dependents: usize,

        /// Show unused/orphaned files
        #[arg(long)]
        unused: bool,

        /// Show disconnected components (islands)
        #[arg(long)]
        islands: bool,

        /// Minimum island size (default: 2)
        #[arg(long, default_value = "2", requires = "islands")]
        min_island_size: usize,

        /// Maximum island size (default: 500 or 50% of total files)
        #[arg(long, requires = "islands")]
        max_island_size: Option<usize>,

        /// Output format: tree (default), table, dot
        #[arg(short = 'f', long, default_value = "tree")]
        format: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Pretty-print JSON output
        #[arg(long)]
        pretty: bool,

        /// Only show count and timing, not the actual results
        #[arg(short, long)]
        count: bool,

        /// Return all results (no limit)
        /// Equivalent to --limit 0, convenience flag for unlimited results
        #[arg(short = 'a', long)]
        all: bool,

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

        /// Force execution of potentially expensive queries
        /// Bypasses broad query detection
        #[arg(long)]
        force: bool,

        /// Maximum number of results
        #[arg(short = 'n', long)]
        limit: Option<usize>,

        /// Pagination offset
        #[arg(short = 'o', long)]
        offset: Option<usize>,

        /// Sort order for results: asc (ascending) or desc (descending)
        /// Applies to --hotspots (by import_count), --islands (by size), --circular (by cycle length)
        /// Default: desc (most important first)
        #[arg(long)]
        sort: Option<String>,
    },

    /// Analyze dependencies for a specific file
    ///
    /// Show dependencies and dependents for a single file.
    /// For graph-wide analysis, use 'rfx analyze' instead.
    ///
    /// Examples:
    ///   rfx deps src/main.rs                  # Show dependencies
    ///   rfx deps src/config.rs --reverse      # Show dependents
    ///   rfx deps src/api.rs --depth 3         # Transitive deps
    Deps {
        /// File path to analyze
        file: PathBuf,

        /// Show files that depend on this file (reverse lookup)
        #[arg(short, long)]
        reverse: bool,

        /// Traversal depth for transitive dependencies (default: 1)
        #[arg(short, long, default_value = "1")]
        depth: usize,

        /// Output format: tree (default), table, dot
        #[arg(short = 'f', long, default_value = "tree")]
        format: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Pretty-print JSON output
        #[arg(long)]
        pretty: bool,
    },

    /// Ask a natural language question and generate search queries
    ///
    /// Uses an LLM to translate natural language questions into `rfx query` commands.
    /// Requires API key configuration for one of: OpenAI, Anthropic, Gemini, or Groq.
    ///
    /// Configuration:
    ///   1. Run interactive setup wizard (recommended):
    ///      rfx ask --configure
    ///
    ///   2. OR set API key via environment variable:
    ///      - OPENAI_API_KEY, ANTHROPIC_API_KEY, GEMINI_API_KEY, or GROQ_API_KEY
    ///
    ///   3. Optional: Configure provider in .reflex/config.toml:
    ///      [semantic]
    ///      provider = "groq"  # or openai, anthropic, gemini
    ///      model = "llama-3.3-70b-versatile"  # optional, defaults to provider default
    ///
    /// Examples:
    ///   rfx ask --configure                           # Interactive setup wizard
    ///   rfx ask "Find all TODOs in Rust files"
    ///   rfx ask "Where is the main function defined?" --execute
    ///   rfx ask "Show me error handling code" --provider groq
    Ask {
        /// Natural language question
        question: Option<String>,

        /// Execute queries immediately without confirmation
        #[arg(short, long)]
        execute: bool,

        /// Override configured LLM provider (openai, anthropic, gemini, groq)
        #[arg(short, long)]
        provider: Option<String>,

        /// Output format as JSON
        #[arg(long)]
        json: bool,

        /// Pretty-print JSON output (only with --json)
        #[arg(long)]
        pretty: bool,

        /// Launch interactive configuration wizard to set up AI provider and API key
        #[arg(long)]
        configure: bool,
    },

    /// Internal command: Run background symbol indexing (hidden from help)
    #[command(hide = true)]
    IndexSymbolsInternal {
        /// Cache directory path
        cache_dir: PathBuf,
    },
}

/// Try to run background cache compaction if needed
///
/// Checks if 24+ hours have passed since last compaction.
/// If yes, spawns a non-blocking background thread to compact the cache.
/// Main command continues immediately without waiting for compaction.
///
/// Compaction is skipped for commands that don't need it:
/// - Clear (will delete the cache anyway)
/// - Mcp (long-running server process)
/// - Watch (long-running watcher process)
/// - Serve (long-running HTTP server)
fn try_background_compact(cache: &CacheManager, command: &Command) {
    // Skip compaction for certain commands
    match command {
        Command::Clear { .. } => {
            log::debug!("Skipping compaction for Clear command");
            return;
        }
        Command::Mcp => {
            log::debug!("Skipping compaction for Mcp command");
            return;
        }
        Command::Watch { .. } => {
            log::debug!("Skipping compaction for Watch command");
            return;
        }
        Command::Serve { .. } => {
            log::debug!("Skipping compaction for Serve command");
            return;
        }
        _ => {}
    }

    // Check if compaction should run
    let should_compact = match cache.should_compact() {
        Ok(true) => true,
        Ok(false) => {
            log::debug!("Compaction not needed yet (last run <24h ago)");
            return;
        }
        Err(e) => {
            log::warn!("Failed to check compaction status: {}", e);
            return;
        }
    };

    if !should_compact {
        return;
    }

    log::info!("Starting background cache compaction...");

    // Clone cache path for background thread
    let cache_path = cache.path().to_path_buf();

    // Spawn background thread for compaction
    std::thread::spawn(move || {
        let cache = CacheManager::new(cache_path.parent().expect("Cache should have parent directory"));

        match cache.compact() {
            Ok(report) => {
                log::info!(
                    "Background compaction completed: {} files removed, {:.2} MB saved, took {}ms",
                    report.files_removed,
                    report.space_saved_bytes as f64 / 1_048_576.0,
                    report.duration_ms
                );
            }
            Err(e) => {
                log::warn!("Background compaction failed: {}", e);
            }
        }
    });

    log::debug!("Background compaction thread spawned - main command continuing");
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

        // Try background compaction (non-blocking) before command execution
        if let Some(ref command) = self.command {
            // Use current directory as default cache location
            let cache = CacheManager::new(".");
            try_background_compact(&cache, command);
        }

        // Execute the subcommand, or launch interactive mode if no command provided
        match self.command {
            None => {
                // No subcommand: launch interactive mode
                handle_interactive()
            }
            Some(Command::Index { path, force, languages, quiet, command }) => {
                match command {
                    None => {
                        // Default: run index build
                        handle_index_build(&path, &force, &languages, &quiet)
                    }
                    Some(IndexSubcommand::Status) => {
                        handle_index_status()
                    }
                    Some(IndexSubcommand::Compact { json, pretty }) => {
                        handle_index_compact(&json, &pretty)
                    }
                }
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
            Some(Command::Analyze { circular, hotspots, min_dependents, unused, islands, min_island_size, max_island_size, format, json, pretty, count, all, plain, glob, exclude, force, limit, offset, sort }) => {
                handle_analyze(circular, hotspots, min_dependents, unused, islands, min_island_size, max_island_size, format, json, pretty, count, all, plain, glob, exclude, force, limit, offset, sort)
            }
            Some(Command::Deps { file, reverse, depth, format, json, pretty }) => {
                handle_deps(file, reverse, depth, format, json, pretty)
            }
            Some(Command::Ask { question, execute, provider, json, pretty, configure }) => {
                handle_ask(question, execute, provider, json, pretty, configure)
            }
            Some(Command::IndexSymbolsInternal { cache_dir }) => {
                handle_index_symbols_internal(cache_dir)
            }
        }
    }
}

/// Handle the `index status` subcommand
fn handle_index_status() -> Result<()> {
    log::info!("Checking background symbol indexing status");

    let cache = CacheManager::new(".");
    let cache_path = cache.path().to_path_buf();

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

/// Handle the `index compact` subcommand
fn handle_index_compact(json: &bool, pretty: &bool) -> Result<()> {
    log::info!("Running cache compaction");

    let cache = CacheManager::new(".");
    let report = cache.compact()?;

    // Output results in requested format
    if *json {
        let json_str = if *pretty {
            serde_json::to_string_pretty(&report)?
        } else {
            serde_json::to_string(&report)?
        };
        println!("{}", json_str);
    } else {
        println!("Cache Compaction Complete");
        println!("=========================");
        println!("Files removed:    {}", report.files_removed);
        println!("Space saved:      {:.2} MB", report.space_saved_bytes as f64 / 1_048_576.0);
        println!("Duration:         {}ms", report.duration_ms);
    }

    Ok(())
}

fn handle_index_build(path: &PathBuf, force: &bool, languages: &[String], quiet: &bool) -> Result<()> {
    log::info!("Starting index build");

    let cache = CacheManager::new(path);
    let cache_path = cache.path().to_path_buf();

    if *force {
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
            println!("  Check status with: rfx index status");
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
        println!("  Check status with: rfx index status");
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
    let (query_response, mut flat_results, total_results, has_more) = if use_ast {
        // AST query: pattern is the S-expression, scan all files
        match engine.search_ast_all_files(&pattern, filter.clone()) {
            Ok(ast_results) => {
                let count = ast_results.len();
                (None, ast_results, count, false)
            }
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
                    return Err(e);
                }
            }
        }
    } else {
        // Use metadata-aware search for all queries (to get pagination info)
        match engine.search_with_metadata(&pattern, filter.clone()) {
            Ok(response) => {
                let total = response.pagination.total;
                let has_more = response.pagination.has_more;

                // Flatten grouped results to SearchResult vec for plain text formatting
                let flat = response.results.iter()
                    .flat_map(|file_group| {
                        file_group.matches.iter().map(move |m| {
                            crate::models::SearchResult {
                                path: file_group.path.clone(),
                                lang: crate::models::Language::Unknown, // Will be set by formatter if needed
                                kind: m.kind.clone(),
                                symbol: m.symbol.clone(),
                                span: m.span.clone(),
                                preview: m.preview.clone(),
                                dependencies: file_group.dependencies.clone(),
                            }
                        })
                    })
                    .collect();

                (Some(response), flat, total, has_more)
            }
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
                    return Err(e);
                }
            }
        }
    };

    // Apply preview truncation unless --no-truncate is set
    if !no_truncate {
        const MAX_PREVIEW_LENGTH: usize = 100;
        for result in &mut flat_results {
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
            let locations: Vec<serde_json::Value> = flat_results.iter()
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
            let mut response = if let Some(resp) = query_response {
                // We already have a response from search_with_metadata
                // Apply truncation to the response (the flat_results were already truncated)
                let mut resp = resp;

                // Apply truncation to results
                if !no_truncate {
                    const MAX_PREVIEW_LENGTH: usize = 100;
                    for file_group in resp.results.iter_mut() {
                        for m in file_group.matches.iter_mut() {
                            m.preview = truncate_preview(&m.preview, MAX_PREVIEW_LENGTH);
                        }
                    }
                }

                resp
            } else {
                // For AST queries, build a response with minimal metadata
                // Group flat results by file path
                use crate::models::{PaginationInfo, IndexStatus, FileGroupedResult, MatchResult};
                use std::collections::HashMap;

                let mut grouped: HashMap<String, Vec<crate::models::SearchResult>> = HashMap::new();
                for result in &flat_results {
                    grouped
                        .entry(result.path.clone())
                        .or_default()
                        .push(result.clone());
                }

                let mut file_results: Vec<FileGroupedResult> = grouped
                    .into_iter()
                    .map(|(path, file_matches)| {
                        let matches: Vec<MatchResult> = file_matches
                            .into_iter()
                            .map(|r| MatchResult {
                                kind: r.kind,
                                symbol: r.symbol,
                                span: r.span,
                                preview: r.preview,
                            })
                            .collect();
                        FileGroupedResult {
                            path,
                            dependencies: None,
                            matches,
                        }
                    })
                    .collect();

                // Sort by path for deterministic output
                file_results.sort_by(|a, b| a.path.cmp(&b.path));

                crate::models::QueryResponse {
                    ai_instruction: None,  // Will be populated below if ai_mode is true
                    status: IndexStatus::Fresh,
                    can_trust_results: true,
                    warning: None,
                    pagination: PaginationInfo {
                        total: flat_results.len(),
                        count: flat_results.len(),
                        offset: offset.unwrap_or(0),
                        limit,
                        has_more: false, // AST already applied pagination
                    },
                    results: file_results,
                }
            };

            // Generate AI instruction if in AI mode
            if ai_mode {
                let result_count: usize = response.results.iter().map(|fg| fg.matches.len()).sum();

                response.ai_instruction = crate::query::generate_ai_instruction(
                    result_count,
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

            let result_count: usize = response.results.iter().map(|fg| fg.matches.len()).sum();
            eprintln!("Found {} results in {}", result_count, timing_str);
        }
    } else {
        // Standard output with formatting
        if count_only {
            println!("Found {} results in {}", flat_results.len(), timing_str);
            return Ok(());
        }

        if paths_only {
            // Paths-only plain text mode: output one path per line
            if flat_results.is_empty() {
                eprintln!("No results found (searched in {}).", timing_str);
            } else {
                for result in &flat_results {
                    println!("{}", result.path);
                }
                eprintln!("Found {} unique files in {}", flat_results.len(), timing_str);
            }
        } else {
            // Standard result formatting
            if flat_results.is_empty() {
                println!("No results found (searched in {}).", timing_str);
            } else {
                // Use formatter for pretty output
                let formatter = crate::formatter::OutputFormatter::new(plain);
                formatter.format_results(&flat_results, &pattern)?;

                // Print summary at the bottom with pagination details
                if total_results > flat_results.len() {
                    // Results were paginated - show detailed count
                    println!("\nFound {} results ({} total) in {}", flat_results.len(), total_results, timing_str);
                    // Show pagination hint if there are more results available
                    if has_more {
                        println!("Use --limit and --offset to paginate");
                    }
                } else {
                    // All results shown - simple count
                    println!("\nFound {} results in {}", flat_results.len(), timing_str);
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

/// Handle the `analyze` subcommand
#[allow(clippy::too_many_arguments)]
fn handle_analyze(
    circular: bool,
    hotspots: bool,
    min_dependents: usize,
    unused: bool,
    islands: bool,
    min_island_size: usize,
    max_island_size: Option<usize>,
    format: String,
    as_json: bool,
    pretty_json: bool,
    count_only: bool,
    all: bool,
    plain: bool,
    _glob_patterns: Vec<String>,
    _exclude_patterns: Vec<String>,
    _force: bool,
    limit: Option<usize>,
    offset: Option<usize>,
    sort: Option<String>,
) -> Result<()> {
    use crate::dependency::DependencyIndex;

    log::info!("Starting analyze command");

    let cache = CacheManager::new(".");

    if !cache.exists() {
        anyhow::bail!(
            "No index found in current directory.\n\
             \n\
             Run 'rfx index' to build the code search index first.\n\
             \n\
             Example:\n\
             $ rfx index             # Index current directory\n\
             $ rfx analyze           # Run dependency analysis"
        );
    }

    let deps_index = DependencyIndex::new(cache);

    // JSON mode overrides format
    let format = if as_json { "json" } else { &format };

    // Smart limit handling for analyze commands (default: 200 per page)
    let final_limit = if all {
        None  // --all means no limit
    } else if let Some(user_limit) = limit {
        Some(user_limit)  // Use user-specified limit
    } else {
        Some(200)  // Default: limit to 200 results per page for token efficiency
    };

    // If no specific flags, show summary
    if !circular && !hotspots && !unused && !islands {
        return handle_analyze_summary(&deps_index, min_dependents, count_only, as_json, pretty_json);
    }

    // Run specific analyses based on flags
    if circular {
        handle_deps_circular(&deps_index, format, pretty_json, final_limit, offset, count_only, plain, sort.clone())?;
    }

    if hotspots {
        handle_deps_hotspots(&deps_index, format, pretty_json, final_limit, offset, min_dependents, count_only, plain, sort.clone())?;
    }

    if unused {
        handle_deps_unused(&deps_index, format, pretty_json, final_limit, offset, count_only, plain)?;
    }

    if islands {
        handle_deps_islands(&deps_index, format, pretty_json, final_limit, offset, min_island_size, max_island_size, count_only, plain, sort.clone())?;
    }

    Ok(())
}

/// Handle analyze summary (default --analyze behavior)
fn handle_analyze_summary(
    deps_index: &crate::dependency::DependencyIndex,
    min_dependents: usize,
    count_only: bool,
    as_json: bool,
    pretty_json: bool,
) -> Result<()> {
    // Gather counts
    let cycles = deps_index.detect_circular_dependencies()?;
    let hotspots = deps_index.find_hotspots(None, min_dependents)?;
    let unused = deps_index.find_unused_files()?;
    let all_islands = deps_index.find_islands()?;

    if as_json {
        // JSON output
        let summary = serde_json::json!({
            "circular_dependencies": cycles.len(),
            "hotspots": hotspots.len(),
            "unused_files": unused.len(),
            "islands": all_islands.len(),
            "min_dependents": min_dependents,
        });

        let json_str = if pretty_json {
            serde_json::to_string_pretty(&summary)?
        } else {
            serde_json::to_string(&summary)?
        };
        println!("{}", json_str);
    } else if count_only {
        // Just show counts without any extra formatting
        println!("{} circular dependencies", cycles.len());
        println!("{} hotspots ({}+ dependents)", hotspots.len(), min_dependents);
        println!("{} unused files", unused.len());
        println!("{} islands", all_islands.len());
    } else {
        // Full summary with headers and suggestions
        println!("Dependency Analysis Summary\n");

        // Circular dependencies
        println!("Circular Dependencies: {} cycle(s)", cycles.len());

        // Hotspots
        println!("Hotspots: {} file(s) with {}+ dependents", hotspots.len(), min_dependents);

        // Unused
        println!("Unused Files: {} file(s)", unused.len());

        // Islands
        println!("Islands: {} disconnected component(s)", all_islands.len());

        println!("\nUse specific flags for detailed results:");
        println!("  rfx analyze --circular");
        println!("  rfx analyze --hotspots");
        println!("  rfx analyze --unused");
        println!("  rfx analyze --islands");
    }

    Ok(())
}

/// Handle the `deps` subcommand
fn handle_deps(
    file: PathBuf,
    reverse: bool,
    depth: usize,
    format: String,
    as_json: bool,
    pretty_json: bool,
) -> Result<()> {
    use crate::dependency::DependencyIndex;

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

    // JSON mode overrides format
    let format = if as_json { "json" } else { &format };

    // Convert file path to string
    let file_str = file.to_string_lossy().to_string();

    // Get file ID
    let file_id = deps_index.get_file_id_by_path(&file_str)?
        .ok_or_else(|| anyhow::anyhow!("File '{}' not found in index", file_str))?;

    if reverse {
        // Show dependents (who imports this file)
        let dependents = deps_index.get_dependents(file_id)?;
        let paths = deps_index.get_file_paths(&dependents)?;

        match format.as_ref() {
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

            match format.as_ref() {
                "json" => {
                    let output: Vec<_> = deps.iter()
                        .map(|dep| serde_json::json!({
                            "imported_path": dep.imported_path,
                            "resolved_file_id": dep.resolved_file_id,
                            "import_type": match dep.import_type {
                                crate::models::ImportType::Internal => "internal",
                                crate::models::ImportType::External => "external",
                                crate::models::ImportType::Stdlib => "stdlib",
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
                    eprintln!("Found {} dependencies for {}", deps.len(), file_str);
                }
                "tree" => {
                    println!("Dependencies of {}:", file_str);
                    for dep in &deps {
                        let type_label = match dep.import_type {
                            crate::models::ImportType::Internal => "[internal]",
                            crate::models::ImportType::External => "[external]",
                            crate::models::ImportType::Stdlib => "[stdlib]",
                        };
                        println!("  └─ {} {} (line {})", dep.imported_path, type_label, dep.line_number);
                    }
                    eprintln!("\nFound {} dependencies", deps.len());
                }
                "table" => {
                    println!("Path                          Type       Line");
                    println!("----------------------------  ---------  ----");
                    for dep in &deps {
                        let type_str = match dep.import_type {
                            crate::models::ImportType::Internal => "internal",
                            crate::models::ImportType::External => "external",
                            crate::models::ImportType::Stdlib => "stdlib",
                        };
                        println!("{:<28}  {:<9}  {}", dep.imported_path, type_str, dep.line_number);
                    }
                    eprintln!("\nFound {} dependencies", deps.len());
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

            match format.as_ref() {
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

/// Handle the `ask` command
fn handle_ask(
    question: Option<String>,
    auto_execute: bool,
    provider_override: Option<String>,
    as_json: bool,
    pretty_json: bool,
    configure: bool,
) -> Result<()> {
    // If --configure flag is set, launch the configuration wizard
    if configure {
        log::info!("Launching configuration wizard");
        return crate::semantic::run_configure_wizard();
    }

    // Otherwise, require a question
    let question = question.ok_or_else(|| {
        anyhow::anyhow!("Question is required unless --configure is used")
    })?;

    log::info!("Starting ask command");

    let cache = CacheManager::new(".");

    if !cache.exists() {
        anyhow::bail!(
            "No index found in current directory.\n\
             \n\
             Run 'rfx index' to build the code search index first.\n\
             \n\
             Example:\n\
             $ rfx index                          # Index current directory\n\
             $ rfx ask \"Find all TODOs\"          # Ask questions"
        );
    }

    // Create a tokio runtime for async operations
    let runtime = tokio::runtime::Runtime::new()
        .context("Failed to create async runtime")?;

    // Call the async function using the runtime
    let response = runtime.block_on(async {
        crate::semantic::ask_question(&question, &cache, provider_override).await
    }).context("Failed to generate semantic queries")?;

    log::info!("LLM generated {} queries", response.queries.len());

    // Output in JSON format if requested
    if as_json {
        let json_str = if pretty_json {
            serde_json::to_string_pretty(&response)?
        } else {
            serde_json::to_string(&response)?
        };
        println!("{}", json_str);
        return Ok(());
    }

    // Display generated queries
    println!("\nGenerated Queries:");
    println!("==================");
    for (idx, query_cmd) in response.queries.iter().enumerate() {
        println!(
            "{}. [order: {}, merge: {}] rfx {}",
            idx + 1,
            query_cmd.order,
            query_cmd.merge,
            query_cmd.command
        );
    }
    println!();

    // If not auto-execute, ask for confirmation
    if !auto_execute {
        use std::io::{self, Write};

        print!("Execute these queries? [y/N]: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let input = input.trim().to_lowercase();
        if input != "y" && input != "yes" {
            println!("Queries not executed.");
            return Ok(());
        }
    }

    // Execute queries
    println!("\nExecuting queries...");
    println!("====================\n");

    let (results, total_count, count_only) = runtime.block_on(async {
        crate::semantic::execute_queries(response.queries, &cache).await
    }).context("Failed to execute queries")?;

    // Display results
    if count_only {
        // Count-only mode: just show the total count (matching direct CLI behavior)
        println!("Found {} results", total_count);
    } else if results.is_empty() {
        println!("No results found.");
    } else {
        println!("Found {} total results across {} files:\n", total_count, results.len());

        for file_group in &results {
            println!("{}:", file_group.path);
            for match_result in &file_group.matches {
                println!(
                    "  Line {}-{}: {}",
                    match_result.span.start_line,
                    match_result.span.end_line,
                    match_result.preview.lines().next().unwrap_or("")
                );
            }
            println!();
        }
    }

    Ok(())
}

/// Handle --circular flag (detect cycles)
fn handle_deps_circular(
    deps_index: &crate::dependency::DependencyIndex,
    format: &str,
    pretty_json: bool,
    limit: Option<usize>,
    offset: Option<usize>,
    count_only: bool,
    _plain: bool,
    sort: Option<String>,
) -> Result<()> {
    let mut all_cycles = deps_index.detect_circular_dependencies()?;

    // Apply sorting (default: descending - longest cycles first)
    let sort_order = sort.as_deref().unwrap_or("desc");
    match sort_order {
        "asc" => {
            // Ascending: shortest cycles first
            all_cycles.sort_by_key(|cycle| cycle.len());
        }
        "desc" => {
            // Descending: longest cycles first (default)
            all_cycles.sort_by_key(|cycle| std::cmp::Reverse(cycle.len()));
        }
        _ => {
            anyhow::bail!("Invalid sort order '{}'. Supported: asc, desc", sort_order);
        }
    }

    let total_count = all_cycles.len();

    if count_only {
        println!("Found {} circular dependencies", total_count);
        return Ok(());
    }

    if all_cycles.is_empty() {
        println!("No circular dependencies found.");
        return Ok(());
    }

    // Apply offset pagination
    let offset_val = offset.unwrap_or(0);
    let mut cycles: Vec<_> = all_cycles.into_iter().skip(offset_val).collect();

    // Apply limit
    if let Some(lim) = limit {
        cycles.truncate(lim);
    }

    if cycles.is_empty() {
        println!("No circular dependencies found at offset {}.", offset_val);
        return Ok(());
    }

    let count = cycles.len();
    let has_more = offset_val + count < total_count;

    match format {
        "json" => {
            let file_ids: Vec<i64> = cycles.iter().flat_map(|c| c.iter()).copied().collect();
            let paths = deps_index.get_file_paths(&file_ids)?;

            let results: Vec<_> = cycles.iter()
                .map(|cycle| {
                    let cycle_paths: Vec<_> = cycle.iter()
                        .filter_map(|id| paths.get(id).cloned())
                        .collect();
                    serde_json::json!({
                        "paths": cycle_paths,
                    })
                })
                .collect();

            let output = serde_json::json!({
                "pagination": {
                    "total": total_count,
                    "count": count,
                    "offset": offset_val,
                    "limit": limit,
                    "has_more": has_more,
                },
                "results": results,
            });

            let json_str = if pretty_json {
                serde_json::to_string_pretty(&output)?
            } else {
                serde_json::to_string(&output)?
            };
            println!("{}", json_str);
            if total_count > count {
                eprintln!("Found {} circular dependencies ({} total)", count, total_count);
            } else {
                eprintln!("Found {} circular dependencies", count);
            }
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
            if total_count > count {
                eprintln!("\nFound {} cycles ({} total)", count, total_count);
                if has_more {
                    eprintln!("Use --limit and --offset to paginate");
                }
            } else {
                eprintln!("\nFound {} cycles", count);
            }
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
            if total_count > count {
                eprintln!("\nFound {} cycles ({} total)", count, total_count);
                if has_more {
                    eprintln!("Use --limit and --offset to paginate");
                }
            } else {
                eprintln!("\nFound {} cycles", count);
            }
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
    offset: Option<usize>,
    min_dependents: usize,
    count_only: bool,
    _plain: bool,
    sort: Option<String>,
) -> Result<()> {
    // Get all hotspots without limit first to track total count
    let mut all_hotspots = deps_index.find_hotspots(None, min_dependents)?;

    // Apply sorting (default: descending - most imports first)
    let sort_order = sort.as_deref().unwrap_or("desc");
    match sort_order {
        "asc" => {
            // Ascending: least imports first
            all_hotspots.sort_by(|a, b| a.1.cmp(&b.1));
        }
        "desc" => {
            // Descending: most imports first (default)
            all_hotspots.sort_by(|a, b| b.1.cmp(&a.1));
        }
        _ => {
            anyhow::bail!("Invalid sort order '{}'. Supported: asc, desc", sort_order);
        }
    }

    let total_count = all_hotspots.len();

    if count_only {
        println!("Found {} hotspots with {}+ dependents", total_count, min_dependents);
        return Ok(());
    }

    if all_hotspots.is_empty() {
        println!("No hotspots found.");
        return Ok(());
    }

    // Apply offset pagination
    let offset_val = offset.unwrap_or(0);
    let mut hotspots: Vec<_> = all_hotspots.into_iter().skip(offset_val).collect();

    // Apply limit
    if let Some(lim) = limit {
        hotspots.truncate(lim);
    }

    if hotspots.is_empty() {
        println!("No hotspots found at offset {}.", offset_val);
        return Ok(());
    }

    let count = hotspots.len();
    let has_more = offset_val + count < total_count;

    let file_ids: Vec<i64> = hotspots.iter().map(|(id, _)| *id).collect();
    let paths = deps_index.get_file_paths(&file_ids)?;

    match format {
        "json" => {
            let results: Vec<_> = hotspots.iter()
                .filter_map(|(id, import_count)| {
                    paths.get(id).map(|path| serde_json::json!({
                        "path": path,
                        "import_count": import_count,
                    }))
                })
                .collect();

            let output = serde_json::json!({
                "pagination": {
                    "total": total_count,
                    "count": count,
                    "offset": offset_val,
                    "limit": limit,
                    "has_more": has_more,
                },
                "results": results,
            });

            let json_str = if pretty_json {
                serde_json::to_string_pretty(&output)?
            } else {
                serde_json::to_string(&output)?
            };
            println!("{}", json_str);
            if total_count > count {
                eprintln!("Found {} hotspots ({} total)", count, total_count);
            } else {
                eprintln!("Found {} hotspots", count);
            }
        }
        "tree" => {
            println!("Hotspots (Most-Imported Files):");
            for (idx, (id, import_count)) in hotspots.iter().enumerate() {
                if let Some(path) = paths.get(id) {
                    println!("  {}. {} ({} imports)", idx + 1, path, import_count);
                }
            }
            if total_count > count {
                eprintln!("\nFound {} hotspots ({} total)", count, total_count);
                if has_more {
                    eprintln!("Use --limit and --offset to paginate");
                }
            } else {
                eprintln!("\nFound {} hotspots", count);
            }
        }
        "table" => {
            println!("Rank  Imports  File");
            println!("----  -------  ----");
            for (idx, (id, import_count)) in hotspots.iter().enumerate() {
                if let Some(path) = paths.get(id) {
                    println!("{:<4}  {:<7}  {}", idx + 1, import_count, path);
                }
            }
            if total_count > count {
                eprintln!("\nFound {} hotspots ({} total)", count, total_count);
                if has_more {
                    eprintln!("Use --limit and --offset to paginate");
                }
            } else {
                eprintln!("\nFound {} hotspots", count);
            }
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
    offset: Option<usize>,
    count_only: bool,
    _plain: bool,
) -> Result<()> {
    let all_unused = deps_index.find_unused_files()?;
    let total_count = all_unused.len();

    if count_only {
        println!("Found {} unused files", total_count);
        return Ok(());
    }

    if all_unused.is_empty() {
        println!("No unused files found (all files have incoming dependencies).");
        return Ok(());
    }

    // Apply offset pagination
    let offset_val = offset.unwrap_or(0);
    let mut unused: Vec<_> = all_unused.into_iter().skip(offset_val).collect();

    if unused.is_empty() {
        println!("No unused files found at offset {}.", offset_val);
        return Ok(());
    }

    // Apply limit
    if let Some(lim) = limit {
        unused.truncate(lim);
    }

    let count = unused.len();
    let has_more = offset_val + count < total_count;

    let paths = deps_index.get_file_paths(&unused)?;

    match format {
        "json" => {
            // Return flat array of path strings (no "path" key wrapper)
            let results: Vec<String> = unused.iter()
                .filter_map(|id| paths.get(id).cloned())
                .collect();

            let output = serde_json::json!({
                "pagination": {
                    "total": total_count,
                    "count": count,
                    "offset": offset_val,
                    "limit": limit,
                    "has_more": has_more,
                },
                "results": results,
            });

            let json_str = if pretty_json {
                serde_json::to_string_pretty(&output)?
            } else {
                serde_json::to_string(&output)?
            };
            println!("{}", json_str);
            if total_count > count {
                eprintln!("Found {} unused files ({} total)", count, total_count);
            } else {
                eprintln!("Found {} unused files", count);
            }
        }
        "tree" => {
            println!("Unused Files (No Incoming Dependencies):");
            for (idx, id) in unused.iter().enumerate() {
                if let Some(path) = paths.get(id) {
                    println!("  {}. {}", idx + 1, path);
                }
            }
            if total_count > count {
                eprintln!("\nFound {} unused files ({} total)", count, total_count);
                if has_more {
                    eprintln!("Use --limit and --offset to paginate");
                }
            } else {
                eprintln!("\nFound {} unused files", count);
            }
        }
        "table" => {
            println!("Path");
            println!("----");
            for id in &unused {
                if let Some(path) = paths.get(id) {
                    println!("{}", path);
                }
            }
            if total_count > count {
                eprintln!("\nFound {} unused files ({} total)", count, total_count);
                if has_more {
                    eprintln!("Use --limit and --offset to paginate");
                }
            } else {
                eprintln!("\nFound {} unused files", count);
            }
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
    limit: Option<usize>,
    offset: Option<usize>,
    min_island_size: usize,
    max_island_size: Option<usize>,
    count_only: bool,
    _plain: bool,
    sort: Option<String>,
) -> Result<()> {
    let mut all_islands = deps_index.find_islands()?;
    let total_components = all_islands.len();

    // Get total file count from the cache for percentage calculation
    let cache = deps_index.get_cache();
    let total_files = cache.stats()?.total_files as usize;

    // Calculate max_island_size default: min of 500 or 50% of total files
    let max_size = max_island_size.unwrap_or_else(|| {
        let fifty_percent = (total_files as f64 * 0.5) as usize;
        fifty_percent.min(500)
    });

    // Filter islands by size
    let mut islands: Vec<_> = all_islands.into_iter()
        .filter(|island| {
            let size = island.len();
            size >= min_island_size && size <= max_size
        })
        .collect();

    // Apply sorting (default: descending - largest islands first)
    let sort_order = sort.as_deref().unwrap_or("desc");
    match sort_order {
        "asc" => {
            // Ascending: smallest islands first
            islands.sort_by_key(|island| island.len());
        }
        "desc" => {
            // Descending: largest islands first (default)
            islands.sort_by_key(|island| std::cmp::Reverse(island.len()));
        }
        _ => {
            anyhow::bail!("Invalid sort order '{}'. Supported: asc, desc", sort_order);
        }
    }

    let filtered_count = total_components - islands.len();

    if count_only {
        if filtered_count > 0 {
            println!("Found {} islands (filtered {} of {} total components by size: {}-{})",
                islands.len(), filtered_count, total_components, min_island_size, max_size);
        } else {
            println!("Found {} islands", islands.len());
        }
        return Ok(());
    }

    // Apply offset pagination first
    let offset_val = offset.unwrap_or(0);
    if offset_val > 0 && offset_val < islands.len() {
        islands = islands.into_iter().skip(offset_val).collect();
    } else if offset_val >= islands.len() {
        if filtered_count > 0 {
            println!("No islands found at offset {} (filtered {} of {} total components by size: {}-{}).",
                offset_val, filtered_count, total_components, min_island_size, max_size);
        } else {
            println!("No islands found at offset {}.", offset_val);
        }
        return Ok(());
    }

    // Apply limit to number of islands
    if let Some(lim) = limit {
        islands.truncate(lim);
    }

    if islands.is_empty() {
        if filtered_count > 0 {
            println!("No islands found matching criteria (filtered {} of {} total components by size: {}-{}).",
                filtered_count, total_components, min_island_size, max_size);
        } else {
            println!("No islands found.");
        }
        return Ok(());
    }

    // Get all file IDs from all islands and track pagination
    let count = islands.len();
    let has_more = offset_val + count < total_components - filtered_count;

    let file_ids: Vec<i64> = islands.iter().flat_map(|island| island.iter()).copied().collect();
    let paths = deps_index.get_file_paths(&file_ids)?;

    match format {
        "json" => {
            let results: Vec<_> = islands.iter()
                .enumerate()
                .map(|(idx, island)| {
                    let island_paths: Vec<_> = island.iter()
                        .filter_map(|id| paths.get(id).cloned())
                        .collect();
                    serde_json::json!({
                        "island_id": idx + 1,
                        "size": island.len(),
                        "paths": island_paths,
                    })
                })
                .collect();

            let output = serde_json::json!({
                "pagination": {
                    "total": total_components - filtered_count,
                    "count": count,
                    "offset": offset_val,
                    "limit": limit,
                    "has_more": has_more,
                },
                "results": results,
            });

            let json_str = if pretty_json {
                serde_json::to_string_pretty(&output)?
            } else {
                serde_json::to_string(&output)?
            };
            println!("{}", json_str);
            if filtered_count > 0 {
                eprintln!("Found {} islands (filtered {} of {} total components by size: {}-{})",
                    count, filtered_count, total_components, min_island_size, max_size);
            } else if total_components - filtered_count > count {
                eprintln!("Found {} islands ({} total)", count, total_components - filtered_count);
            } else {
                eprintln!("Found {} islands (disconnected components)", count);
            }
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
            if filtered_count > 0 {
                eprintln!("\nFound {} islands (filtered {} of {} total components by size: {}-{})",
                    count, filtered_count, total_components, min_island_size, max_size);
                if has_more {
                    eprintln!("Use --limit and --offset to paginate");
                }
            } else if total_components - filtered_count > count {
                eprintln!("\nFound {} islands ({} total)", count, total_components - filtered_count);
                if has_more {
                    eprintln!("Use --limit and --offset to paginate");
                }
            } else {
                eprintln!("\nFound {} islands", count);
            }
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
            if filtered_count > 0 {
                eprintln!("\nFound {} islands (filtered {} of {} total components by size: {}-{})",
                    count, filtered_count, total_components, min_island_size, max_size);
                if has_more {
                    eprintln!("Use --limit and --offset to paginate");
                }
            } else if total_components - filtered_count > count {
                eprintln!("\nFound {} islands ({} total)", count, total_components - filtered_count);
                if has_more {
                    eprintln!("Use --limit and --offset to paginate");
                }
            } else {
                eprintln!("\nFound {} islands", count);
            }
        }
        _ => {
            anyhow::bail!("Unknown format '{}'. Supported: json, tree, table", format);
        }
    }

    Ok(())
}

