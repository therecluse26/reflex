//! RefLex: Local-first, structure-aware code search engine
//!
//! RefLex is a fast, deterministic code search tool designed specifically
//! for AI coding agents. It provides structured results (symbols, spans,
//! scopes) with sub-100ms latency by maintaining a lightweight, incremental
//! cache in `.reflex/`.
//!
//! # Architecture
//!
//! - **Indexer**: Scans and parses code with Tree-sitter; writes to cache
//! - **Query Engine**: Loads cache on demand; executes deterministic searches
//! - **Cache**: Memory-mapped storage for symbols, tokens, and metadata
//!
//! # Example Usage
//!
//! ```no_run
//! use reflex::{cache::CacheManager, indexer::Indexer, models::IndexConfig};
//!
//! // Create and initialize index
//! let cache = CacheManager::new(".");
//! let config = IndexConfig::default();
//! let indexer = Indexer::new(cache, config);
//! let stats = indexer.index(".", false).unwrap();
//!
//! println!("Indexed {} symbols", stats.total_symbols);
//! ```

pub mod cache;
pub mod cli;
pub mod content_store;
pub mod indexer;
pub mod models;
pub mod parsers;
pub mod query;
pub mod trigram;

// Re-export commonly used types
pub use cache::CacheManager;
pub use indexer::Indexer;
pub use models::{IndexConfig, IndexStats, IndexedFile, Language, SearchResult, Span, SymbolKind};
pub use query::{QueryEngine, QueryFilter};
