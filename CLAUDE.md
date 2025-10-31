# CLAUDE.md

## Project Overview
**RefLex** is a local-first, structure-aware code search engine written in Rust. It’s a fast, deterministic, machine-friendly replacement for tools like grep/ripgrep/plain code search, designed specifically to feed AI coding agents and automation.

RefLex returns structured results (symbols, spans, scopes, imports, docstrings) with sub-100 ms query latency on large monorepos by reading a lightweight, incremental cache stored in `.reflex/` (which is `.gitignored`).

---

## Core Principles
1. Local-first: runs fully offline; all data stays on the developer’s machine.  
2. Structured, not semantic: uses parsers (Tree-sitter) and deterministic indexes; no embeddings required.  
3. Deterministic results: same query → same answer; no probabilistic ranking.  
4. Instant access: memory-mapped cache enables per-invocation queries in tens of milliseconds.  
5. Agent-oriented: outputs normalized JSON built for programmatic consumption (LLMs, IDEs, scripts).

---

## Architecture Overview

### Components
| Module | Description |
| --- | --- |
| Indexer | Scans and parses code with Tree-sitter; writes token/AST summaries and metadata into `.reflex/`. |
| Query Engine | Loads the cache on demand; executes lexical + structural lookups; returns structured JSON. |
| CLI / API Layer | Single binary for human and programmatic use (CLI and optional HTTP/MCP). |
| Watcher (optional) | Incrementally updates cache on save, commit, or branch change. |

### Index Cache Structure (`.reflex/`)
    .reflex/
      meta.db          # Metadata and config
      symbols.bin      # Serialized symbol table (functions/classes/consts, spans, scopes)
      tokens.bin       # Compressed lexical tokens / n-grams
      hashes.json      # blake3(file) -> metadata for incremental updates
      config.toml      # Index settings (languages, filters, ignore rules)

---

## CLI Usage

    # Build or update the local cache
    reflex index

    # Query symbol definitions
    reflex query "symbol:get_user"

    # Structural pattern search (Tree-sitter pattern)
    reflex query 'fn :name(params)' --lang rust --ast

    # Export results as JSON (for AI agent use)
    reflex query 'class User' --json

    # Serve a local HTTP API (optional)
    reflex serve --port 7878

---

## Tech Stack
- Language: Rust (Edition 2024)  
- Crates:
  - Parsing: `tree-sitter` + language grammars  
  - Index/Meta: `tantivy` (or a custom mmap’d store), `serde`, `serde_json`  
  - Incremental: `blake3` (content hashing), `notify` (FS watcher, optional)  
  - Perf/infra: `rayon` (parallel), `zstd`/`lz4` (compression), `clap` (CLI)

---

## Development Workflow

### Build
    cargo build --release

### Test
    cargo test

### Refresh Index
    reflex index

### Debug Queries
    RUST_LOG=debug reflex query "fn main"

---

## Design Notes
- Incremental by content: files reindexed only if `blake3` changes.  
- Memory-mapped reads: cache is mmap’d for zero-copy, fast cold-start.  
- Deterministic filters: queries support lexical, regex, and AST-pattern filters (no embeddings).  
- Respect ignore rules: honors `.gitignore` plus `config.toml` language/path allow/deny lists.  
- Programmatic output: results are normalized JSON like:
  - `{ path, lang, kind, symbol, span {start,end}, scope, preview }`.

---

## MVP Goals
1. <100 ms per query on 100k+ files (warm path, OS cache).
2. Accurate symbol-level and scope-aware retrieval for Rust, TS/JS, Go, Python, PHP, C, C++, and Java.
3. Fully offline; no daemon required (per-request invocation loads mmap'd cache).
4. Clean, stable JSON API suitable for LLM tools and editor integrations.
5. Optional on-save incremental indexing.

---

## Future Work
- `reflexd`: tiny background helper for continuous indexing (opt-in).  
- MCP / LSP adapters for direct IDE/agent integration.  
- Graph queries (imports/exports, limited call graph).  
- Branch-aware context diffing and filters (e.g., `--since`, `--branch`).  
- Binary protocol for ultra-low-latency local queries.

---

## Repository Conventions
- Source: `src/`  
- Core library: `src/lib.rs`  
- CLI entrypoint: `src/main.rs`  
- Tests: `tests/`  
- Local cache/config: `.reflex/` (added to `.gitignore`)

---

## Project Philosophy
RefLex favors local autonomy, speed, and clarity.

- Fast enough to call multiple times per agent step.  
- Deterministic for repeatable reasoning.  
- Simple to rebuild: delete `.reflex/` and re-index at any time.

> “Understand your code the way your compiler does — instantly.”
