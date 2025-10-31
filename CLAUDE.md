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
- Context/planning: `.context/` (tracked in git)

---

## Context Management & AI Workflow

### `.context/` Directory Structure

The `.context/` directory contains planning documents, research notes, and decision logs to maintain context across development sessions. **All AI assistants working on RefLex must actively use and update these files.**

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

When working on RefLex, AI assistants should:

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
RefLex favors local autonomy, speed, and clarity.

- Fast enough to call multiple times per agent step.  
- Deterministic for repeatable reasoning.  
- Simple to rebuild: delete `.reflex/` and re-index at any time.

> “Understand your code the way your compiler does — instantly.”
