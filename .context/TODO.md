# RefLex TODO

**Last Updated:** 2025-10-31
**Project Status:** Architecture Redesign - Trigram-Based Full-Text Search

> **⚠️ AI Assistants:** Read the "Context Management & AI Workflow" section in `CLAUDE.md` for instructions on maintaining this file and creating RESEARCH.md documents. This TODO.md MUST be updated as you work on tasks.

---

## 🔄 MAJOR ARCHITECTURAL DECISION (2025-10-31)

**Decision:** RefLex is being redesigned from a **symbol-only index** to a **trigram-based full-text code search engine** (like Sourcegraph/Zoekt).

**Rationale:**
- User requirement: "Local, fast replacement for Sourcegraph Code Search for AI workflows"
- Symbol-only approach was incomplete: `query "extract_symbols"` found 1/8 occurrences (12.5% recall)
- Full-text search needed to find function calls, variable usage, comments, etc.
- Trigram indexing enables <100ms queries on 10k+ files with complete coverage

**Key Changes:**
1. **Indexing**: Extract trigrams from all file content (not just symbol snippets)
2. **Storage**: `trigrams.bin` (inverted index) + `content.bin` (full file contents)
3. **Querying**: Intersect trigram posting lists → verify matches → return line-based results
4. **Symbol filter**: Keep Tree-sitter integration for `symbol:` prefix queries
5. **Regex support**: Extract trigrams from patterns; fall back to full scan when needed

**Implementation Status:** Documentation updated (CLAUDE.md), implementation in progress

**See:** `.context/TRIGRAM_RESEARCH.md` for technical details

---

## 🎯 Current Status Summary (Updated: 2025-11-01)

### ✅ FULLY FUNCTIONAL
RefLex is **operational as a local code search engine** with the following capabilities:

**Working Features:**
- ✅ Full-text trigram-based search (finds ALL occurrences of patterns)
- ✅ Symbol-only search (functions, structs, enums, traits, etc. for Rust)
- ✅ Incremental indexing (only reindexes changed files)
- ✅ Memory-mapped I/O for fast cache access
- ✅ CLI with rich filtering (--lang, --kind, --file, --expand, --exact, --symbols)
- ✅ JSON output for AI/automation consumption
- ✅ .gitignore support (uses `ignore` crate)
- ✅ SQLite metadata tracking
- ✅ Statistics and cache management commands

**Supported Languages (for indexing):**
- ✅ **Rust** - Full symbol extraction (functions, structs, enums, traits, impls, methods, constants, modules, type aliases)
- ⚠️ **Python, JavaScript, TypeScript, Go, Java, PHP, C, C++** - Grammars loaded, parsers stubbed (ready to implement)

**What Works:**
```bash
# Index current directory
reflex index

# Full-text search (finds all occurrences)
reflex query "extract_symbols"  # Finds: definitions + call sites

# Symbol-only search (definitions only)
reflex query "parse" --symbols --kind function

# With filters
reflex query "unwrap" --lang rust --limit 10 --json
```

### ⚠️ LIMITATIONS / TODO

**Known Issues:**
1. **Trigram index not persisted** - Rebuilt on each query (TODO: serialize to trigrams.bin)
2. **Only Rust parser implemented** - Other languages need parser implementations
3. **HTTP server not implemented** - CLI works, serve command is stub only
4. **AST pattern matching not implemented** - Framework exists but not functional

**Performance Note:**
- Queries are fast for symbol-only search (memory-mapped symbols.bin)
- Full-text search rebuilds trigram index on each query (still fast but could be faster)

### 📊 Implementation Progress

| Component | Status | Completeness |
|-----------|--------|--------------|
| **Core Infrastructure** | ✅ Complete | 100% |
| **Cache System** | ✅ Complete | 100% |
| **Indexer** | ✅ Complete | 100% |
| **Query Engine** | ✅ Complete | 95% (AST patterns missing) |
| **Trigram Search** | ✅ Complete | 90% (not persisted) |
| **Content Store** | ✅ Complete | 100% |
| **Symbol Storage** | ✅ Complete | 100% |
| **Rust Parser** | ✅ Complete | 100% |
| **Other Parsers** | ⚠️ Stubbed | 10% (grammars loaded) |
| **CLI** | ✅ Complete | 95% (serve stub) |
| **HTTP Server** | ⚠️ Stub | 0% |
| **Tests** | ✅ Partial | ~40% (core modules tested) |

---

## Executive Summary

RefLex has **successfully transitioned to a trigram-based full-text search engine** with the following architecture:

**Implemented:**
- ✅ Trigram indexing module (src/trigram.rs) - FULLY FUNCTIONAL
- ✅ Content store (src/content_store.rs) - Memory-mapped file storage
- ✅ Symbol extraction (src/parsers/rust.rs) - Comprehensive Rust support
- ✅ Dual-mode querying: full-text (trigrams) + symbol-only (Tree-sitter)
- ✅ CLI framework (6 commands: index, query, stats, clear, list-files, serve)
- ✅ Incremental indexing with blake3 hashing
- ✅ SQLite metadata tracking

**Architecture:**
```
reflex index  →  [Directory Walker] → [Rust Parser (Tree-sitter)]
                       ↓                      ↓
                 [Trigram Extractor]    [Symbol Extractor]
                       ↓                      ↓
                 [content.bin]          [symbols.bin]
                 (Memory-mapped)        (rkyv + mmap)

reflex query  →  [Query Engine] → [Mode: Full-text or Symbol-only]
                       ↓
                 [Trigram Search]  OR  [Symbol Index Search]
                       ↓                      ↓
                 [Candidate Files]      [Symbol Matches]
                       ↓                      ↓
                 [Content Verification] → [Results (JSON/Text)]
```

**Next Phase:**
1. Persist trigram index to trigrams.bin
2. Implement parsers for Python, TypeScript, Go, etc.
3. Add HTTP server (optional)

---

## Priority Levels

- **P0 (MVP):** Required for minimum viable product (sub-100ms queries, basic symbol search)
- **P1 (Core):** Essential features for production readiness
- **P2 (Enhancement):** Nice-to-have features and optimizations
- **P3 (Future):** Long-term roadmap items from CLAUDE.md

---

## 🎯 MVP Goals (from CLAUDE.md)

- [ ] **Goal 1:** <100 ms per query on 100k+ files (warm path, OS cache)
- [ ] **Goal 2:** Accurate symbol-level and scope-aware retrieval for Rust, TS/JS, Go, Python, PHP, C, C++, and Java
- [ ] **Goal 3:** Fully offline; no daemon required (per-request invocation loads mmap'd cache)
- [ ] **Goal 4:** Clean, stable JSON API suitable for LLM tools and editor integrations
- [ ] **Goal 5:** Optional on-save incremental indexing

---

## 📋 Task Breakdown by Module

### 1. Cache Module (`src/cache.rs`)

#### P0: Core Cache Infrastructure ✅ COMPLETED
- [x] **Implement cache file initialization** (cache.rs:48-72)
  - Create `meta.db` with schema (SQLite) ✅
  - Create empty `symbols.bin` with header ✅
  - Create empty `tokens.bin` with header ✅
  - Create `hashes.json` with empty JSON object `{}` ✅
  - Create default `config.toml` with sensible defaults ✅

- [x] **Implement hash persistence** (cache.rs:270-299)
  - `load_hashes()`: Read and deserialize `hashes.json` ✅
  - `save_hashes()`: Serialize and write `hashes.json` ✅

- [x] **Implement cache statistics** (cache.rs:390-455)
  - Read actual file sizes from disk ✅
  - Count symbols from SQLite database ✅
  - Count files from SQLite database ✅
  - Store and retrieve last update timestamp ✅

#### P1: Memory-Mapped Readers ✅ COMPLETED
- [x] **Implement SymbolReader** (cache/symbol_reader.rs)
  - Memory-map `symbols.bin` for zero-copy reads ✅
  - Define binary format for symbol storage (rkyv) ✅
  - Implement symbol deserialization ✅
  - Add index structure for fast lookups (HashMap) ✅
  - Uses `memmap2` crate ✅

- [x] **Implement TokenReader**
  - **Note:** Replaced by trigram-based full-text search (src/trigram.rs)
  - Trigrams extracted during indexing ✅
  - Currently rebuilt on each query (TODO: persist trigrams.bin)

- [x] **Implement MetaReader** (cache.rs:355-455)
  - Read metadata from `meta.db` via SQLite ✅
  - Support queries for statistics ✅
  - Queries execute directly (no separate reader needed) ✅

#### P2: Advanced Cache Features
- [ ] Add cache versioning for schema migrations
- [ ] Implement cache corruption detection and repair
- [ ] Add cache compaction/optimization command
- [ ] Support multiple index versions (for branch switching)

---

### 2. Indexer Module (`src/indexer.rs`)

#### P0: Directory Walking & File Discovery ✅ COMPLETED
- [x] **Implement directory tree walking** (indexer.rs:193-216)
  - Use `ignore` crate to respect `.gitignore` ✅
  - Filter by configured include/exclude patterns ✅
  - Handle symlinks according to config ✅
  - Collect all eligible source files ✅

- [x] **Implement file filtering** (indexer.rs:219-244)
  - Check file extensions against supported languages ✅
  - Respect max file size limits ✅
  - Skip binary files and generated code (via ignore crate) ✅
  - TODO: Custom include/exclude glob patterns (planned)

#### P0: Incremental Indexing ✅ COMPLETED
- [x] **Implement hash-based change detection** (indexer.rs:82-113)
  - Compute blake3 hash for each file ✅
  - Compare with `hashes.json` to detect changes ✅
  - Skip unchanged files (incremental indexing) ✅
  - Preserve symbols from unchanged files ✅

- [x] **Update hash storage** (indexer.rs:180)
  - Track all indexed file hashes ✅
  - Call `cache.save_hashes()` after indexing ✅
  - Handle deleted files (remove from hash map) ✅

#### P0: Tree-sitter Integration ✅ MOSTLY COMPLETE
- [x] **Set up Tree-sitter grammar dependencies** (Cargo.toml:26-35)
  - Add `tree-sitter-rust` to Cargo.toml ✅
  - Add `tree-sitter-python` to Cargo.toml ✅
  - Add `tree-sitter-javascript` to Cargo.toml ✅
  - Add `tree-sitter-typescript` to Cargo.toml ✅
  - Add `tree-sitter-go` to Cargo.toml ✅
  - Add `tree-sitter-php` to Cargo.toml ✅
  - Add `tree-sitter-c` to Cargo.toml ✅
  - Add `tree-sitter-cpp` to Cargo.toml ✅
  - Add `tree-sitter-java` to Cargo.toml ✅

- [x] **Implement language-specific parsers** (src/parsers/)
  - Create `ParserFactory` wrapper that selects grammar by language ✅
  - Parse file into AST using Tree-sitter ✅
  - Handle parse errors gracefully ✅
  - **Rust parser complete** (parsers/rust.rs) ✅
  - **Other languages:** Stub implemented, ready for expansion

- [x] **Implement AST traversal & symbol extraction** (Rust only)
  - **Goal:** Extract ALL symbol types that Tree-sitter can identify for each language
  - **Approach:** Traverse the complete AST and identify every node that represents a searchable code entity
  - **For each language, extract (examples, not exhaustive):**
    - **Rust:** fn, struct, enum, trait, impl, const, static, mod, macro, type alias, associated types, generic parameters, lifetimes, doc comments
    - **Python:** def, class, async def/class, lambda, decorators, class/static methods, properties, docstrings, type hints, comprehensions
    - **TypeScript/JavaScript:** function, class, interface, type, const, let, var, arrow functions, async/generator functions, methods, getters/setters, JSDoc comments, type parameters
    - **Go:** func, type, struct, interface, const, var, package-level declarations, methods, embedded types
    - **PHP:** function, class, interface, trait, enum (PHP 8.1+), abstract/final classes, namespaces, methods, properties, constants, magic methods, anonymous classes, PHPDoc comments
    - **C:** function declarations/definitions, struct, enum, union, typedef, static/extern variables, macros (#define), preprocessor directives, Doxygen comments
    - **C++:** function, class, struct, namespace, template (class/function), enum/enum class, using declarations, constructors/destructors, operators, virtual/override methods, friend declarations, Doxygen comments
    - **Java:** class, interface, enum, record (Java 14+), annotation, method, field, constructor, static/abstract/final modifiers, packages, inner/anonymous classes, JavaDoc comments
  - **For ALL languages:**
    - Capture complete symbol metadata (visibility, modifiers, parameters, return types)
    - Extract associated documentation/comments
    - Track scope hierarchy and fully-qualified names
    - Handle language-specific features (generics, annotations, etc.)
  - **Note:** The lists above are starting points. Implementers should examine Tree-sitter grammar documentation for each language and extract ALL relevant node types.

- [ ] **Compute symbol spans** (Line 93)
  - Extract start/end line and column from AST nodes
  - Store as `Span { start_line, start_col, end_line, end_col }`

- [ ] **Extract scope context** (Line 93)
  - Track parent scope (e.g., "impl MyStruct", "class User")
  - Build fully-qualified symbol names
  - Handle nested scopes (modules, classes, etc.)

#### P1: Token Extraction for Lexical Search
- [ ] **Implement token extraction** (Line 43)
  - Tokenize source code (identifiers, keywords, strings)
  - Build n-gram index for fuzzy matching
  - Compress tokens with zstd
  - Write to `tokens.bin`

#### P1: Cache Writing ✅ COMPLETED
- [x] **Write symbols to cache** (indexer.rs:168-177)
  - Serialize symbols to binary format (rkyv) ✅
  - Write to `symbols.bin` (rebuild) ✅
  - Maintain index structure for fast lookups ✅

- [x] **Update metadata** (indexer.rs:182-183)
  - Write statistics to `meta.db` ✅
  - Update timestamp, file counts, symbol counts ✅

#### P1: Future-Proof Symbol Extraction
- [ ] **Implement generic fallback for unknown symbol types**
  - Design symbol extraction to handle AST nodes gracefully even if not explicitly recognized
  - Use heuristics: nodes with "name" fields, declaration-pattern node kinds, etc.
  - Classify unknown symbols with generic types (e.g., `SymbolKind::Unknown`)
  - Extract basic metadata (name, span, scope) even without language-specific handling
  - **Goal:** RefLex should work with newer language versions without crashing or missing symbols entirely

- [ ] **Add language version tracking**
  - Track which Tree-sitter grammar version was used during indexing
  - Store in meta.db for debugging and compatibility checks
  - Log warnings when grammar versions change between index/query

#### P2: Advanced Indexing Features
- [ ] Parallel file parsing with `rayon`
- [ ] Progress reporting during indexing
- [ ] Handle extremely large files (streaming parse)
- [ ] Extract import/export relationships
- [ ] Build call graph (limited, for future use)

---

### 3. Query Engine Module (`src/query.rs`)

#### P0: Cache Loading ✅ COMPLETED
- [x] **Load memory-mapped cache** (query.rs:72-74, 195-197)
  - Memory-map `symbols.bin` on query start (SymbolReader) ✅
  - Load `content.bin` for full-text search (ContentReader) ✅
  - Keep file handles open for duration of query ✅

#### P0: Query Pattern Parsing ✅ COMPLETED
- [x] **Implement query pattern parser** (query.rs:77-99)
  - Parse plain text for full-text search ✅
  - Parse `--symbols` flag for symbol-only search ✅
  - Parse `*` wildcard for prefix/substring matching ✅
  - **Note:** `symbol:name` syntax handled via CLI flags instead

#### P0: Symbol Search ✅ COMPLETED
- [x] **Implement symbol name matching** (query.rs:77-99)
  - Exact match: `--exact` flag ✅
  - Prefix match: `pattern*` ✅
  - Substring match: default behavior ✅
  - Use symbol index for fast lookups ✅

#### P1: AST Pattern Matching ⚠️ PLANNED
- [ ] **Implement Tree-sitter query support**
  - Parse Tree-sitter S-expression patterns
  - Match patterns against indexed AST data
  - Support patterns like `(function_item name: (identifier) @name)`
  - **Status:** Framework in place, not yet implemented

#### P1: Lexical Search ✅ COMPLETED (via Trigram)
- [x] **Implement trigram-based full-text search** (query.rs:192-264)
  - Search trigram index for candidate files ✅
  - Verify matches in actual content ✅
  - Return matches with context ✅
  - **Note:** Replaces token-based search with trigrams

#### P0: Filtering & Ranking ✅ COMPLETED
- [x] **Apply query filters** (query.rs:102-119)
  - Filter by language (if specified) ✅
  - Filter by symbol kind (if specified) ✅
  - Filter by file path pattern ✅
  - Apply limit to result count ✅

- [x] **Implement deterministic ranking** (query.rs:146-149)
  - Sort by file path (lexicographic) ✅
  - Sort by line number within file ✅
  - Ensure consistent ordering across runs ✅

#### P1: Result Context & Preview ✅ COMPLETED
- [x] **Generate code previews** (query.rs:121-143, content_store.rs:301-340)
  - Extract context around match ✅
  - Include full symbol body with `--expand` flag ✅
  - Format as clean, readable snippet ✅

#### P2: Advanced Query Features
- [ ] Support regex patterns
- [ ] Support wildcard patterns (`*`, `?`)
- [ ] Implement query result caching
- [ ] Add relevance scoring (optional, with deterministic tie-breaking)

---

### 4. HTTP Server (`src/cli.rs`)

#### P1: Axum Server Setup ⚠️ STUB ONLY
- [ ] **Implement HTTP server** (cli.rs:313-331)
  - Create axum router with routes
  - Bind to configured host:port
  - Handle graceful shutdown (Ctrl+C)
  - **Status:** Placeholder implementation, returns error

#### P1: API Endpoints ⚠️ NOT IMPLEMENTED
- [ ] **GET /query** endpoint
  - Query parameters: `q` (pattern), `lang`, `limit`, `ast`
  - Return JSON array of SearchResults
  - Handle errors with proper HTTP status codes

- [ ] **GET /stats** endpoint
  - Return IndexStats as JSON
  - Include cache size, file count, symbol count

- [ ] **POST /index** endpoint
  - Trigger reindexing
  - Accept optional body with IndexConfig
  - Return 202 Accepted (async indexing)

#### P2: Advanced Server Features
- [ ] Add CORS support for browser clients
- [ ] Add request logging middleware
- [ ] Implement rate limiting
- [ ] Add API authentication (optional)
- [ ] WebSocket support for streaming results

---

### 5. Tree-sitter Grammar Integration

#### Required Cargo.toml Additions ✅ COMPLETED
- [x] tree-sitter-rust = "0.23" ✅
- [x] tree-sitter-python = "0.23" ✅
- [x] tree-sitter-javascript = "0.23" ✅
- [x] tree-sitter-typescript = "0.23" ✅
- [x] tree-sitter-go = "0.23" ✅
- [x] tree-sitter-php = "0.23" ✅
- [x] tree-sitter-c = "0.23" ✅
- [x] tree-sitter-cpp = "0.23" ✅
- [x] tree-sitter-java = "0.23" ✅

#### Implementation Checklist ⚠️ PARTIALLY COMPLETE
- [x] Create `src/parsers/mod.rs` module ✅
- [x] Create `src/parsers/rust.rs` - Rust grammar integration ✅ **FULLY IMPLEMENTED**
- [ ] Create `src/parsers/python.rs` - Python grammar integration (stub exists)
- [ ] Create `src/parsers/typescript.rs` - TS/JS grammar integration (stub exists)
- [ ] Create `src/parsers/go.rs` - Go grammar integration (stub exists)
- [ ] Create `src/parsers/php.rs` - PHP grammar integration (stub exists)
- [ ] Create `src/parsers/c.rs` - C grammar integration (stub exists)
- [ ] Create `src/parsers/cpp.rs` - C++ grammar integration (stub exists)
- [ ] Create `src/parsers/java.rs` - Java grammar integration (stub exists)
- [x] Implement parser factory (select parser by Language enum) ✅
- [x] Write unit tests for Rust parser (7 tests) ✅
- [ ] Write unit tests for other parsers (when implemented)
- [ ] Document query patterns for each language

---

### 6. Data Format Design

#### P0: Binary Format Design ✅ COMPLETED
- [x] **Design symbols.bin format** (cache/symbol_writer.rs)
  - Header: magic bytes "RFLX", version, symbol count ✅
  - Symbol entries: kind, name, span, scope, path ✅
  - Index structure: HashMap for fast name lookups ✅
  - **Decision:** Using rkyv for zero-copy deserialization ✅

- [x] **Design content.bin format** (content_store.rs)
  - Header: magic bytes "RFCT", version, num_files, index_offset ✅
  - File contents: Concatenated file contents ✅
  - File index: path, offset, length for each file ✅
  - Memory-mapped for zero-copy access ✅

- [ ] **Design trigrams.bin format** ⚠️ NOT PERSISTED YET
  - Currently: Trigram index rebuilt on each query
  - TODO: Persist inverted index to disk for faster query startup
  - Proposed format: HashMap<Trigram, Vec<FileLocation>> serialized

- [x] **Design meta.db schema** (cache.rs:74-139)
  - **Decision:** SQLite (easier, more flexible) ✅
  - Tables: files (path, hash, language, symbol_count) ✅
  - Tables: statistics (key-value for totals) ✅
  - Tables: config (key-value for settings) ✅

#### P1: Serialization Implementation ✅ COMPLETED
- [x] Implement serialization for all data structures (rkyv, serde_json) ✅
- [x] Implement deserialization with version compatibility ✅
- [ ] Add schema version migration support (planned)

---

### 7. Testing & Quality

#### P0: Unit Tests
- [ ] Add tests for `CacheManager` (init, load, save, clear)
- [ ] Add tests for `Indexer` (file filtering, hashing)
- [ ] Add tests for `QueryEngine` (pattern parsing, filtering)
- [ ] Add tests for each Tree-sitter parser

#### P1: Integration Tests
- [ ] Test full indexing workflow (create test repo → index → verify)
- [ ] Test query workflow (index → query → verify results)
- [ ] Test incremental indexing (index → modify → reindex → verify)
- [ ] Test error handling (corrupt cache, missing files, parse errors)

#### P1: Performance Tests
- [ ] Benchmark indexing speed (files/sec, symbols/sec)
- [ ] Benchmark query latency (target: <100ms on 100k files)
- [ ] Test memory usage during indexing
- [ ] Test cache size vs. project size

#### P2: End-to-End Tests
- [ ] Test on real-world codebases (Linux kernel, Rust compiler, etc.)
- [ ] Verify correctness of extracted symbols
- [ ] Measure actual query performance

---

### 8. Documentation

#### P1: User Documentation
- [ ] Write comprehensive README.md
  - Installation instructions
  - Quick start guide
  - CLI usage examples
  - Configuration reference

- [ ] Write ARCHITECTURE.md
  - System design overview
  - Cache format documentation
  - Extension guide (adding new languages)

- [ ] Write API.md
  - HTTP API reference
  - JSON response format
  - Error codes

#### P1: Developer Documentation
- [ ] Add rustdoc comments to all public APIs
- [ ] Document binary file formats
- [ ] Create developer setup guide
- [ ] Add CONTRIBUTING.md

---

### 9. Tooling & Infrastructure

#### P1: Development Tools
- [ ] Add `cargo fmt` check to CI
- [ ] Add `cargo clippy` check to CI
- [ ] Set up GitHub Actions workflow
- [ ] Add pre-commit hooks

#### P2: Release Engineering
- [ ] Set up cross-compilation for Linux, macOS, Windows
- [ ] Create release binaries
- [ ] Publish to crates.io
- [ ] Create installation script

---

### 10. Future Work (P3, from CLAUDE.md)

#### Long-term Features
- [ ] `reflexd`: Background indexing daemon
- [ ] MCP (Model Context Protocol) adapter
- [ ] LSP (Language Server Protocol) adapter
- [ ] Graph queries (imports/exports, call graph)
- [ ] Branch-aware context diffing (`--since`, `--branch`)
- [ ] Binary protocol for ultra-low-latency queries
- [ ] File system watcher for auto-reindexing
- [ ] Plugin system for custom languages

---

## 🔗 Dependency Graph

```
Cache File Formats (design) ──→ Cache Implementation ──→ Indexer & Query Engine
                                                      ↓
Tree-sitter Grammars ──────────→ AST Extraction ──────→ Symbol Indexing
                                                      ↓
                                                Query Execution
                                                      ↓
                                              HTTP Server (optional)
```

**Critical Path for MVP:**
1. Design and implement cache file formats
2. Integrate Tree-sitter grammars
3. Implement indexer with symbol extraction
4. Implement query engine with symbol search
5. Test on real codebases

---

## 📊 Current TODO Comments in Source

### src/cache.rs
- Line 43: Create initial cache files
- Line 84: Implement hash loading from hashes.json
- Line 91: Implement hash saving to hashes.json
- Line 97: Read actual stats from cache
- Line 108: Implement memory-mapped readers

### src/indexer.rs
- Line 36: Implement the actual indexing logic (7 steps)
- Line 64: Implement filtering logic (4 steps)
- Line 83: Implement Tree-sitter parsing (5 steps)

### src/query.rs
- Line 58: Implement query execution (6 steps)

### src/cli.rs
- Line 238: Implement HTTP server using axum (3 endpoints)

---

## 🚀 Suggested Implementation Order

### Phase 1: Foundation (Week 1)
1. Design binary file formats for cache
2. Implement CacheManager core functionality
3. Add Tree-sitter grammar dependencies
4. Set up parser module structure

### Phase 2: Indexing (Week 2-3)
1. Implement directory walking and file filtering
2. Implement Tree-sitter parsers for Rust (start with one language)
3. Implement symbol extraction for Rust
4. Write symbols to cache (basic format)
5. Implement incremental indexing with hashes
6. Test on small Rust project

### Phase 3: Querying (Week 3-4)
1. Implement memory-mapped cache readers
2. Implement symbol search (exact match)
3. Implement query filtering
4. Implement result preview generation
5. Test query performance

### Phase 4: Multi-Language Support (Week 4-6)
1. Add Python parser and symbol extraction
2. Add TypeScript/JavaScript parser
3. Add Go parser
4. Add PHP parser and symbol extraction
5. Add C parser and symbol extraction
6. Add C++ parser and symbol extraction
7. Add Java parser and symbol extraction
8. Test on polyglot projects

### Phase 5: Polish & Performance (Week 7-8)
1. Optimize query latency (target <100ms)
2. Add comprehensive tests for all languages
3. Write documentation
4. Implement HTTP server (optional)

---

## ✅ Completed Items

### Project Foundation
- [x] Project scaffolding (Cargo.toml, module structure)
- [x] CLI framework with all subcommands
- [x] Core data models (SearchResult, Span, Language, etc.)
- [x] Error handling setup (anyhow)
- [x] Logging setup (env_logger)
- [x] Basic integration test structure
- [x] .gitignore configuration
- [x] Dependency management (all required crates added)

### Tree-sitter Grammars (COMPLETED - All in Cargo.toml)
- [x] tree-sitter-rust = "0.23"
- [x] tree-sitter-python = "0.23"
- [x] tree-sitter-javascript = "0.23"
- [x] tree-sitter-typescript = "0.23"
- [x] tree-sitter-go = "0.23"
- [x] tree-sitter-php = "0.23"
- [x] tree-sitter-c = "0.23"
- [x] tree-sitter-cpp = "0.23"
- [x] tree-sitter-java = "0.23"

### Trigram Indexing (COMPLETED - src/trigram.rs)
- [x] Trigram extraction from text
- [x] Inverted index: trigram → file locations
- [x] Posting list intersection algorithms
- [x] File-based candidate search
- [x] Comprehensive tests (11 test cases)
- [x] Integration with indexer and query engine

### Content Store (COMPLETED - src/content_store.rs)
- [x] Binary format design (magic bytes, header, index)
- [x] ContentWriter for building content.bin
- [x] ContentReader with memory-mapped I/O
- [x] Context extraction around matches
- [x] Comprehensive tests (5 test cases)
- [x] Integration with indexer and query engine

### Symbol Storage (COMPLETED - src/cache/symbol_{reader,writer}.rs)
- [x] SymbolWriter with rkyv serialization
- [x] SymbolReader with memory-mapped I/O
- [x] Symbol index for fast name lookups
- [x] Find by name, prefix, substring
- [x] Comprehensive tests (4 test cases)

### Cache Infrastructure (COMPLETED - src/cache.rs)
- [x] Cache initialization (init())
- [x] Create meta.db with SQLite schema
- [x] Create empty symbols.bin with header
- [x] Create empty tokens.bin with header
- [x] Create hashes.json with empty map
- [x] Create default config.toml
- [x] Hash persistence (load_hashes, save_hashes)
- [x] Cache statistics (stats())
- [x] File metadata tracking (update_file)
- [x] List indexed files (list_files)

### Indexer (MOSTLY COMPLETE - src/indexer.rs)
- [x] Directory walking with .gitignore support (ignore crate)
- [x] File filtering by language and size
- [x] Hash-based incremental indexing (blake3)
- [x] Tree-sitter integration (Rust parser)
- [x] Trigram index building
- [x] Content store population
- [x] Symbol extraction and caching
- [x] Preserve unchanged files during incremental indexing

### Rust Parser (COMPLETED - src/parsers/rust.rs)
- [x] Parse functions (fn)
- [x] Parse structs
- [x] Parse enums
- [x] Parse traits
- [x] Parse impl blocks and methods
- [x] Parse constants
- [x] Parse modules
- [x] Parse type aliases
- [x] Extract spans (line/col)
- [x] Extract scope context
- [x] Comprehensive tests (7 test cases)

### Query Engine (COMPLETED - src/query.rs)
- [x] Load memory-mapped cache (SymbolReader, ContentReader)
- [x] Symbol-only search mode (--symbols flag)
- [x] Trigram-based full-text search
- [x] Query pattern parsing (plain text, symbol:, prefix *)
- [x] Symbol name matching (exact, prefix, substring)
- [x] Filter by language
- [x] Filter by symbol kind
- [x] Filter by file path (substring)
- [x] Deterministic sorting (path → line number)
- [x] Result limit
- [x] Expand mode for full symbol bodies (--expand)

### CLI (MOSTLY COMPLETE - src/cli.rs)
- [x] index command (with --force, --languages)
- [x] query command (with all filters: --symbols, --lang, --kind, --json, --limit, --expand, --file, --exact)
- [x] stats command (with --json)
- [x] clear command (with --yes)
- [x] list-files command (with --json)
- [x] Verbose logging (-v, -vv, -vvv)
- [x] JSON output support across commands

---

## 📝 Notes & Design Decisions

### Open Questions
1. **Cache format:** SQLite vs custom binary for meta.db?
   - SQLite: Easier, more flexible, built-in query support
   - Custom: Potentially faster, smaller, more control
   - **Recommendation:** Start with SQLite, optimize later if needed

2. **Symbol index structure:** B-tree vs hash table?
   - B-tree: Better for range queries, ordered iteration
   - Hash table: Faster for exact lookups
   - **Recommendation:** Hash table for symbol names, B-tree for file paths

3. **Compression:** When to compress?
   - Compress tokens.bin (high redundancy)
   - Don't compress symbols.bin (need random access)
   - **Recommendation:** Use zstd for tokens, raw binary for symbols

### Performance Targets
- Indexing: >10,000 files/sec on modern SSD
- Query: <100ms on 100k files (warm cache)
- Memory: <500MB for 100k files
- Cache size: <10% of source code size

### Maintenance & Updates

**Will RefLex need updates when languages evolve?**

Yes, but we can minimize the impact:

1. **Tree-sitter Grammar Dependencies**
   - RefLex depends on external Tree-sitter grammars (e.g., `tree-sitter-php = "0.23"`)
   - When languages add new syntax (PHP enums, Java records, etc.), grammars are updated
   - RefLex must periodically update grammar versions and test compatibility

2. **Future-Proofing Strategy**
   - **Explicit handling:** Common, stable symbols get full support with complete metadata
   - **Generic fallback:** Unknown/new symbols are still extracted with basic info (name, location, scope)
   - **Graceful degradation:** New language features won't crash RefLex, just may be classified generically
   - **Periodic updates:** Release RefLex updates when major language versions add significant new syntax

3. **Update Frequency**
   - **Minor updates:** Bug fixes, grammar version bumps (quarterly)
   - **Major updates:** New language support, significant syntax additions (as needed)
   - **Grammar updates are opt-in:** Users can update Cargo.toml to newer grammars independently

4. **Compatibility Promise**
   - Cache format versioning allows migration between RefLex versions
   - Older caches can be rebuilt with newer RefLex versions
   - Breaking changes will be clearly documented with migration guides

---

**END OF TODO.md**
