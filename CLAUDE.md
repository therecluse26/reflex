# CLAUDE.md

## Project Overview
**Reflex** is a local-first, full-text code search engine written in Rust. It's a fast, deterministic replacement for Sourcegraph Code Search, designed specifically for AI coding workflows and automation.

Reflex uses **trigram-based indexing** to enable sub-100ms full-text search across large codebases (10k+ files). Unlike symbol-only tools, Reflex finds **every occurrence** of patterns—function calls, variable usage, comments, and more—not just definitions. Results include file paths, line numbers, and surrounding context, with optional symbol-aware filtering.

---

## Core Principles
1. **Local-first**: Runs fully offline; all data stays on the developer's machine
2. **Complete coverage**: Finds every occurrence, not just symbol definitions
3. **Deterministic results**: Same query → same answer; no probabilistic ranking
4. **Instant access**: Trigram index + memory-mapping enables sub-100ms queries
5. **Agent-oriented**: Clean JSON output built for AI coding agents and automation
6. **Regex support**: Extract trigrams from patterns for fast regex search

---

## Architecture Overview

### Components
| Module | Description |
| --- | --- |
| **Trigram Indexer** | Extracts trigrams from all code files; builds inverted index (trigram → file locations) |
| **Content Store** | Stores full file contents (memory-mapped); enables context extraction around matches |
| **Query Engine** | Intersects trigram posting lists; verifies matches; returns line-by-line results with context |
| **Runtime Symbol Parser** | Uses Tree-sitter to parse candidate files at query time (only files matching trigrams) |
| **CLI / API Layer** | Single binary for human and programmatic use (CLI and optional HTTP/MCP) |
| **Watcher (optional)** | Incrementally updates index on file changes |

### Index Cache Structure (`.reflex/`)
    .reflex/
      meta.db          # SQLite: file metadata, stats, config
      trigrams.bin     # Inverted index: trigram → [file_id, line_no] posting lists
      content.bin      # Memory-mapped full file contents for context extraction
      config.toml      # Index settings (languages, filters, ignore rules)

---

## CLI Usage

    # Build or update the local cache
    rfx index

    # Full-text search (default - finds all occurrences)
    rfx query "extract_symbols"
    → Finds function definition + all call sites (11 total)

    # Filter to symbol definitions only (uses runtime tree-sitter parsing)
    rfx query "extract_symbols" --symbols
    → Finds only the function definition (1 result)

    # Full-text search with language filter
    rfx query "unwrap" --lang rust

    # Export results as JSON (for AI agents)
    rfx query "format!" --json

    # Watch for file changes and auto-reindex (new!)
    rfx watch                    # 15s debounce (default)
    rfx watch --debounce 20000   # 20s debounce
    rfx watch --quiet            # Suppress output

    # Serve a local HTTP API (optional)
    rfx serve --port 7878

---

## Supported Languages & Frameworks

Reflex currently supports symbol extraction for the following languages and frameworks:

### Fully Supported (Tree-sitter parsers implemented)

| Language/Framework | Extensions | Symbol Extraction | Notes |
|-------------------|------------|------------------|-------|
| **Rust** | `.rs` | Functions, structs, enums, traits, impls, modules, methods, constants, type aliases | Complete Rust support |
| **Python** | `.py` | Functions, classes, methods, constants, lambdas, decorators | Full Python support including async/await |
| **TypeScript** | `.ts`, `.tsx`, `.mts`, `.cts` | Functions, classes, interfaces, types, enums, methods | Full TypeScript + JSX support |
| **JavaScript** | `.js`, `.jsx`, `.mjs`, `.cjs` | Functions, classes, constants, methods | Includes React/JSX support via TSX grammar |
| **Go** | `.go` | Functions, structs, interfaces, methods, constants, variables, packages | Full Go support |
| **Java** | `.java` | Classes, interfaces, enums, methods, fields, constructors, annotations | Full Java support including generics |
| **C** | `.c`, `.h` | Functions, structs, enums, unions, typedefs, global variables, macros | Complete C support |
| **C++** | `.cpp`, `.cc`, `.cxx`, `.hpp`, `.hxx`, `.C`, `.H` | Functions, classes, structs, namespaces, templates, methods, type aliases | Full C++ support including templates |
| **C#** | `.cs` | Classes, interfaces, structs, enums, records, delegates, methods, properties, namespaces | Full C# support (C# 1-13) |
| **PHP** | `.php` | Functions, classes, interfaces, traits, methods, properties, constants, namespaces, enums | Full PHP support including PHP 8.1+ enums |
| **Ruby** | `.rb`, `.rake`, `.gemspec` | Classes, modules, methods, singleton methods, constants, blocks | Full Ruby support including Rails patterns |
| **Kotlin** | `.kt`, `.kts` | Classes, objects, interfaces, functions, properties, data classes, sealed classes | Full Kotlin support including Android development |
| **Zig** | `.zig` | Functions, structs, enums, constants, variables, tests, error sets | Full Zig support |
| **~~Swift~~** | `.swift` | ~~Classes, structs, enums, protocols, functions, extensions, properties, actors~~ | **Temporarily disabled** - requires tree-sitter 0.23 (Reflex uses 0.24) |
| **Vue** | `.vue` | Functions, constants, methods from `<script>` blocks | Supports both Options API and Composition API |
| **Svelte** | `.svelte` | Functions, variables, reactive declarations (`$:`), module context | Full Svelte component support |

### React/JSX Support Details
- **React Components**: Function and class components automatically detected
- **Hooks**: Custom hooks extracted as functions (e.g., `useCounter`)
- **TypeScript + JSX**: Full support for `.tsx` files with type annotations
- **Interfaces & Types**: Props interfaces and type definitions extracted

### Vue Support Details
- **Script Blocks**: Extracts symbols from all `<script>` sections
- **Composition API**: Full support for `<script setup>` syntax
- **TypeScript**: Supports `<script lang="ts">` and `<script setup lang="ts">`
- **Parsing Method**: Line-based extraction (tree-sitter-vue incompatible with tree-sitter 0.24+)

### Svelte Support Details
- **Component Scripts**: Extracts from both regular and `context="module"` scripts
- **Reactive Declarations**: Tracks `$:` reactive statements
- **TypeScript**: Supports `<script lang="ts">`
- **Parsing Method**: Line-based extraction (tree-sitter-svelte incompatible with tree-sitter 0.24+)

### PHP Support Details
- **Functions**: Global function definitions
- **Classes**: Regular, abstract, and final classes
- **Interfaces**: Interface declarations
- **Traits**: PHP trait definitions and usage
- **Methods**: With class/trait/interface scope tracking
- **Properties**: Public, protected, private visibility
- **Constants**: Class constants and global constants
- **Namespaces**: Full namespace support
- **Enums**: PHP 8.1+ enum declarations

**Coverage**: Reflex supports **90%+ of all codebases** across web, mobile, systems, enterprise, and AI/ML development (18 languages: Rust, Python, TypeScript, JavaScript, Go, Java, C, C++, C#, PHP, Ruby, Kotlin, Zig, Vue, Svelte, plus experimental Swift support once tree-sitter compatibility is resolved).

**Note on Swift**: Swift support is temporarily disabled due to tree-sitter version incompatibility. The tree-sitter-swift grammar requires tree-sitter 0.23, while Reflex uses tree-sitter 0.24 for better performance and compatibility with other languages. Swift support will be restored when the grammar is updated to 0.24+.

**Note**: Full-text trigram search works for **all file types** regardless of parser support. Symbol filtering (`symbol:` queries) requires a language parser.

---

## Tech Stack
- **Language**: Rust (Edition 2024)
- **Core Algorithm**: Trigram-based inverted index (inspired by Zoekt/Google Code Search)
- **Crates**:
  - **Indexing**: Custom trigram extraction, `memmap2` (zero-copy I/O)
  - **Parsing**: `tree-sitter` + language grammars (runtime symbol parsing at query time)
  - **Storage**: `rusqlite` (metadata), custom binary format (trigrams + content)
  - **Incremental**: `blake3` (content hashing), `ignore` (gitignore support)
  - **Performance**: `rayon` (parallel indexing), memory-mapped I/O
  - **CLI**: `clap` (argument parsing), `serde_json` (JSON output)

---

## Development Workflow

### Build
    cargo build --release

### Test
    cargo test

### Refresh Index
    rfx index

### Debug Queries
    RUST_LOG=debug rfx query "fn main"

---

## Runtime Symbol Detection Architecture

Reflex uses a unique **runtime symbol detection** approach that combines the speed of trigram indexing with the precision of tree-sitter parsing:

### How It Works

1. **Indexing Phase** (no tree-sitter parsing):
   - Extract trigrams from all files → build inverted index
   - Store full file contents in memory-mapped content.bin
   - No symbol extraction or tree-sitter parsing during indexing

2. **Query Phase** (lazy parsing only when needed):
   - **Full-text queries**: Use trigrams only (instant results)
   - **Symbol queries** (`--symbols` or `--kind function`):
     1. Trigram search narrows 62K files → ~10-100 candidates
     2. Parse only candidate files with tree-sitter (2-224ms overhead)
     3. Filter to symbol definitions and return results

### Performance Benefits

| Approach | Indexing Time | Query Time | Memory Usage |
|----------|---------------|------------|--------------|
| **Old (indexed symbols)** | Slow (parse all files) | 4125ms (load 3.3M symbols) | High (symbols.bin) |
| **New (runtime parsing)** | Fast (trigrams only) | 2-224ms (parse 10 files) | Low (no symbols.bin) |

**Improvement**: 2000x faster on small codebases (4125ms → 2ms), 18x faster on Linux kernel (4125ms → 224ms)

### Why This Works

- **Trigrams are excellent filters**: Reduce search space by 100-1000x
- **Most queries are full-text**: Symbol filtering is the minority case
- **Parsing is fast**: Tree-sitter parses 10 files in ~2ms
- **Lazy evaluation wins**: Parse only what's needed, when it's needed

### Architecture Simplification

Removed components:
- `symbols.bin` (entire symbol storage file)
- `SymbolWriter` (~250 lines of serialization code)
- `SymbolReader` (~250 lines of deserialization code)

Result: **Simpler, faster, smaller cache, more flexible symbol filtering**

---

## Design Notes
- **Trigram Algorithm**: Extracts 3-character substrings; builds inverted index for O(1) lookups
- **Runtime Symbol Detection**: Parse only candidate files at query time (10-100 files vs 62K+ files at index time)
- **Incremental by content**: Files reindexed only if `blake3` hash changes
- **Memory-mapped I/O**: Zero-copy access to trigrams.bin and content.bin
- **Regex support**: Extracts guaranteed trigrams from patterns; falls back to full scan if needed
- **Deterministic**: Same query always returns same results (sorted by file:line)
- **Respects .gitignore**: Uses `ignore` crate to skip untracked files
- **Programmatic output**: Line-based results with context:
  ```json
  {
    "file": "src/parsers/rust.rs",
    "line": 67,
    "column": 12,
    "match": "extract_symbols(source, root, &query, ...)",
    "context_before": ["    symbols.extend(extract_functions(...", ""],
    "context_after": ["    symbols.extend(extract_structs(...", ""]
  }
  ```

---

## MVP Goals
1. **<100ms per query** on 10k+ files (trigram index reduces search space 100-1000x) ✅
2. **Complete coverage**: Find every occurrence of patterns, not just definitions ✅
3. **Deterministic results**: Same query → same results (sorted by file:line) ✅
4. **Fully offline**: No daemon; per-query invocation with memory-mapped cache ✅
5. **Clean JSON API**: Structured output for AI agents and editor integrations ✅
6. **Symbol filtering**: Runtime tree-sitter parsing on candidate files (2-224ms overhead) ✅
7. **Regex support**: Extract trigrams from regex for fast pattern matching ✅
8. **Incremental indexing**: Only reindex changed files (blake3 hashing) ✅

### Performance Benchmarks (Linux Kernel - 62K files)
- **Full-text search**: 124ms
- **Regex search**: 156ms
- **Symbol search**: 224ms (runtime parsing of ~3 candidate C files)
- **Reflex codebase** (small): 2-3ms for all query types

**Result**: Reflex is the **fastest structure-aware local code search tool** available.

---

## Future Work
- ✅ **File watcher** (`rfx watch`): Auto-reindex on file changes with configurable debouncing - **COMPLETED (2025-11-03)**
- ✅ **MCP server** (`rfx mcp`): Model Context Protocol server for AI agents like Claude Code - **COMPLETED (2025-11-03)**
- ✅ **AST pattern matching** (`--ast` flag): Structure-aware code search using Tree-sitter S-expressions - **COMPLETED (2025-11-03)**
- ✅ **HTTP server** (`rfx serve`): REST API for programmatic access - **COMPLETED (2025-11-03)**
- Interactive mode for exploratory workflows
- Semantic query building (natural language → Reflex commands via tiny local LLMs)
- Graph queries (imports/exports, limited call graph)
- Branch-aware context diffing and filters (e.g., `--since`, `--branch`)
- Binary protocol for ultra-low-latency local queries

---

## Repository Conventions
- Source: `src/`
- Core library: `src/lib.rs`
- CLI entrypoint: `src/main.rs`
- Tests: `tests/`
- Local cache/config: `.reflex/` (added to `.gitignore`)
- Context/planning: `.context/` (tracked in git)

---

## Context Management & AI Workflow

### `.context/` Directory Structure

The `.context/` directory contains planning documents, research notes, and decision logs to maintain context across development sessions. **All AI assistants working on Reflex must actively use and update these files.**

#### Required Files

**`.context/TODO.md`** - Primary task tracking and implementation roadmap
- **MUST be consulted** at the start of every development session
- **MUST be updated** when:
  - Starting work on a task (mark as `in_progress`)
  - Completing a task (mark as `completed`)
  - Discovering new tasks or requirements
  - Making architectural decisions that affect the roadmap
  - Changing priorities or timelines
- Contains:
  - MVP goals and success criteria
  - Task breakdown by module with priority levels (P0/P1/P2/P3)
  - Implementation phases and timeline
  - Open questions and design decisions
  - Performance targets and benchmarks
  - Maintenance strategy and update policy

#### Optional Research Files

Create RESEARCH.md files as needed to cache important findings:

**`.context/TREE_SITTER_RESEARCH.md`** - Tree-sitter grammar investigation
- Document findings about each language grammar
- Node types and AST structure for symbol extraction
- Query patterns and examples
- Quirks, gotchas, and edge cases
- Version compatibility notes

**`.context/PERFORMANCE_RESEARCH.md`** - Optimization findings
- Benchmarking results and bottleneck analysis
- Memory-mapping techniques and best practices
- Indexing speed optimizations
- Query latency improvements
- Cache format trade-offs

**`.context/BINARY_FORMAT_RESEARCH.md`** - Data serialization decisions
- Binary format design rationale
- Alternatives considered and rejected
- Serialization library comparisons (bincode, rkyv, custom)
- Versioning and migration strategies

**`.context/LANGUAGE_SPECIFIC_NOTES.md`** - Per-language implementation details
- Language-specific symbol extraction challenges
- Parser implementation patterns
- Testing strategies for each language
- Real-world codebase findings

### AI Assistant Workflow

When working on Reflex, AI assistants should:

1. **Start Every Session:**
   - Read `CLAUDE.md` for project overview
   - Read `.context/TODO.md` to understand current state
   - Identify which tasks are blocked, in progress, or ready to start

2. **During Development:**
   - Update `.context/TODO.md` task statuses in real-time
   - Create/update RESEARCH.md files when conducting investigations
   - Document decisions and rationale inline
   - Add new tasks as they're discovered

3. **Before Ending Session:**
   - Ensure all task statuses are accurate
   - Document any blocking issues or open questions
   - Update implementation notes if approach changed
   - Commit research findings to appropriate RESEARCH.md files

4. **When Conducting Research:**
   - Create focused RESEARCH.md files rather than losing findings
   - Include code examples, links, and specific version numbers
   - Note what was tried and why it didn't work (avoid repeated dead ends)
   - Cross-reference related TODO.md tasks

5. **Decision Documentation:**
   - Major decisions go in `.context/TODO.md` under "Notes & Design Decisions"
   - Technical deep-dives go in specific RESEARCH.md files
   - Quick notes and TODOs stay in source code comments

### Example: Starting a New Language Parser

```bash
# 1. Check TODO.md for the task
# 2. Create research file
touch .context/RUST_PARSER_RESEARCH.md

# 3. Document investigation
# - Examine tree-sitter-rust grammar
# - List all node types for symbols
# - Create example AST traversal code
# - Note edge cases (macros, proc macros, etc.)

# 4. Update TODO.md
# - Mark parser task as in_progress
# - Add any new subtasks discovered
# - Document key decisions

# 5. Implement based on research
# 6. Update TODO.md to completed
# 7. Reference RESEARCH.md in code comments
```

### Context Preservation Goals

The `.context/` directory enables:
- **Session continuity:** Pick up where previous work left off
- **Decision tracking:** Understand why choices were made
- **Avoiding rework:** Don't re-research solved problems
- **Onboarding:** New contributors understand the project state
- **AI handoff:** Different AI assistants can collaborate effectively

---

## Project Philosophy
Reflex favors local autonomy, speed, and clarity.

- Fast enough to call multiple times per agent step.
- Deterministic for repeatable reasoning.
- Simple to rebuild: delete `.reflex/` and re-index at any time.

> "Understand your code the way your compiler does — instantly."

---

## Release Management

Reflex follows **semantic versioning** (SemVer) and uses automated release tooling for changelog generation and version bumping.

### Semantic Versioning

Version format: `MAJOR.MINOR.PATCH` (e.g., `1.2.3`)

- **MAJOR**: Breaking changes (incompatible API changes)
- **MINOR**: New features (backward-compatible functionality)
- **PATCH**: Bug fixes (backward-compatible bug fixes)

**Examples:**
- `1.0.0 → 1.0.1`: Bug fix (PATCH bump)
- `1.0.1 → 1.1.0`: New feature like `--timeout` flag (MINOR bump)
- `1.1.0 → 2.0.0`: Breaking change like removing HTTP API (MAJOR bump)

### Conventional Commits

Reflex uses **Conventional Commits** for automatic changelog generation and version bumping:

```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

**Types:**
- `feat:` - New feature (triggers MINOR version bump)
- `fix:` - Bug fix (triggers PATCH version bump)
- `docs:` - Documentation changes only
- `refactor:` - Code refactoring (no functional changes)
- `perf:` - Performance improvements
- `test:` - Adding or updating tests
- `chore:` - Maintenance tasks (dependencies, build, etc.)
- `BREAKING CHANGE:` - Breaking change (triggers MAJOR version bump)

**Examples:**
```bash
# Feature: Adds timeout support (1.0.0 → 1.1.0)
feat(query): add --timeout flag for query timeout control

Adds configurable query timeout with default of 30 seconds.
Users can override with --timeout flag or set to 0 for no limit.

# Bug fix: Fixes crash (1.1.0 → 1.1.1)
fix(indexer): handle empty files without panic

# Breaking change: Removes deprecated API (1.1.1 → 2.0.0)
feat(api): remove deprecated /search endpoint

BREAKING CHANGE: The /search endpoint has been removed.
Use /query instead.
```

### Release Automation with release-plz

Reflex uses **[release-plz](https://release-plz.ieni.dev/)** for automated releases:

1. **On every commit to `main`:**
   - Analyzes commits since last release
   - Determines next version based on conventional commits
   - Updates `Cargo.toml` version
   - Generates/updates `CHANGELOG.md`
   - Opens a PR with these changes (title: "chore: release")

2. **When the release PR is merged:**
   - Creates a git tag (e.g., `v1.1.0`)
   - Publishes a GitHub Release with changelog notes
   - Optionally publishes to crates.io (if configured)

**Workflow:**
```bash
# 1. Make changes and commit with conventional commit message
git add .
git commit -m "feat(query): add regex support for pattern matching"
git push origin main

# 2. release-plz GitHub Action runs automatically
# - Opens PR: "chore: release v1.1.0"
# - Contains: Cargo.toml version bump + CHANGELOG.md update

# 3. Review and merge the release PR
# 4. GitHub Action creates tag and GitHub Release automatically
```

### Commit Linting with cocogitto

**[Cocogitto](https://github.com/cocogitto/cocogitto)** validates conventional commit messages:

```bash
# Install cocogitto (optional, for local validation)
cargo install cocogitto

# Validate commits locally
cog check

# Generate changelog preview
cog changelog

# Create a conventional commit interactively
cog commit
```

**Pre-commit hook (optional):**
```bash
# .git/hooks/commit-msg
#!/bin/sh
cog verify --file $1
```

### GitHub Actions Workflow

**`.github/workflows/release-plz.yml`** - Automated release management

Triggers:
- On push to `main` branch
- On demand via workflow_dispatch

Actions:
1. Checkout code
2. Run release-plz to analyze commits
3. Create release PR if changes detected
4. On PR merge: create tag and GitHub Release

**Configuration:** `.github/release-plz.toml`

### Manual Release Process (Fallback)

If automated releases are not available:

```bash
# 1. Update version in Cargo.toml
vim Cargo.toml  # Change version = "1.0.0" to "1.1.0"

# 2. Update CHANGELOG.md manually
vim CHANGELOG.md  # Add new section for v1.1.0

# 3. Commit the version bump
git add Cargo.toml CHANGELOG.md
git commit -m "chore: release v1.1.0"

# 4. Create git tag
git tag -a v1.1.0 -m "Release v1.1.0"

# 5. Push changes and tag
git push origin main
git push origin v1.1.0

# 6. Create GitHub Release manually
# - Go to https://github.com/therecluse26/reflex/releases/new
# - Select tag: v1.1.0
# - Copy changelog content into release notes
# - Publish release
```

### CHANGELOG.md Format

```markdown
# Changelog

All notable changes to Reflex will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [1.1.0] - 2025-11-03

### Added
- Query timeout support with `--timeout` flag
- HTTP API timeout parameter

### Fixed
- Handle empty files without panicking

## [1.0.0] - 2025-11-01

### Added
- Initial release
- Trigram-based full-text search
- Symbol-aware filtering
- Multi-language support
```

### Binary Distribution (Future)

**cargo-dist** for cross-platform binary releases:

```bash
# Install cargo-dist
cargo install cargo-dist

# Generate release workflow
cargo dist init

# Build binaries for all targets
cargo dist build
```

**Targets:**
- `x86_64-unknown-linux-gnu` (Linux x64)
- `x86_64-unknown-linux-musl` (Linux x64 static)
- `x86_64-apple-darwin` (macOS Intel)
- `aarch64-apple-darwin` (macOS ARM)
- `x86_64-pc-windows-msvc` (Windows x64)

**Install script:**
```bash
curl -fsSL https://raw.githubusercontent.com/therecluse26/reflex/main/install.sh | sh
```

### Release Checklist

Before creating a release:

- [ ] All tests pass (`cargo test`)
- [ ] Documentation is up to date (README.md, ARCHITECTURE.md, CLAUDE.md)
- [ ] CHANGELOG.md includes all changes
- [ ] Version in Cargo.toml is correct
- [ ] Git tag matches Cargo.toml version
- [ ] GitHub Release notes are complete
- [ ] (Optional) Binaries are built for all targets
- [ ] (Optional) crates.io publication successful

### First Release (v1.0.0)

For the initial 1.0.0 release:

1. Ensure all MVP goals are completed (see above)
2. Complete comprehensive testing (334 tests passing)
3. Finalize documentation (README.md, ARCHITECTURE.md, CLAUDE.md)
4. Set up release-plz and cocogitto
5. Create CHANGELOG.md with v1.0.0 notes
6. Tag v1.0.0 and create GitHub Release
7. Announce on relevant channels (Reddit, HN, Discord, etc.)
