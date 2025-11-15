# Reflex

**Local-first, full-text code search engine for AI coding workflows**

Reflex is a blazingly fast, trigram-based code search engine designed for developers and AI coding assistants. Unlike symbol-only tools, Reflex finds **every occurrence** of patterns—function calls, variable usage, comments, and more—with sub-100ms query times on large codebases.

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)]()
[![Tests](https://img.shields.io/badge/tests-347%20passing-brightgreen)]()
[![License](https://img.shields.io/badge/license-MIT-blue)]()

## ✨ Features

- **🔍 Complete Coverage**: Find every occurrence, not just symbol definitions
- **⚡ Blazing Fast**: Sub-100ms queries on 10k+ files via trigram indexing
- **🎯 Symbol-Aware**: Runtime tree-sitter parsing for precise symbol filtering
- **🔄 Incremental**: Only reindexes changed files (blake3 hashing)
- **🌍 Multi-Language**: Rust, TypeScript/JavaScript, Vue, Svelte, PHP, Python, Go, Java, C, C++, C#, Ruby, Kotlin, Zig
- **🤖 AI-Ready**: Clean JSON output built for LLM tools and automation
- **🌐 HTTP API**: REST API for editor plugins and external tools
- **📡 MCP Support**: Model Context Protocol server for AI assistants
- **📦 Local-First**: Fully offline, all data stays on your machine
- **🎨 Regex Support**: Trigram-optimized regex search
- **🌳 AST Queries**: Structure-aware search with Tree-sitter
- **📊 Interactive TUI**: Full-featured terminal interface (run `rfx` with no args)
- **🔒 Deterministic**: Same query → same results (no probabilistic ranking)

## 🚀 Quick Start

### Installation

```bash
# Clone and build from source
git clone https://github.com/reflex-search/reflex.git
cd reflex
cargo build --release

# Binary will be at target/release/rfx
```

### Basic Usage

```bash
# Index your codebase
rfx index

# Launch interactive mode (TUI)
rfx

# Full-text search (finds all occurrences)
rfx query "extract_symbols"

# Symbol-only search (definitions only)
rfx query "extract_symbols" --symbols

# Filter by language and symbol kind
rfx query "parse" --lang rust --kind function --symbols

# Include dependency information (imports/exports)
rfx query "MyStruct" --dependencies

# Regex search
rfx query "fn.*test" --regex

# Paths-only mode (for piping to other tools)
vim $(rfx query "TODO" --paths)

# Export as JSON for AI agents
rfx query "unwrap" --json --limit 10
```

## 📋 Command Reference

### `rfx` (no arguments)

Launch **interactive mode** - a full-featured TUI for exploring your codebase with real-time search, syntax highlighting, and keyboard navigation.

### `rfx index`

Build or update the local search index.

```bash
rfx index [OPTIONS]

Options:
  --force              Force full reindex (ignore incremental)
  --languages <LANGS>  Limit to specific languages (comma-separated)
  --status             Show background symbol indexing status
```

**Background Symbol Indexing:** After indexing, Reflex automatically starts a background process to cache symbols for faster queries. Check status with `rfx index --status`.

### `rfx query`

Search the codebase. Run `rfx query --help` for full options.

**Key Options:**
- `--symbols, -s` - Symbol-only search (definitions, not usage)
- `--regex, -r` - Treat pattern as regex
- `--lang <LANG>` - Filter by language
- `--kind <KIND>` - Filter by symbol kind (function, class, struct, etc.)
- `--dependencies` - Include dependency information (currently Rust only)
- `--paths, -p` - Return only file paths (no content)
- `--json` - Output as JSON
- `--limit <N>` - Limit number of results
- `--timeout <SECS>` - Query timeout (default: 30s)

**Examples:**
```bash
# Find function definitions named "parse"
rfx query "parse" --symbols --kind function

# Find test functions using regex
rfx query "fn test_\w+" --regex

# Search Rust files only
rfx query "unwrap" --lang rust

# Get paths of files with TODOs
rfx query "TODO" --paths

# Include import information
rfx query "Config" --symbols --dependencies
```

### `rfx serve`

Start an HTTP API server for programmatic access.

```bash
rfx serve --port 7878 --host 127.0.0.1
```

**Endpoints:**
- `GET /query` - Search (params: `q`, `lang`, `kind`, `limit`, `symbols`, `regex`, `dependencies`, etc.)
- `GET /stats` - Index statistics
- `POST /index` - Trigger reindexing
- `GET /health` - Health check

### `rfx mcp`

Start as an MCP (Model Context Protocol) server for AI coding assistants.

**Configuration for Claude Code** (`~/.claude/claude_code_config.json`):
```json
{
  "mcpServers": {
    "reflex": {
      "type": "stdio",
      "command": "rfx",
      "args": ["mcp"]
    }
  }
}
```

**Available MCP Tools:**
1. **`list_locations`** - Fast location discovery (file + line only, minimal tokens)
2. **`count_occurrences`** - Quick statistics (total count + file count)
3. **`search_code`** - Full-text or symbol search with detailed results
4. **`search_regex`** - Regex pattern matching
5. **`search_ast`** - AST pattern matching (structure-aware, slow)
6. **`index_project`** - Trigger reindexing
7. **`get_dependencies`** - Get all dependencies of a specific file
8. **`get_dependents`** - Get all files that depend on a file (reverse lookup)
9. **`get_transitive_deps`** - Get transitive dependencies up to a specified depth
10. **`find_hotspots`** - Find most-imported files (with pagination)
11. **`find_circular`** - Detect circular dependencies (with pagination)
12. **`find_unused`** - Find files with no incoming dependencies (with pagination)
13. **`find_islands`** - Find disconnected components (with pagination)

### `rfx analyze`

Analyze codebase structure and dependencies. By default shows a summary; use specific flags for detailed results.

**Subcommands:**
- `--circular` - Detect circular dependencies (A → B → C → A)
- `--hotspots` - Find most-imported files
- `--unused` - Find files with no incoming dependencies
- `--islands` - Find disconnected components

**Pagination (default: 200 results per page):**
- Use `--limit N` to specify results per page
- Use `--offset N` to skip first N results
- Use `--all` to return unlimited results

**Examples:**
```bash
# Show summary of all analyses
rfx analyze

# Find circular dependencies
rfx analyze --circular

# Find hotspots (most-imported files)
rfx analyze --hotspots --min-dependents 5

# Find unused files
rfx analyze --unused

# Find disconnected components (islands)
rfx analyze --islands --min-island-size 3

# Paginate results
rfx analyze --hotspots --limit 50 --offset 0  # First 50
rfx analyze --hotspots --limit 50 --offset 50 # Next 50

# Export as JSON with pagination metadata
rfx analyze --circular --json
```

**JSON Output Format:**
```json
{
  "pagination": {
    "total": 347,
    "count": 200,
    "offset": 0,
    "limit": 200,
    "has_more": true
  },
  "results": [...]
}
```

### Other Commands

- `rfx stats` - Display index statistics
- `rfx clear` - Clear the search index
- `rfx list-files` - List all indexed files
- `rfx watch` - Watch for file changes and auto-reindex
- `rfx deps <file>` - Analyze dependencies for a specific file

Run `rfx <command> --help` for detailed options.

## 🌳 AST Pattern Matching

Reflex supports **structure-aware code search** using Tree-sitter AST queries.

**⚠️ WARNING:** AST queries are **SLOW** (500ms-2s+) and scan the entire codebase. **Use `--symbols` instead for 95% of cases** (10-100x faster).

**When to use AST queries:**
- You need to match code structure, not just text
- `--symbols` search is insufficient for your use case
- You have a very specific structural pattern

**Basic usage:**
```bash
rfx query <PATTERN> --ast <AST_PATTERN> --lang <LANGUAGE>

# Example: Find all Rust functions
rfx query "fn" --ast "(function_item) @fn" --lang rust

# Example: Find all TypeScript classes
rfx query "class" --ast "(class_declaration) @class" --lang typescript
```

**Supported languages:** Rust, TypeScript, JavaScript, Python, Go, Java, C, C++, C#, PHP, Ruby, Kotlin, Zig

For detailed AST query syntax and examples, see the [Tree-sitter documentation](https://tree-sitter.github.io/tree-sitter/using-parsers#pattern-matching-with-queries).

## 🌐 Supported Languages

| Language | Extensions | Symbol Extraction |
|----------|------------|-------------------|
| **Rust** | `.rs` | Functions, structs, enums, traits, impls, modules, methods |
| **TypeScript** | `.ts`, `.tsx`, `.mts`, `.cts` | Functions, classes, interfaces, types, enums, React components |
| **JavaScript** | `.js`, `.jsx`, `.mjs`, `.cjs` | Functions, classes, constants, methods, React components |
| **Vue** | `.vue` | Functions, constants, methods from `<script>` blocks |
| **Svelte** | `.svelte` | Functions, variables, reactive declarations |
| **PHP** | `.php` | Functions, classes, interfaces, traits, methods, namespaces, enums |
| **Python** | `.py` | Functions, classes, methods, decorators, lambdas |
| **Go** | `.go` | Functions, types, interfaces, methods, constants |
| **Java** | `.java` | Classes, interfaces, enums, methods, fields, constructors |
| **C** | `.c`, `.h` | Functions, structs, enums, unions, typedefs |
| **C++** | `.cpp`, `.hpp`, `.cxx` | Functions, classes, namespaces, templates, methods |
| **C#** | `.cs` | Classes, interfaces, structs, enums, methods, properties |
| **Ruby** | `.rb`, `.rake`, `.gemspec` | Classes, modules, methods, constants, variables |
| **Kotlin** | `.kt`, `.kts` | Classes, functions, interfaces, objects, properties |
| **Zig** | `.zig` | Functions, structs, enums, constants, variables |

**Note:** Full-text search works on **all file types** regardless of parser support. Symbol filtering requires a language parser.

## 🏗️ Architecture

Reflex uses a **trigram-based inverted index** combined with **runtime symbol detection**:

### Indexing Phase
1. Extract trigrams (3-character substrings) from all files
2. Build inverted index: `trigram → [file_id, line_no]`
3. Store full file contents in memory-mapped `content.bin`
4. Start background symbol indexing (caches symbols for faster queries)

### Query Phase
1. **Full-text queries**: Intersect trigram posting lists → verify matches
2. **Symbol queries**: Trigrams narrow to ~10-100 candidates → parse with tree-sitter → filter symbols
3. Memory-mapped I/O for instant cache access

### Cache Structure (`.reflex/`)
```
.reflex/
  meta.db          # SQLite: file metadata, stats, config, hashes
  trigrams.bin     # Inverted index (memory-mapped)
  content.bin      # Full file contents (memory-mapped)
  config.toml      # Index settings
  indexing.status  # Background symbol indexer status
```

## ⚡ Performance

### Query Performance (Real-World Benchmarks)

| Codebase | Files | Full-Text Query | Symbol Query | Regex Query |
|----------|-------|-----------------|--------------|-------------|
| **Reflex** (small) | 96 | 5-6 ms | 581 ms | 6 ms |
| **Test corpus** (medium) | 100-500 | 2 ms | 944 ms | 2 ms |
| **Large project** | 1,000+ | 2-3 ms | 1-2 sec | 2-3 ms |

**Key Insights:**
- **Full-text & Regex**: Blazing fast (2-6ms) regardless of codebase size
- **Symbol queries**: Slower (500ms-2s) due to runtime tree-sitter parsing
- **Cached queries**: 1ms average for repeated queries

### Indexing Performance

| Operation | Files | Time | Notes |
|-----------|-------|------|-------|
| **Initial index** | 100-1,000 | 95-106ms | Parallel processing with 80% CPU cores |
| **Incremental** | 10/100 changed | 32ms | Only rehashes changed files |

## 🔧 Configuration

Reflex respects `.gitignore` files automatically. Additional configuration via `.reflex/config.toml`:

```toml
[index]
languages = []  # Empty = all supported languages
max_file_size = 10485760  # 10 MB
follow_symlinks = false

[search]
default_limit = 100

[performance]
parallel_threads = 0  # 0 = auto (80% of available cores)
```

## 🤖 AI Integration

Reflex outputs clean JSON for AI coding assistants:

```bash
rfx query "parse_tree" --json --symbols
```

**Example JSON output:**
```json
{
  "status": "fresh",
  "can_trust_results": true,
  "pagination": {
    "total": 1,
    "count": 1,
    "offset": 0,
    "limit": 100,
    "has_more": false
  },
  "results": [
    {
      "path": "src/parsers/rust.rs",
      "span": {
        "start_line": 45,
        "end_line": 45
      },
      "symbol": "parse_tree",
      "kind": "Function",
      "preview": "pub fn parse_tree(source: &str) -> Tree {"
    }
  ]
}
```

## 🔍 Use Cases

### For Developers
- **Code Navigation**: Find all usages of a function/class
- **Refactoring**: Identify all call sites before renaming
- **Code Review**: Search for patterns across files
- **Debugging**: Locate where variables are used

### For AI Coding Assistants
- **Context Gathering**: Retrieve relevant code snippets
- **Symbol Lookup**: Find function definitions and signatures
- **Pattern Analysis**: Search for architectural patterns
- **Dependency Tracking**: Understand import relationships

### For Teams
- **Code Search**: Local alternative to Sourcegraph
- **Documentation**: Find examples of API usage
- **Onboarding**: Explore unfamiliar codebases
- **Security**: Search for potential vulnerabilities

## 🧪 Testing

Reflex has **347 comprehensive tests** covering all functionality:
- **261+ unit tests**: Core modules (cache, indexer, query, parsers, trigrams, AST, symbol cache)
- **42+ corpus tests**: Real-world code samples across all supported languages
- **17+ integration tests**: End-to-end workflows, multi-language support, error handling
- **10+ performance tests**: Indexing speed, query latency, scalability benchmarks

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test module
cargo test indexer::tests
```

All tests pass on Linux, macOS, and Windows. See [TESTING.md](docs/TESTING.md) for details.

## 🤝 Contributing

Contributions welcome! Reflex is built to be:
- **Fast**: Sub-100ms queries on large codebases
- **Accurate**: Complete coverage with deterministic results
- **Extensible**: Easy to add new language parsers

See [ARCHITECTURE.md](docs/ARCHITECTURE.md) for implementation details and [CLAUDE.md](CLAUDE.md) for development workflow.

## 📚 Documentation

- **[ARCHITECTURE.md](docs/ARCHITECTURE.md)**: System design, data formats, extension guide
- **[CLAUDE.md](CLAUDE.md)**: Project overview and development workflow
- **[TESTING.md](docs/TESTING.md)**: Test suite documentation
- **[DISCREPANCIES.md](docs/DISCREPANCIES.md)**: Known documentation issues

## 📄 License

MIT License - see [LICENSE](LICENSE) for details.

## 🙏 Acknowledgments

Built with:
- [tree-sitter](https://tree-sitter.github.io/tree-sitter/) - Incremental parsing
- [rkyv](https://rkyv.org/) - Zero-copy deserialization
- [memmap2](https://github.com/RazrFalcon/memmap2-rs) - Memory-mapped I/O
- [rusqlite](https://github.com/rusqlite/rusqlite) - SQLite bindings
- [blake3](https://github.com/BLAKE3-team/BLAKE3) - Fast hashing
- [ignore](https://github.com/BurntSushi/ripgrep/tree/master/crates/ignore) - gitignore support

Inspired by:
- [Zoekt](https://github.com/sourcegraph/zoekt) - Trigram-based code search
- [Sourcegraph](https://sourcegraph.com/) - Code search for teams
- [ripgrep](https://github.com/BurntSushi/ripgrep) - Fast text search

---

**Made with ❤️ for developers and AI coding assistants**
