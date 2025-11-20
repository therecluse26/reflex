# Reflex

**Local-first, full-text code search engine for AI coding workflows**

Reflex is a blazingly fast, trigram-based code search engine designed for developers and AI coding assistants. Unlike symbol-only tools, Reflex finds **every occurrence** of patterns—function calls, variable usage, comments, and more.

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)]()
[![Tests](https://img.shields.io/badge/tests-347%20passing-brightgreen)]()
[![License](https://img.shields.io/badge/license-MIT-blue)]()

## ✨ Features

- **🔍 Complete Coverage**: Find every occurrence, not just symbol definitions
- **⚡ Blazing Fast**: Lightning-fast queries via trigram indexing
- **🎯 Symbol-Aware**: Runtime tree-sitter parsing for precise symbol filtering
- **🔄 Incremental**: Only reindexes changed files (blake3 hashing)
- **🌍 Multi-Language**: Rust, TypeScript/JavaScript, Vue, Svelte, PHP, Python, Go, Java, C, C++, C#, Ruby, Kotlin, Zig
- **🤖 AI Query Assistant**: Natural language search with `rfx ask` (OpenAI, Anthropic, Groq)
- **🌐 HTTP API**: REST API for editor plugins and external tools
- **📡 MCP Support**: Model Context Protocol server for AI assistants
- **📦 Local-First**: Fully offline, all data stays on your machine
- **🎨 Regex Support**: Trigram-optimized regex search
- **🌳 AST Queries**: Structure-aware search with Tree-sitter
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

## 🤖 AI Query Assistant

Don't want to remember search syntax? Use `rfx ask` to translate natural language questions into `rfx query` commands.

### Setup

First-time setup requires configuring an AI provider (OpenAI, Anthropic, or Groq):

```bash
# Interactive configuration wizard (recommended)
rfx ask --configure
```

This will guide you through:
- Selecting an AI provider
- Entering your API key
- Choosing a model (optional)

Configuration is saved to `~/.reflex/config.toml`:

```toml
[semantic]
provider = "openai"  # or anthropic, groq

[credentials]
openai_api_key = "sk-..."
openai_model = "gpt-4o-mini"  # optional
```

Alternatively, set environment variables:
```bash
export OPENAI_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-ant-..."
export GROQ_API_KEY="gsk_..."
```

### Usage

```bash
# Ask a question (generates and optionally executes rfx query commands)
rfx ask "Find all TODOs in Rust files"

# Auto-execute without confirmation
rfx ask "Where is the main function defined?" --execute

# Use a specific provider
rfx ask "Show me error handling code" --provider groq

# Interactive chat mode with conversation history
rfx ask --interactive

# Agentic mode (multi-step reasoning with automatic context gathering)
rfx ask "How does authentication work?" --agentic --answer

# Get a conversational answer based on search results
rfx ask "What does the indexer module do?" --answer
```

**How it works:**
1. Your natural language question is sent to an LLM
2. The LLM generates one or more `rfx query` commands
3. You review and confirm (or use `--execute` to auto-run)
4. Results are displayed as normal search output

**Agentic mode** (`--agentic`) enables multi-step reasoning where the LLM can:
- Gather context by running multiple searches
- Refine queries based on initial results
- Iteratively explore the codebase
- Generate comprehensive answers with `--answer`

## 📋 Command Reference

### `rfx index`

Build or update the search index.

```bash
rfx index [OPTIONS]

Options:
  --force              Force full reindex (ignore incremental)
  --languages <LANGS>  Limit to specific languages (comma-separated)

Subcommands:
  status               Show background symbol indexing status
  compact              Compact cache (remove deleted files, reclaim space)
```

### `rfx query`

Search the codebase. Run `rfx query --help` for full options.

**Key Options:**
- `--symbols, -s` - Symbol-only search (definitions, not usage)
- `--regex, -r` - Treat pattern as regex
- `--lang <LANG>` - Filter by language
- `--kind <KIND>` - Filter by symbol kind (function, class, struct, etc.)
- `--dependencies` - Include dependency information (supports: Rust, TypeScript, JavaScript, Python, Go, Java, C, C++, C#, PHP, Ruby, Kotlin)
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
14. **`analyze_summary`** - Get dependency analysis summary (counts only)

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

# Get JSON summary of all analyses
rfx analyze --json

# Get pretty-printed JSON summary
rfx analyze --json --pretty

# Paginate results
rfx analyze --hotspots --limit 50 --offset 0  # First 50
rfx analyze --hotspots --limit 50 --offset 50 # Next 50

# Export as JSON with pagination metadata
rfx analyze --circular --json
```

**JSON Output Format (specific analyses with pagination):**
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

**Summary JSON Output Format (bare `rfx analyze --json`):**
```json
{
  "circular_dependencies": 17,
  "hotspots": 10,
  "unused_files": 82,
  "islands": 81,
  "min_dependents": 2
}
```

### `rfx deps`

Analyze dependencies for a specific file. Shows what a file imports (dependencies) or what imports it (dependents).

**Key Options:**
- `--reverse` - Show files that depend on this file (reverse lookup)
- `--depth N` - Traverse N levels deep for transitive dependencies (default: 1)
- `--format` - Output format: tree, table, json (default: tree)
- `--json` - Output as JSON
- `--pretty` - Pretty-print JSON output

**Examples:**
```bash
# Show direct dependencies
rfx deps src/main.rs

# Show files that import this file (reverse lookup)
rfx deps src/config.rs --reverse

# Show transitive dependencies (depth 3)
rfx deps src/api.rs --depth 3

# JSON output
rfx deps src/main.rs --json

# Pretty-printed JSON
rfx deps src/main.rs --json --pretty

# Table format
rfx deps src/main.rs --format table
```

**Supported Languages:** Rust, TypeScript, JavaScript, Python, Go, Java, C, C++, C#, PHP, Ruby, Kotlin

**Note:** Only static imports (string literals) are tracked. Dynamic imports are filtered by design. See [CLAUDE.md](CLAUDE.md) for details.

### `rfx ask`

Translate natural language questions into `rfx query` commands using AI.

**Setup:**
```bash
# First-time setup: interactive configuration wizard
rfx ask --configure
```

**Key Options:**
- `--configure` - Launch interactive setup wizard for API keys
- `--execute, -e` - Auto-execute generated queries without confirmation
- `--provider <PROVIDER>` - Override configured provider (openai, anthropic, groq)
- `--interactive, -i` - Launch interactive chat mode with conversation history
- `--agentic` - Enable multi-step reasoning with automatic context gathering
- `--answer` - Generate conversational answer based on search results
- `--json` - Output as JSON
- `--debug` - Show full LLM prompts and retain terminal history

**Examples:**
```bash
# Interactive setup
rfx ask --configure

# Simple query (reviews generated commands before executing)
rfx ask "Find all TODOs in Rust files"

# Auto-execute without confirmation
rfx ask "Where is the main function defined?" --execute

# Interactive chat mode
rfx ask --interactive

# Agentic mode with answer generation
rfx ask "How does the indexer work?" --agentic --answer

# Use specific provider
rfx ask "Show error handling" --provider groq
```

**See also:** The [AI Query Assistant](#-ai-query-assistant) section for detailed setup and usage information.

### `rfx context`

Generate codebase context for AI prompts. Useful with `rfx ask --additional-context`.

**Key Options:**
- `--structure` - Show directory structure (enabled by default)
- `--file-types` - Show file type distribution (enabled by default)
- `--project-type` - Detect project type (CLI/library/webapp/monorepo)
- `--framework` - Detect frameworks and conventions
- `--entry-points` - Show entry point files
- `--test-layout` - Show test organization pattern
- `--config-files` - List important configuration files
- `--full` - Enable all context types
- `--path <PATH>` - Focus on specific directory
- `--depth <N>` - Tree depth for structure (default: 1)

**Examples:**
```bash
# Basic overview (structure + file types)
rfx context

# Full context for monorepo subdirectory
rfx context --path services/backend --full

# Specific context types
rfx context --framework --entry-points

# Use with semantic queries
rfx ask "find auth code" --additional-context "$(rfx context --framework)"
```

### Other Commands

- `rfx stats` - Display index statistics
- `rfx clear` - Clear the search index
- `rfx list-files` - List all indexed files
- `rfx watch` - Watch for file changes and auto-reindex

Run `rfx <command> --help` for detailed options.

## 🌳 AST Pattern Matching

Reflex supports **structure-aware code search** using Tree-sitter AST queries.

**⚠️ WARNING:** AST queries are **SLOW** and scan the entire codebase. **Use `--symbols` instead for 95% of cases** (much faster).

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

Reflex is designed for speed at every level:

**Query Performance:**
- **Full-text & Regex**: Lightning-fast queries via trigram indexing
- **Symbol queries**: Slower due to runtime tree-sitter parsing, but still efficient
- **Cached queries**: Near-instant for repeated searches
- Scales well from small projects to large codebases (10k+ files)

**Indexing Performance:**
- **Initial indexing**: Fast parallel processing using 80% of CPU cores
- **Incremental updates**: Only reindexes changed files via blake3 hashing
- **Memory-mapped I/O**: Zero-copy access for instant cache reads

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

Reflex provides clean JSON output for AI coding assistants and automation:

```bash
rfx query "parse_tree" --json --symbols
```

Output includes file paths, line numbers, symbol types, and code previews with pagination metadata.

## 🔍 Use Cases

- **Code Navigation**: Find all usages of functions, classes, and variables
- **Refactoring**: Identify all call sites before making changes
- **AI Assistants**: Retrieve relevant code snippets and context for LLMs
- **Debugging**: Locate where variables and functions are used
- **Documentation**: Find examples of API usage across the codebase
- **Security**: Search for potential vulnerabilities or anti-patterns

## 🧪 Testing

Reflex has **347 comprehensive tests** covering core modules, real-world code samples across all supported languages, and end-to-end workflows.

```bash
cargo test                    # Run all tests
cargo test -- --nocapture     # Run with output
cargo test indexer::tests     # Run specific module
```

See [TESTING.md](docs/TESTING.md) for details.

## 🤝 Contributing

Contributions welcome! Reflex is built to be:
- **Fast**: Lightning-fast queries on large codebases
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
