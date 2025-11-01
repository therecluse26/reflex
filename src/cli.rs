//! CLI argument parsing and command handlers

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::time::Instant;

use crate::cache::CacheManager;
use crate::indexer::Indexer;
use crate::models::{IndexConfig, Language};
use crate::query::{QueryEngine, QueryFilter};

/// RefLex: Local-first, structure-aware code search for AI agents
#[derive(Parser, Debug)]
#[command(
    name = "reflex",
    version,
    about = "A fast, deterministic code search engine built for AI",
    long_about = "RefLex is a local-first, structure-aware code search engine that returns \
                  structured results (symbols, spans, scopes) with sub-100ms latency. \
                  Designed for AI coding agents and automation."
)]
pub struct Cli {
    /// Enable verbose logging (can be repeated for more verbosity)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Command,
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

        /// Show progress bar during indexing
        #[arg(short, long)]
        progress: bool,
    },

    /// Query the code index
    ///
    /// Search modes:
    ///   - Default: Full-text trigram search (finds all occurrences)
    ///     Example: reflex query "extract_symbols"
    ///
    ///   - Symbol search: Search symbol definitions only
    ///     Example: reflex query "parse" --symbols
    ///     Example: reflex query "parse" --kind function  (implies --symbols)
    Query {
        /// Search pattern
        pattern: String,

        /// Search symbol definitions only (functions, classes, etc.)
        #[arg(short, long)]
        symbols: bool,

        /// Filter by language
        /// Supported: rust, javascript (js), typescript (ts), vue, svelte
        #[arg(short, long)]
        lang: Option<String>,

        /// Filter by symbol kind (implies --symbols)
        /// Supported: function, class, struct, enum, trait, etc.
        #[arg(short, long)]
        kind: Option<String>,

        /// Use AST pattern matching
        #[arg(long)]
        ast: bool,

        /// Output format as JSON
        #[arg(long)]
        json: bool,

        /// Maximum number of results
        #[arg(short = 'n', long)]
        limit: Option<usize>,

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

        /// Only show count and timing, not the actual results
        #[arg(short, long)]
        count: bool,
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

        // Execute the subcommand
        match self.command {
            Command::Index { path, force, languages, progress } => {
                handle_index(path, force, languages, progress)
            }
            Command::Query { pattern, symbols, lang, kind, ast, json, limit, expand, file, exact, count } => {
                handle_query(pattern, symbols, lang, kind, ast, json, limit, expand, file, exact, count)
            }
            Command::Serve { port, host } => {
                handle_serve(port, host)
            }
            Command::Stats { json } => {
                handle_stats(json)
            }
            Command::Clear { yes } => {
                handle_clear(yes)
            }
            Command::ListFiles { json } => {
                handle_list_files(json)
            }
        }
    }
}

/// Handle the `index` subcommand
fn handle_index(path: PathBuf, force: bool, languages: Vec<String>, show_progress: bool) -> Result<()> {
    log::info!("Starting index command");

    let cache = CacheManager::new(&path);

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
    let stats = indexer.index(&path, show_progress)?;

    println!("Indexing complete!");
    println!("  Files indexed: {}", stats.total_files);
    println!("  Symbols found: {}", stats.total_symbols);
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
        println!("  {:<width$}  Files", "Language", width = lang_width);
        println!("  {}  {}", "-".repeat(lang_width), "-----");

        // Print rows
        for (language, count) in lang_vec {
            println!("  {:<width$}  {}", language, count, width = lang_width);
        }
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

/// Handle the `query` subcommand
fn handle_query(
    pattern: String,
    symbols_flag: bool,
    lang: Option<String>,
    kind_str: Option<String>,
    use_ast: bool,
    as_json: bool,
    limit: Option<usize>,
    expand: bool,
    file_pattern: Option<String>,
    exact: bool,
    count_only: bool,
) -> Result<()> {
    log::info!("Starting query command");

    let cache = CacheManager::new(".");
    let engine = QueryEngine::new(cache);

    // Parse and validate language filter
    let language = if let Some(lang_str) = lang.as_deref() {
        match lang_str.to_lowercase().as_str() {
            "rust" | "rs" => Some(Language::Rust),
            "javascript" | "js" => Some(Language::JavaScript),
            "typescript" | "ts" => Some(Language::TypeScript),
            "vue" => Some(Language::Vue),
            "svelte" => Some(Language::Svelte),
            // Unsupported languages (no parser yet)
            "python" | "py" => {
                anyhow::bail!("Language 'python' is not yet supported. Supported languages: rust, javascript (js), typescript (ts), vue, svelte, php");
            }
            "go" => {
                anyhow::bail!("Language 'go' is not yet supported. Supported languages: rust, javascript (js), typescript (ts), vue, svelte, php");
            }
            "java" => {
                anyhow::bail!("Language 'java' is not yet supported. Supported languages: rust, javascript (js), typescript (ts), vue, svelte, php");
            }
            "php" => Some(Language::PHP),
            "c" => {
                anyhow::bail!("Language 'c' is not yet supported. Supported languages: rust, javascript (js), typescript (ts), vue, svelte, php");
            }
            "cpp" | "c++" => {
                anyhow::bail!("Language 'c++' is not yet supported. Supported languages: rust, javascript (js), typescript (ts), vue, svelte, php");
            }
            _ => {
                anyhow::bail!("Unknown language '{}'. Supported languages: rust, javascript (js), typescript (ts), vue, svelte, php", lang_str);
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

    let filter = QueryFilter {
        language,
        kind,
        use_ast,
        limit,
        symbols_mode,
        expand,
        file_pattern,
        exact,
    };

    // Measure query time
    let start = Instant::now();
    let results = engine.search(&pattern, filter)?;
    let elapsed = start.elapsed();

    // Format timing string
    let timing_str = if elapsed.as_millis() < 1 {
        format!("{:.1}ms", elapsed.as_secs_f64() * 1000.0)
    } else {
        format!("{}ms", elapsed.as_millis())
    };

    // Count-only mode: just show the count and timing
    if count_only {
        println!("Found {} results in {}", results.len(), timing_str);
        return Ok(());
    }

    if as_json {
        println!("{}", serde_json::to_string_pretty(&results)?);
        eprintln!("Found {} results in {}", results.len(), timing_str);
    } else {
        if results.is_empty() {
            println!("No results found (searched in {}).", timing_str);
        } else {
            println!("Found {} results in {}:\n", results.len(), timing_str);
            for result in results {
                println!("{}:{} - {} {}",
                         result.path,
                         result.span.start_line,
                         format!("{:?}", result.kind),
                         result.symbol);
                if let Some(scope) = result.scope {
                    println!("  Scope: {}", scope);
                }
                println!("  {}\n", result.preview);
            }
        }
    }

    Ok(())
}

/// Handle the `serve` subcommand
fn handle_serve(port: u16, host: String) -> Result<()> {
    log::info!("Starting HTTP server on {}:{}", host, port);

    // TODO: Implement HTTP server using axum
    // - GET /query?q=pattern&lang=rust
    // - GET /stats
    // - POST /index

    println!("Starting RefLex HTTP server...");
    println!("  Address: http://{}:{}", host, port);
    println!("\nEndpoints:");
    println!("  GET  /query?q=<pattern>&lang=<lang>");
    println!("  GET  /stats");
    println!("  POST /index");
    println!("\nPress Ctrl+C to stop.");

    // Placeholder until full implementation
    anyhow::bail!("HTTP server not yet implemented");
}

/// Handle the `stats` subcommand
fn handle_stats(as_json: bool) -> Result<()> {
    log::info!("Showing index statistics");

    let cache = CacheManager::new(".");

    if !cache.exists() {
        anyhow::bail!("No index found. Run 'reflex index' first.");
    }

    let stats = cache.stats()?;

    if as_json {
        println!("{}", serde_json::to_string_pretty(&stats)?);
    } else {
        println!("RefLex Index Statistics");
        println!("=======================");
        println!("Files indexed:  {}", stats.total_files);
        println!("Symbols found:  {}", stats.total_symbols);
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
        println!("This will delete the local RefLex cache at: {:?}", cache.path());
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
fn handle_list_files(as_json: bool) -> Result<()> {
    let cache = CacheManager::new(".");

    if !cache.exists() {
        anyhow::bail!("No index found. Run 'reflex index' first.");
    }

    let files = cache.list_files()?;

    if as_json {
        println!("{}", serde_json::to_string_pretty(&files)?);
    } else {
        if files.is_empty() {
            println!("No files indexed yet.");
        } else {
            println!("Indexed Files ({} total):", files.len());
            println!();
            for file in files {
                println!("  {} ({} - {} symbols)",
                         file.path,
                         file.language,
                         file.symbol_count);
            }
        }
    }

    Ok(())
}
