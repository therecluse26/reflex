//! Core data models for RefLex
//!
//! These structures represent the normalized, deterministic output format
//! that RefLex provides to AI agents and other programmatic consumers.

use serde::{Deserialize, Serialize};
use strum::{EnumString, Display};

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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, EnumString, Display)]
#[strum(serialize_all = "PascalCase")]
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
    /// Catch-all for symbol kinds not yet explicitly supported.
    /// This ensures no data loss when encountering new tree-sitter node types.
    /// The string contains the original kind name from the parser.
    #[strum(default)]
    Unknown(String),
}

/// Programming language identifier
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Vue,
    Svelte,
    Go,
    Java,
    PHP,
    C,
    Cpp,
    CSharp,
    Ruby,
    Kotlin,
    Swift,
    Zig,
    Unknown,
}

impl Language {
    pub fn from_extension(ext: &str) -> Self {
        match ext {
            "rs" => Language::Rust,
            "py" => Language::Python,
            "js" | "mjs" | "cjs" | "jsx" => Language::JavaScript,
            "ts" | "mts" | "cts" | "tsx" => Language::TypeScript,
            "vue" => Language::Vue,
            "svelte" => Language::Svelte,
            "go" => Language::Go,
            "java" => Language::Java,
            "php" => Language::PHP,
            "c" | "h" => Language::C,
            "cpp" | "cc" | "cxx" | "hpp" | "hxx" | "C" | "H" => Language::Cpp,
            "cs" => Language::CSharp,
            "rb" | "rake" | "gemspec" => Language::Ruby,
            "kt" | "kts" => Language::Kotlin,
            "swift" => Language::Swift,
            "zig" => Language::Zig,
            _ => Language::Unknown,
        }
    }

    /// Check if this language has a parser implementation
    ///
    /// Returns true only for languages with working Tree-sitter parsers.
    /// This determines which files will be indexed by RefLex.
    pub fn is_supported(&self) -> bool {
        match self {
            Language::Rust => true,
            Language::TypeScript => true,
            Language::JavaScript => true,
            Language::Vue => true,
            Language::Svelte => true,
            Language::Python => true,
            Language::Go => true,
            Language::Java => true,
            Language::PHP => true,
            Language::C => true,
            Language::Cpp => true,
            Language::CSharp => true,
            Language::Ruby => true,
            Language::Kotlin => true,
            Language::Swift => false,  // Temporarily disabled - requires tree-sitter 0.23
            Language::Zig => true,
            Language::Unknown => false,
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
    /// None for text/regex matches where symbol name cannot be accurately determined
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
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
        symbol: Option<String>,
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
    /// Number of threads for parallel indexing (0 = auto, 80% of available cores)
    pub parallel_threads: usize,
    /// Query timeout in seconds (0 = no timeout)
    pub query_timeout_secs: u64,
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            languages: vec![],
            include_patterns: vec![],
            exclude_patterns: vec![],
            follow_symlinks: false,
            max_file_size: 10 * 1024 * 1024, // 10 MB
            parallel_threads: 0, // 0 = auto (80% of available cores)
            query_timeout_secs: 30, // 30 seconds default timeout
        }
    }
}

/// Statistics about the index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    /// Total files indexed
    pub total_files: usize,
    /// Index size on disk (bytes)
    pub index_size_bytes: u64,
    /// Last update timestamp
    pub last_updated: String,
    /// File count breakdown by language
    pub files_by_language: std::collections::HashMap<String, usize>,
    /// Line count breakdown by language
    pub lines_by_language: std::collections::HashMap<String, usize>,
}

/// Information about an indexed file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedFile {
    /// File path
    pub path: String,
    /// Detected language
    pub language: String,
    /// Last indexed timestamp
    pub last_indexed: String,
}

/// Index status for query responses
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IndexStatus {
    /// Index is fresh and up-to-date
    Fresh,
    /// Index is stale (any issue: branch not indexed, commit changed, files modified)
    Stale,
}

/// Warning details when index is stale
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexWarning {
    /// Human-readable reason why index is stale
    pub reason: String,
    /// Command to run to fix the issue
    pub action_required: String,
    /// Additional context (git branch info, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<IndexWarningDetails>,
}

/// Detailed information about index staleness
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexWarningDetails {
    /// Current branch (if in git repo)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_branch: Option<String>,
    /// Indexed branch (if in git repo)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexed_branch: Option<String>,
    /// Current commit SHA (if in git repo)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_commit: Option<String>,
    /// Indexed commit SHA (if in git repo)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexed_commit: Option<String>,
}

/// Query response with results and index status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResponse {
    /// Status of the index (fresh or stale)
    pub status: IndexStatus,
    /// Whether the results can be trusted
    pub can_trust_results: bool,
    /// Warning information (only present if stale)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<IndexWarning>,
    /// Search results
    pub results: Vec<SearchResult>,
}
