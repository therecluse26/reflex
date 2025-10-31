//! Core data models for RefLex
//!
//! These structures represent the normalized, deterministic output format
//! that RefLex provides to AI agents and other programmatic consumers.

use serde::{Deserialize, Serialize};

/// Represents a source code location span (line:col range)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Span {
    /// Starting line number (1-indexed)
    pub start_line: usize,
    /// Starting column number (0-indexed)
    pub start_col: usize,
    /// Ending line number (1-indexed)
    pub end_line: usize,
    /// Ending column number (0-indexed)
    pub end_col: usize,
}

impl Span {
    pub fn new(start_line: usize, start_col: usize, end_line: usize, end_col: usize) -> Self {
        Self {
            start_line,
            start_col,
            end_line,
            end_col,
        }
    }
}

/// Type of symbol found in code
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SymbolKind {
    Function,
    Class,
    Struct,
    Enum,
    Interface,
    Trait,
    Constant,
    Variable,
    Method,
    Module,
    Namespace,
    Type,
    Import,
    Export,
}

/// Programming language identifier
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Java,
    PHP,
    C,
    Cpp,
    Unknown,
}

impl Language {
    pub fn from_extension(ext: &str) -> Self {
        match ext {
            "rs" => Language::Rust,
            "py" => Language::Python,
            "js" | "mjs" | "cjs" => Language::JavaScript,
            "ts" | "mts" | "cts" => Language::TypeScript,
            "go" => Language::Go,
            "java" => Language::Java,
            "php" => Language::PHP,
            "c" | "h" => Language::C,
            "cpp" | "cc" | "cxx" | "hpp" | "hxx" | "C" | "H" => Language::Cpp,
            _ => Language::Unknown,
        }
    }
}

/// A search result representing a symbol or code location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Absolute or relative path to the file
    pub path: String,
    /// Detected programming language
    pub lang: Language,
    /// Type of symbol found
    pub kind: SymbolKind,
    /// Symbol name (e.g., function name, class name)
    pub symbol: String,
    /// Location span in the source file
    pub span: Span,
    /// Scope context (e.g., "impl MyStruct", "class User")
    pub scope: Option<String>,
    /// Code preview (few lines around the match)
    pub preview: String,
}

impl SearchResult {
    pub fn new(
        path: String,
        lang: Language,
        kind: SymbolKind,
        symbol: String,
        span: Span,
        scope: Option<String>,
        preview: String,
    ) -> Self {
        Self {
            path,
            lang,
            kind,
            symbol,
            span,
            scope,
            preview,
        }
    }
}

/// Configuration for indexing behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexConfig {
    /// Languages to include (empty = all supported)
    pub languages: Vec<Language>,
    /// Glob patterns to include
    pub include_patterns: Vec<String>,
    /// Glob patterns to exclude
    pub exclude_patterns: Vec<String>,
    /// Follow symbolic links
    pub follow_symlinks: bool,
    /// Maximum file size to index (bytes)
    pub max_file_size: usize,
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            languages: vec![],
            include_patterns: vec![],
            exclude_patterns: vec![],
            follow_symlinks: false,
            max_file_size: 10 * 1024 * 1024, // 10 MB
        }
    }
}

/// Statistics about the index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    /// Total files indexed
    pub total_files: usize,
    /// Total symbols found
    pub total_symbols: usize,
    /// Index size on disk (bytes)
    pub index_size_bytes: u64,
    /// Last update timestamp
    pub last_updated: String,
}
