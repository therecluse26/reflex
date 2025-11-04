# Reflex

**Local-first, full-text code search engine for AI coding workflows**

Reflex is a blazingly fast, trigram-based code search engine designed for developers and AI coding assistants. Unlike symbol-only tools, Reflex finds **every occurrence** of patterns—function calls, variable usage, comments, and more—with sub-100ms query times on large codebases.

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)]()
[![Tests](https://img.shields.io/badge/tests-221%20passing-brightgreen)]()
[![License](https://img.shields.io/badge/license-MIT-blue)]()

## ✨ Features

- **🔍 Complete Coverage**: Find every occurrence, not just symbol definitions
- **⚡ Blazing Fast**: Sub-100ms queries on 10k+ files via trigram indexing
- **🎯 Symbol-Aware**: Runtime tree-sitter parsing for precise symbol filtering
- **🌳 AST Pattern Matching**: Structure-aware search with Tree-sitter queries
- **🔄 Incremental**: Only reindexes changed files (blake3 hashing)
- **🌍 Multi-Language**: Rust, TypeScript/JavaScript, Vue, Svelte, PHP, Python, Go, Java, C, C++
- **🤖 AI-Ready**: Clean JSON output built for LLM tools and automation
- **🌐 HTTP API**: REST API for editor plugins and external tools
- **📦 Local-First**: Fully offline, all data stays on your machine
- **🎨 Regex Support**: Trigram-optimized regex search with pattern matching
- **🔒 Deterministic**: Same query → same results (no probabilistic ranking)

## 🚀 Quick Start

### Installation

```bash
# Clone and build from source
git clone https://github.com/therecluse26/reflex.git
cd reflex
cargo build --release

# Binary will be at target/release/rfx
```

### Basic Usage

```bash
# Index your codebase
rfx index

# Full-text search (finds all occurrences)
rfx query "extract_symbols"
# → Finds: function definitions + all call sites

# Symbol-only search (definitions only)
rfx query "extract_symbols" --symbols
# → Finds: only the function definition

# Regex search
rfx query "fn.*test" --regex

# Filter by language and symbol kind
rfx query "parse" --lang rust --kind function --symbols

# Export as JSON for AI agents
rfx query "unwrap" --json --limit 10

# Get index statistics
rfx stats

# Clear cache
rfx clear --yes
```

## 📋 Command Reference

### `rfx index`

Build or update the local search index.

```bash
rfx index [OPTIONS]

Options:
  --force              Force full reindex (ignore incremental)
  --languages <LANGS>  Limit to specific languages (comma-separated)
  --progress           Show progress during indexing
```

**Examples:**
```bash
# Incremental index (only changed files)
rfx index

# Force full reindex
rfx index --force

# Index only Rust and TypeScript files
rfx index --languages rust,typescript
```

### `rfx query`

Search the codebase.

```bash
rfx query <PATTERN> [OPTIONS]

Options:
  --symbols, -s        Symbol-only search (definitions, not usage)
  --regex, -r          Treat pattern as regex
  --ast <PATTERN>      AST pattern matching (requires --lang)
  --exact, -e          Exact match (no substring matching)
  --lang <LANG>        Filter by language (rust, typescript, python, etc.)
  --kind <KIND>        Filter by symbol kind (function, class, struct, etc.)
  --file <PATTERN>     Filter by file path (substring)
  --limit <N>          Limit number of results
  --expand             Show full symbol body (not just signature)
  --json               Output as JSON
  --count              Show only match count
  --timeout <SECS>     Query timeout in seconds (0 = no timeout, default: 30)
```

**Examples:**
```bash
# Find all occurrences of "hello" (full-text)
rfx query "hello"

# Find function definitions named "parse"
rfx query "parse" --symbols --kind function

# Regex: find test functions
rfx query "fn test_\w+" --regex

# Language filter: Rust files only
rfx query "unwrap" --lang rust

# File path filter: only src/ directory
rfx query "config" --file src/

# JSON output for AI tools
rfx query "format!" --json --limit 5

# Count matches
rfx query "TODO" --count

# Set custom timeout (10 seconds)
rfx query "complex.*pattern" --regex --timeout 10

# AST pattern matching (structure-aware search)
rfx query "fn" --ast "(function_item) @fn" --lang rust
```

### AST Pattern Matching

Reflex supports **structure-aware code search** using Tree-sitter AST queries. This allows you to search for specific code structures (like functions, classes, traits) rather than just text patterns.

**Important:**
- AST queries require `--lang` to be specified
- AST queries must have trigram pre-filtering (pattern text) for performance
- Query patterns must include captures using `@name` syntax

```bash
rfx query <TEXT_PATTERN> --ast <AST_PATTERN> --lang <LANGUAGE>
```

#### Supported Languages for AST Queries

- **Rust** (`rust`)
- **TypeScript** (`typescript`)
- **JavaScript** (`javascript`)
- **PHP** (`php`)

#### S-Expression Query Syntax

AST patterns use Lisp-like S-expressions with **captures** to match Tree-sitter AST nodes:

**Basic pattern structure:**
```
(node_type) @capture_name           Match and capture any node of this type
(node_type (child_type)) @parent    Match node with specific child
(node_type field: (child)) @node    Match node with named field
```

**IMPORTANT**: You must use capture syntax `@name` to extract matched nodes. Without captures, matches will be found but not returned.

#### Common AST Patterns by Language

**Rust:**
```bash
# Find all functions
rfx query "fn" --ast "(function_item) @fn" --lang rust

# Find all struct definitions
rfx query "struct" --ast "(struct_item) @struct" --lang rust

# Find all enum definitions
rfx query "enum" --ast "(enum_item) @enum" --lang rust

# Find all trait definitions
rfx query "trait" --ast "(trait_item) @trait" --lang rust

# Find all impl blocks
rfx query "impl" --ast "(impl_item) @impl" --lang rust
```

**TypeScript/JavaScript:**
```bash
# Find all function declarations
rfx query "function" --ast "(function_declaration) @fn" --lang typescript

# Find all class declarations
rfx query "class" --ast "(class_declaration) @class" --lang typescript

# Find all interface declarations
rfx query "interface" --ast "(interface_declaration) @interface" --lang typescript

# Find all arrow functions
rfx query "=>" --ast "(arrow_function) @fn" --lang typescript

# Find all method definitions
rfx query "method" --ast "(method_definition) @method" --lang typescript
```

**PHP:**
```bash
# Find all function definitions
rfx query "function" --ast "(function_definition) @fn" --lang php

# Find all class declarations
rfx query "class" --ast "(class_declaration) @class" --lang php

# Find all trait declarations
rfx query "trait" --ast "(trait_declaration) @trait" --lang php

# Find all enum declarations (PHP 8.1+)
rfx query "enum" --ast "(enum_declaration) @enum" --lang php
```

#### Advanced AST Pattern Examples

**Multiple captures:**
```bash
# Find functions and extract the name
rfx query "fn" --ast "(function_item name: (identifier) @name) @function" --lang rust

# Find classes with specific body
rfx query "class" --ast "(class_declaration name: (identifier) @name body: (class_body) @body) @class" --lang typescript
```

**Combining with other filters:**
```bash
# AST query + file filter
rfx query "async" --ast "(function_item (async))" --lang rust --file src/

# AST query + limit results
rfx query "class" --ast "(class_declaration)" --lang typescript --limit 10

# AST query + JSON output for AI agents
rfx query "impl" --ast "(impl_item)" --lang rust --json
```

#### How AST Queries Work

1. **Phase 1 - Trigram Filtering**: Text pattern narrows 10,000+ files → ~10-100 candidates
2. **Phase 2 - AST Matching**: Parse candidate files with Tree-sitter and match AST pattern
3. **Phase 3 - Results**: Return matching code structures with symbol names and spans

**Performance:** AST queries add 2-224ms overhead (parsing only candidate files, not entire codebase)

#### Finding Available Node Types

To discover available AST node types for your language:

1. Visit Tree-sitter playground: https://tree-sitter.github.io/tree-sitter/playground
2. Select your language grammar
3. Paste sample code to see AST structure
4. Use node type names in parentheses: `(node_type)`

**Example node types by language:**

- **Rust**: `function_item`, `struct_item`, `enum_item`, `trait_item`, `impl_item`, `mod_item`, `const_item`, `static_item`
- **TypeScript/JavaScript**: `function_declaration`, `class_declaration`, `interface_declaration`, `arrow_function`, `method_definition`, `variable_declarator`
- **PHP**: `function_definition`, `class_declaration`, `trait_declaration`, `interface_declaration`, `enum_declaration`, `method_declaration`

#### Difference from Symbol Search

| Feature | Symbol Search (`--symbols`) | AST Query (`--ast`) |
|---------|----------------------------|---------------------|
| **Purpose** | Find symbol definitions | Match specific code structures |
| **Filter by** | Symbol kind (function, class, etc.) | AST node patterns |
| **Flexibility** | Predefined kinds only | Any Tree-sitter node pattern |
| **Speed** | Fast (simple symbol extraction) | Slightly slower (full AST matching) |
| **Use case** | "Find all functions" | "Find all async functions with pub modifier" |

### `rfx stats`

Display index statistics.

```bash
rfx stats [OPTIONS]

Options:
  --json    Output as JSON
```

**Example output:**
```
Reflex Index Statistics
-----------------------
Total Files: 1,247
Total Size: 12.4 MB
Cache Size: 2.1 MB
Last Updated: 2025-11-03 14:32:45
Languages: Rust (842), TypeScript (305), Python (100)
```

### `rfx clear`

Clear the search index.

```bash
rfx clear [OPTIONS]

Options:
  --yes, -y    Skip confirmation prompt
```

### `rfx list-files`

List all indexed files.

```bash
rfx list-files [OPTIONS]

Options:
  --json    Output as JSON
```

### `rfx serve`

Start an HTTP API server for programmatic access.

```bash
rfx serve [OPTIONS]

Options:
  --port <PORT>    Port to listen on (default: 7878)
  --host <HOST>    Host to bind to (default: 127.0.0.1)
```

**API Endpoints:**

- **GET /query** - Search the codebase
  - Query params: `q`, `lang`, `kind`, `limit`, `symbols`, `regex`, `exact`, `expand`, `file`, `timeout`
  - Returns: `QueryResponse` JSON with results and index status

- **GET /stats** - Get index statistics
  - Returns: `IndexStats` JSON with file counts, sizes, language breakdowns

- **POST /index** - Trigger reindexing
  - Body: `{"force": boolean, "languages": [string]}`
  - Returns: `IndexStats` JSON after indexing completes

- **GET /health** - Health check
  - Returns: "Reflex is running"

**Example Usage:**
```bash
# Start the server
rfx serve --port 7878

# Query from another terminal (or use in AI tools/editor plugins)
curl 'http://localhost:7878/query?q=QueryEngine&limit=5' | jq '.'

# Get stats
curl http://localhost:7878/stats | jq '.'

# Trigger indexing
curl -X POST http://localhost:7878/index \
  -H "Content-Type: application/json" \
  -d '{"force": false, "languages": ["rust"]}'
```

**Features:**
- CORS enabled for browser clients
- Supports all CLI query options via query parameters
- JSON responses compatible with AI agents and automation tools
- Synchronous indexing (returns after completion)

### `rfx mcp`

Start as an MCP (Model Context Protocol) server for AI coding assistants like Claude Code.

```bash
rfx mcp
```

**What is MCP?**

MCP is an open standard for connecting AI assistants to external tools and data sources. Reflex implements MCP over stdio, allowing AI coding assistants to search your codebase directly.

**Configuration for Claude Code:**

Add to `~/.claude/claude_code_config.json`:

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

1. **`search_code`** - Full-text or symbol search
   - Parameters: `pattern` (required), `lang`, `kind`, `symbols`, `exact`, `file`, `limit`, `expand`
   - Returns: Search results with file paths, line numbers, and context

2. **`search_regex`** - Regex pattern matching with trigram optimization
   - Parameters: `pattern` (required), `lang`, `file`, `limit`
   - Returns: Regex search results

3. **`search_ast`** - Structure-aware AST pattern matching
   - Parameters: `pattern`, `ast_pattern`, `lang` (all required), `file`, `limit`
   - Returns: AST query results

4. **`index_project`** - Trigger reindexing
   - Parameters: `force` (optional), `languages` (optional array)
   - Returns: Index statistics after completion

**Usage in Claude Code:**

Once configured, Claude Code will automatically:
- Spawn `rfx mcp` when the session starts
- Expose Reflex tools for natural language queries
- Handle process lifecycle (start/stop/restart)

Example prompts:
- "Search for all async functions in this project"
- "Find usages of the `parse_tree` function"
- "Show me all struct definitions in Rust files"

**Why stdio MCP?**

- **Zero port conflicts**: No network configuration needed
- **Automatic lifecycle**: Claude Code manages the process
- **Per-session isolation**: Each session gets its own subprocess
- **Crash recovery**: Client automatically respawns on failure
- **Secure**: OS-sandboxed, no network exposure

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

**Note:** Full-text search works on **all file types** regardless of parser support. Symbol filtering requires a language parser.

## 🏗️ Architecture

Reflex uses a **trigram-based inverted index** combined with **runtime symbol detection**:

### Indexing Phase
1. Extract trigrams (3-character substrings) from all files
2. Build inverted index: `trigram → [file_id, line_no]`
3. Store full file contents in memory-mapped `content.bin`
4. No tree-sitter parsing (fast indexing)

### Query Phase
1. **Full-text queries**: Intersect trigram posting lists → verify matches
2. **Symbol queries**: Trigrams narrow to ~10-100 candidates → parse with tree-sitter → filter symbols
3. Memory-mapped I/O for instant cache access

### Cache Structure (`.reflex/`)
```
.reflex/
  meta.db          # SQLite: file metadata, stats, config
  trigrams.bin     # Inverted index (memory-mapped)
  content.bin      # Full file contents (memory-mapped)
  hashes.json      # File hashes for incremental indexing
  config.toml      # Index settings
```

## ⚡ Performance

Reflex is the **fastest structure-aware local code search tool** available:

| Codebase | Files | Full-Text Query | Symbol Query | Regex Query |
|----------|-------|-----------------|--------------|-------------|
| **Reflex** | 50 | 2-3 ms | 2-3 ms | 2-3 ms |
| **Linux Kernel** | 62,000 | 124 ms | 224 ms | 156 ms |
| **Medium Project** | 1,000 | 15-30 ms | 20-40 ms | 25-45 ms |

**Indexing Performance:**
- 100 files: <1 second
- 500 files: <3 seconds
- 1,000 files: <2 seconds (incremental: <1s)

## 🔧 Configuration

Reflex respects `.gitignore` files automatically. Additional configuration via `.reflex/config.toml`:

```toml
# Example configuration (auto-generated on first index)
[indexing]
max_file_size_mb = 10
follow_symlinks = false

[languages]
enabled = ["rust", "typescript", "python", "go", "java", "c", "cpp", "php"]

[cache]
compression = false
```

## 🤖 AI Integration

Reflex outputs clean JSON for AI coding assistants:

```bash
rfx query "parse_tree" --json --symbols
```

**Example JSON output:**
```json
[
  {
    "file": "src/parsers/rust.rs",
    "line": 45,
    "column": 8,
    "symbol": "parse_tree",
    "kind": "Function",
    "language": "Rust",
    "match": "pub fn parse_tree(source: &str) -> Tree {",
    "context_before": ["", "/// Parse Rust source code into AST"],
    "context_after": ["    let mut parser = Parser::new();", ""]
  }
]
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
- **Test Coverage**: Find test files and assertions

### For Teams
- **Code Search**: Local alternative to Sourcegraph
- **Documentation**: Find examples of API usage
- **Onboarding**: Explore unfamiliar codebases
- **Security**: Search for potential vulnerabilities

## 🧪 Testing

Reflex has **221 comprehensive tests**:
- **194 unit tests** (cache, indexer, query, parsers, core modules)
- **17 integration tests** (workflows, multi-language, error handling)
- **10 performance tests** (indexing speed, query latency, scalability)

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_full_workflow
```

## 🤝 Contributing

Contributions welcome! Reflex is built to be:
- **Fast**: Sub-100ms queries on large codebases
- **Accurate**: Complete coverage with deterministic results
- **Extensible**: Easy to add new language parsers

See [ARCHITECTURE.md](ARCHITECTURE.md) for implementation details.

## 📚 Documentation

- **[ARCHITECTURE.md](ARCHITECTURE.md)**: System design, data formats, extension guide
- **[CLAUDE.md](CLAUDE.md)**: Project overview and development workflow
- **[.context/TODO.md](.context/TODO.md)**: Implementation roadmap and task tracking

## 🛣️ Roadmap

### Completed ✅
- [x] Comprehensive testing (221 tests)
- [x] README.md with full documentation
- [x] ARCHITECTURE.md with system design
- [x] Rustdoc comments for all public APIs
- [x] HTTP server for programmatic access
- [x] AST pattern matching (Tree-sitter queries)
- [x] MCP (Model Context Protocol) server for AI agents

### Next Phase: Advanced Features
- [ ] LSP (Language Server Protocol) adapter
- [ ] Background indexing daemon (`reflexd`)
- [ ] Graph queries (imports/exports, call graph)

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
