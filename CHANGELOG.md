# Changelog

All notable changes to RefLex will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- Query timeout support with `--timeout` flag (default: 30 seconds)
- HTTP API timeout parameter support
- MCP server with 30-second default timeout for all queries

## [1.0.0] - 2025-11-03

Initial release of RefLex - a local-first, structure-aware code search engine for AI coding workflows.

### Added

#### Core Features
- **Trigram-based indexing** for sub-100ms full-text search on large codebases (10k+ files)
- **Runtime symbol detection** using Tree-sitter parsing on candidate files only
- **Complete coverage** - finds every occurrence of patterns, not just symbol definitions
- **Deterministic results** - same query always returns same results (sorted by file:line)
- **Memory-mapped I/O** for instant cache access (zero-copy)
- **Incremental indexing** using blake3 content hashing (only reindex changed files)
- **Regex support** with trigram optimization for fast pattern matching
- **AST pattern matching** using Tree-sitter S-expression queries

#### Language Support
- **Rust** - Functions, structs, enums, traits, impls, modules, methods
- **TypeScript** - Functions, classes, interfaces, types, enums, React components
- **JavaScript** - Functions, classes, constants, methods, React components
- **Vue** - Functions, constants, methods from `<script>` blocks (Composition API support)
- **Svelte** - Functions, variables, reactive declarations (`$:`)
- **PHP** - Functions, classes, interfaces, traits, methods, namespaces, enums (PHP 8.1+)

#### CLI Commands
- `rfx index` - Build or update the local search index
- `rfx query` - Search the codebase with multiple search modes:
  - Full-text search (default - finds all occurrences)
  - Symbol-only search (`--symbols` flag)
  - Regex search (`--regex` flag)
  - AST pattern matching (`--ast` flag)
- `rfx stats` - Display index statistics
- `rfx clear` - Clear the search index
- `rfx list-files` - List all indexed files
- `rfx watch` - Auto-reindex on file changes with configurable debouncing
- `rfx serve` - Start HTTP API server for programmatic access
- `rfx mcp` - Start MCP (Model Context Protocol) server for AI agents

#### Search Features
- **Symbol filtering** by kind (function, class, struct, enum, etc.)
- **Language filtering** to search specific languages only
- **File path filtering** with substring matching
- **Result limiting** with `--limit` flag
- **Exact matching** mode with `--exact` flag
- **Symbol expansion** with `--expand` flag (show full function/class body)
- **JSON output** for AI agents and automation tools
- **Count-only mode** with `--count` flag

#### HTTP API
- **GET /query** - Search the codebase
  - Query params: `q`, `lang`, `kind`, `limit`, `symbols`, `regex`, `exact`, `expand`, `file`
  - Returns: `QueryResponse` JSON with results and index status
- **GET /stats** - Get index statistics
  - Returns: `IndexStats` JSON with file counts, sizes, language breakdowns
- **POST /index** - Trigger reindexing
  - Body: `{"force": boolean, "languages": [string]}`
  - Returns: `IndexStats` JSON after indexing completes
- **GET /health** - Health check endpoint
- **CORS enabled** for browser clients

#### MCP Server
- **search_code** - Full-text or symbol search
- **search_regex** - Regex pattern matching with trigram optimization
- **search_ast** - Structure-aware AST pattern matching
- **index_project** - Trigger reindexing
- **stdio transport** for seamless integration with Claude Code and other MCP clients

#### File Watching
- **Auto-reindex** on file changes
- **Configurable debouncing** (5-30 seconds, default: 15s)
- **Quiet mode** for background operation
- **Respects .gitignore** patterns automatically

#### Index Features
- **Git-aware** - tracks current branch and commit SHA
- **Staleness detection** - warns when index is out of sync with working tree
- **Branch tracking** - separate indices per branch
- **Dirty state tracking** - knows when uncommitted changes exist
- **.gitignore support** - automatically excludes ignored files
- **Configurable max file size** (default: 10 MB)
- **Parallel indexing** using rayon (default: 80% of CPU cores)

#### Cache Structure
- `.reflex/meta.db` - SQLite database for file metadata and statistics
- `.reflex/trigrams.bin` - Memory-mapped inverted index (trigram → file locations)
- `.reflex/content.bin` - Memory-mapped full file contents for context extraction
- `.reflex/config.toml` - Index configuration (auto-generated)

#### Performance
- **Sub-100ms queries** on 10k+ files (trigram indexing)
- **2-3ms queries** on small codebases (50 files)
- **124ms full-text search** on Linux kernel (62K files)
- **224ms symbol search** on Linux kernel (runtime parsing of ~3 candidate files)
- **Incremental indexing**: <1 second for changed files

#### Documentation
- Comprehensive README.md with usage examples and API reference
- Detailed ARCHITECTURE.md with system design and data formats
- CLAUDE.md with project overview and development workflow
- 221 comprehensive tests (unit, integration, performance)
- Rustdoc comments for all public APIs

#### Quality & Testing
- **221 comprehensive tests**
  - 194 unit tests (cache, indexer, query, parsers, core modules)
  - 17 integration tests (workflows, multi-language, error handling)
  - 10 performance tests (indexing speed, query latency, scalability)
- **Zero unsafe code** - all safe Rust
- **Error handling** - comprehensive error messages with context
- **Logging support** - configurable with RUST_LOG environment variable

### Performance

- **Indexing**: 100 files in <1 second, 1,000 files in <2 seconds
- **Query latency**: Sub-100ms on large codebases (10k+ files)
- **Memory usage**: Efficient memory-mapped I/O (zero-copy)
- **Cache size**: Compressed trigram index + full file contents

### Architecture

- **Trigram-based inverted index** inspired by Zoekt/Google Code Search
- **Runtime symbol detection** - parse only candidate files at query time
- **Memory-mapped I/O** for instant cache access
- **Parallel processing** with rayon for multi-core indexing
- **Incremental updates** using blake3 content hashing
- **Zero-copy deserialization** with memory-mapped files

### Technology Stack

- **Language**: Rust (Edition 2024)
- **Parsing**: tree-sitter with language-specific grammars
- **Storage**: rusqlite (metadata), custom binary format (trigrams + content)
- **Hashing**: blake3 for fast content hashing
- **HTTP**: axum web framework with CORS support
- **CLI**: clap for argument parsing
- **Async**: tokio runtime for HTTP server
- **Parallelism**: rayon for multi-threaded indexing

[Unreleased]: https://github.com/yourusername/reflex/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/yourusername/reflex/releases/tag/v1.0.0
