//! CLI argument parsing and command handlers

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

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
    },

    /// Query the code index
    Query {
        /// Search pattern (e.g., "symbol:get_user", "fn main", or plain text)
        pattern: String,

        /// Filter by language
        #[arg(short, long)]
        lang: Option<String>,

        /// Use AST pattern matching
        #[arg(long)]
        ast: bool,

        /// Output format as JSON
        #[arg(long)]
        json: bool,

        /// Maximum number of results
        #[arg(short = 'n', long)]
        limit: Option<usize>,
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
            0 => "info",
            1 => "debug",
            _ => "trace",
        };
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level))
            .init();

        // Execute the subcommand
        match self.command {
            Command::Index { path, force, languages } => {
                handle_index(path, force, languages)
            }
            Command::Query { pattern, lang, ast, json, limit } => {
                handle_query(pattern, lang, ast, json, limit)
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
fn handle_index(path: PathBuf, force: bool, languages: Vec<String>) -> Result<()> {
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
    let stats = indexer.index(&path)?;

    println!("Indexing complete!");
    println!("  Files indexed: {}", stats.total_files);
    println!("  Symbols found: {}", stats.total_symbols);
    println!("  Cache size: {} bytes", stats.index_size_bytes);
    println!("  Last updated: {}", stats.last_updated);

    Ok(())
}

/// Handle the `query` subcommand
fn handle_query(
    pattern: String,
    lang: Option<String>,
    use_ast: bool,
    as_json: bool,
    limit: Option<usize>,
) -> Result<()> {
    log::info!("Starting query command");

    let cache = CacheManager::new(".");
    let engine = QueryEngine::new(cache);

    let language = lang.as_deref().and_then(|s| match s.to_lowercase().as_str() {
        "rust" | "rs" => Some(Language::Rust),
        "python" | "py" => Some(Language::Python),
        "javascript" | "js" => Some(Language::JavaScript),
        "typescript" | "ts" => Some(Language::TypeScript),
        "go" => Some(Language::Go),
        "java" => Some(Language::Java),
        "php" => Some(Language::PHP),
        "c" => Some(Language::C),
        "cpp" | "c++" => Some(Language::Cpp),
        _ => None,
    });

    let filter = QueryFilter {
        language,
        use_ast,
        limit,
        ..Default::default()
    };

    let results = engine.search(&pattern, filter)?;

    if as_json {
        println!("{}", serde_json::to_string_pretty(&results)?);
    } else {
        if results.is_empty() {
            println!("No results found.");
        } else {
            println!("Found {} results:\n", results.len());
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
