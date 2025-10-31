# RefLex TODO

**Last Updated:** 2025-10-31
**Project Status:** Scaffolded, Core Implementation Pending

> **⚠️ AI Assistants:** Read the "Context Management & AI Workflow" section in `CLAUDE.md` for instructions on maintaining this file and creating RESEARCH.md documents. This TODO.md MUST be updated as you work on tasks.

---

## Executive Summary

RefLex has been successfully scaffolded with all major modules in place:
- ✅ CLI framework (clap-based, 5 subcommands)
- ✅ Core data models (SearchResult, Span, Language, SymbolKind, etc.)
- ✅ Module structure (cache, indexer, query, cli)
- ✅ Build system (Rust 2024, all dependencies configured)
- ✅ Basic tests (integration test stubs)

**Current State:** All modules are placeholder implementations with comprehensive TODO comments. The project compiles and the CLI is functional, but no actual indexing or querying capability exists yet.

**Next Phase:** Implement core indexing and query functionality to achieve MVP goals.

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

#### P0: Core Cache Infrastructure
- [ ] **Implement cache file initialization** (Line 43)
  - Create `meta.db` with schema (SQLite or custom binary format)
  - Create empty `symbols.bin` with header
  - Create empty `tokens.bin` with header
  - Create `hashes.json` with empty JSON object `{}`
  - Create default `config.toml` with sensible defaults
  - **Blocked by:** Schema design decision (SQLite vs custom binary)

- [ ] **Implement hash persistence** (Lines 84-92)
  - `load_hashes()`: Read and deserialize `hashes.json`
  - `save_hashes()`: Serialize and write `hashes.json`
  - **Dependencies:** `serde_json` (already included)

- [ ] **Implement cache statistics** (Line 97)
  - Read actual file sizes from disk
  - Count symbols from `symbols.bin`
  - Count files from `hashes.json`
  - Store and retrieve last update timestamp

#### P1: Memory-Mapped Readers
- [ ] **Implement SymbolReader** (Line 108)
  - Memory-map `symbols.bin` for zero-copy reads
  - Define binary format for symbol storage
  - Implement symbol deserialization
  - Add index structure for fast lookups
  - **Dependencies:** Consider `memmap2` crate

- [ ] **Implement TokenReader** (Line 108)
  - Memory-map `tokens.bin` for lexical search
  - Design n-gram or full-text index structure
  - Implement token deserialization
  - Add compression/decompression (zstd)

- [ ] **Implement MetaReader** (Line 108)
  - Read metadata from `meta.db`
  - Support queries for statistics
  - Cache metadata in memory for fast access

#### P2: Advanced Cache Features
- [ ] Add cache versioning for schema migrations
- [ ] Implement cache corruption detection and repair
- [ ] Add cache compaction/optimization command
- [ ] Support multiple index versions (for branch switching)

---

### 2. Indexer Module (`src/indexer.rs`)

#### P0: Directory Walking & File Discovery
- [ ] **Implement directory tree walking** (Line 36, step 1)
  - Use `ignore` crate to respect `.gitignore`
  - Filter by configured include/exclude patterns
  - Handle symlinks according to config
  - Collect all eligible source files
  - **Dependencies:** `ignore` crate (already included)

- [ ] **Implement file filtering** (Line 64)
  - Check file extensions against supported languages
  - Respect max file size limits
  - Apply custom include/exclude glob patterns
  - Skip binary files and generated code

#### P0: Incremental Indexing
- [ ] **Implement hash-based change detection** (Lines 39-40)
  - Compute blake3 hash for each file
  - Compare with `hashes.json` to detect changes
  - Skip unchanged files (incremental indexing)
  - **Dependencies:** `blake3` crate (already included)

- [ ] **Update hash storage** (Line 45)
  - Track all indexed file hashes
  - Call `cache.save_hashes()` after indexing
  - Handle deleted files (remove from hash map)

#### P0: Tree-sitter Integration (CRITICAL PATH)
- [ ] **Set up Tree-sitter grammar dependencies** (Line 83)
  - Add `tree-sitter-rust` to Cargo.toml
  - Add `tree-sitter-python` to Cargo.toml
  - Add `tree-sitter-javascript` to Cargo.toml
  - Add `tree-sitter-typescript` to Cargo.toml
  - Add `tree-sitter-go` to Cargo.toml
  - Add `tree-sitter-php` to Cargo.toml
  - Add `tree-sitter-c` to Cargo.toml
  - Add `tree-sitter-cpp` to Cargo.toml
  - Add `tree-sitter-java` to Cargo.toml
  - **Note:** Each grammar is a separate crate

- [ ] **Implement language-specific parsers** (Lines 84-93)
  - Create `Parser` wrapper that selects grammar by language
  - Parse file into AST using Tree-sitter
  - Handle parse errors gracefully
  - Cache parser instances for reuse

- [ ] **Implement AST traversal & symbol extraction**
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

#### P1: Cache Writing
- [ ] **Write symbols to cache** (Line 44)
  - Serialize symbols to binary format
  - Write to `symbols.bin` (append or rebuild)
  - Maintain index structure for fast lookups

- [ ] **Update metadata** (Line 46)
  - Write statistics to `meta.db`
  - Update timestamp, file counts, symbol counts

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

#### P0: Cache Loading
- [ ] **Load memory-mapped cache** (Line 59)
  - Memory-map `symbols.bin` on query start
  - Memory-map `tokens.bin` on query start
  - Keep file handles open for duration of query
  - **Dependencies:** SymbolReader, TokenReader implementations

#### P0: Query Pattern Parsing
- [ ] **Implement query pattern parser** (Lines 60-62)
  - Parse `symbol:name` syntax (exact symbol name match)
  - Parse plain text for lexical search
  - Validate query syntax, return helpful errors

#### P0: Symbol Search
- [ ] **Implement symbol name matching**
  - Exact match: `symbol:get_user`
  - Prefix match: `symbol:get_*`
  - Fuzzy match (Levenshtein distance)
  - Use symbol index for O(log n) lookup

#### P1: AST Pattern Matching
- [ ] **Implement Tree-sitter query support** (Line 62)
  - Parse Tree-sitter S-expression patterns
  - Match patterns against indexed AST data
  - Support patterns like `(function_item name: (identifier) @name)`
  - **Dependencies:** `tree-sitter` query API

#### P1: Lexical Search
- [ ] **Implement token-based search** (Line 63)
  - Search n-gram index in `tokens.bin`
  - Rank results by relevance
  - Return matches with context

#### P0: Filtering & Ranking
- [ ] **Apply query filters** (Line 64)
  - Filter by language (if specified)
  - Filter by symbol kind (if specified)
  - Apply limit to result count

- [ ] **Implement deterministic ranking** (Line 65)
  - Sort by file path (lexicographic)
  - Sort by line number within file
  - Ensure consistent ordering across runs

#### P1: Result Context & Preview
- [ ] **Generate code previews** (Line 67)
  - Extract 3-5 lines around match
  - Include syntax context (scope, parameters)
  - Format as clean, readable snippet

#### P2: Advanced Query Features
- [ ] Support regex patterns
- [ ] Support wildcard patterns (`*`, `?`)
- [ ] Implement query result caching
- [ ] Add relevance scoring (optional, with deterministic tie-breaking)

---

### 4. HTTP Server (`src/cli.rs`)

#### P1: Axum Server Setup
- [ ] **Implement HTTP server** (Line 238)
  - Create axum router with routes
  - Bind to configured host:port
  - Handle graceful shutdown (Ctrl+C)

#### P1: API Endpoints
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

This is the **CRITICAL PATH** for the MVP. All grammars must be integrated before indexing and querying can work.

#### Required Cargo.toml Additions
```toml
[dependencies]
tree-sitter-rust = "0.23"
tree-sitter-python = "0.23"
tree-sitter-javascript = "0.23"
tree-sitter-typescript = "0.23"
tree-sitter-go = "0.23"
tree-sitter-php = "0.23"
tree-sitter-c = "0.23"
tree-sitter-cpp = "0.23"
tree-sitter-java = "0.23"
```

#### Implementation Checklist
- [ ] Create `src/parsers/mod.rs` module
- [ ] Create `src/parsers/rust.rs` - Rust grammar integration
- [ ] Create `src/parsers/python.rs` - Python grammar integration
- [ ] Create `src/parsers/typescript.rs` - TS/JS grammar integration
- [ ] Create `src/parsers/go.rs` - Go grammar integration
- [ ] Create `src/parsers/php.rs` - PHP grammar integration
- [ ] Create `src/parsers/c.rs` - C grammar integration
- [ ] Create `src/parsers/cpp.rs` - C++ grammar integration
- [ ] Create `src/parsers/java.rs` - Java grammar integration
- [ ] Implement parser factory (select parser by Language enum)
- [ ] Write unit tests for each parser
- [ ] Document query patterns for each language

---

### 6. Data Format Design

#### P0: Binary Format Design
- [ ] **Design symbols.bin format**
  - Header: magic bytes, version, symbol count
  - Symbol entries: kind, name, span, scope, file_id
  - Index structure: B-tree or hash table for fast lookup
  - Consider using bincode or custom serialization

- [ ] **Design tokens.bin format**
  - Header: magic bytes, version, compression type
  - Token entries: file_id, position, token_text
  - N-gram index for substring matching
  - Compressed with zstd

- [ ] **Design meta.db schema**
  - Option 1: SQLite (easier, more flexible)
  - Option 2: Custom binary format (faster, more control)
  - Tables: files, statistics, config
  - **Decision needed:** SQLite vs custom format

#### P1: Serialization Implementation
- [ ] Implement serialization for all data structures
- [ ] Implement deserialization with version compatibility
- [ ] Add schema version migration support

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

- [x] Project scaffolding (Cargo.toml, module structure)
- [x] CLI framework with all subcommands
- [x] Core data models (SearchResult, Span, Language, etc.)
- [x] Error handling setup (anyhow)
- [x] Logging setup (env_logger)
- [x] Basic integration test structure
- [x] .gitignore configuration
- [x] Dependency management (all required crates added)

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
