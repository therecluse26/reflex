# README.md Discrepancies Audit

This document catalogs discrepancies between the README.md documentation and the actual codebase implementation, discovered during an audit on 2025-11-11.

---

## 1. Cache Structure Documentation

**Location:** README.md § "Cache Structure (`.reflex/`)"

**Issue:** Cache structure documentation is outdated

**README Claims:**
```
.reflex/
  meta.db          # SQLite: file metadata, stats, config
  trigrams.bin     # Inverted index (memory-mapped)
  content.bin      # Full file contents (memory-mapped)
  hashes.json      # File hashes for incremental indexing
  config.toml      # Index settings
```

**Actual Implementation (`src/cache.rs:1-25`):**
```
.reflex/
  meta.db          # SQLite: file metadata, stats, config, HASHES (in file_branches table)
  trigrams.bin     # Inverted index (memory-mapped)
  content.bin      # Full file contents (memory-mapped)
  config.toml      # Index settings
  indexing.status  # Background symbol indexer status
```

**Discrepancies:**
1. **`hashes.json` is deprecated** - File hashes are now stored in the `file_branches` table within `meta.db` (per-branch tracking). See `cache.rs:54` comment: "hashes.json is deprecated - hashes are now stored in meta.db"
2. **`indexing.status` is missing from README** - New file added for background symbol indexing status tracking

**Recommendation:** Update README cache structure diagram to remove `hashes.json` and add `indexing.status`

---

## 2. Interactive Mode Status

**Location:** README.md § "Roadmap" § "Next Phase: Advanced Features"

**Issue:** Interactive mode is documented as "In Development" but is actually fully implemented

**README Claims:**
```markdown
- [ ] Interactive mode (TUI) - **In Development** on feature/interactive-mode branch
```

**Actual State:**
- Interactive mode is **fully implemented** with 13 modules in `src/interactive/`
- Accessible by running `rfx` with no arguments
- CLI help text states: "Run 'rfx' with no arguments to launch interactive mode."
- Uses ratatui, tachyonfx, crossterm, syntect for full TUI experience
- Current branch is `feature/dependency-graph`, not `feature/interactive-mode`

**Evidence:**
- `src/interactive/mod.rs` exports complete implementation
- `src/cli.rs:23` documents the feature
- `src/cli.rs:312-316` implements the default behavior
- `Cargo.toml:92-97` includes all TUI dependencies

**Recommendation:** Mark interactive mode as completed (✅) in README roadmap and document its usage in the main feature section

---

## 3. Background Symbol Indexing

**Location:** README.md (missing documentation)

**Issue:** Background symbol indexing feature exists but is not documented in README

**What Exists:**
- Full background indexer implementation in `src/background_indexer.rs`
- Symbol cache system in `src/symbol_cache.rs` (803 lines)
- `rfx index --status` command to check indexing progress
- Daemonized process spawning with cross-platform support (Unix/Windows)
- Status file tracking (`.reflex/indexing.status`)

**CLI Evidence:**
```rust
// src/cli.rs:54-56
/// Show background symbol indexing status
#[arg(long)]
status: bool,
```

**Where It Should Be Documented:**
- Main features list
- `rfx index` command documentation
- Architecture overview

**Recommendation:** Add documentation for background symbol indexing, explain how it improves query performance, and document the `--status` flag

---

## 4. Swift Language Support

**Location:** README.md § "Supported Languages"

**Issue:** Swift is documented as supported but is actually disabled

**README Claims:**
```markdown
| **Swift** | `.swift` | Classes, structs, enums, protocols, functions, extensions, properties, actors | ... |
```

**Actual Implementation:**

`src/models.rs:125`:
```rust
Language::Swift => false,  // Temporarily disabled - requires tree-sitter 0.23
```

`src/parsers/mod.rs:22`:
```rust
// pub mod swift;  // Temporarily disabled - requires tree-sitter 0.23
```

`src/parsers/mod.rs:89-91`:
```rust
Language::Swift => Err(anyhow!(
    "Swift support temporarily disabled (requires tree-sitter 0.23)"
)),
```

`Cargo.toml:42`:
```toml
# tree-sitter-swift = "0.7.1"  # Temporarily disabled - requires tree-sitter 0.23
```

**Root Cause:** Reflex uses tree-sitter 0.24, but tree-sitter-swift requires 0.23 (version incompatibility)

**Recommendation:** Either:
1. Add a note to the Swift entry: "**Temporarily disabled** - requires tree-sitter 0.23 (Reflex uses 0.24)"
2. Or remove Swift entirely from the supported languages table until compatibility is resolved

---

## 5. Dependencies/Import Tracking Feature

**Location:** README.md (missing documentation)

**Issue:** Full dependency tracking system exists but is completely undocumented in README

**What Exists:**
- `file_dependencies` table in SQLite schema (`src/cache.rs:155-183`)
- `DependencyInfo` and `Dependency` models (`src/models.rs:143-173`)
- `DependencyExtractor` trait for parsers (`src/parsers/mod.rs:44-62`)
- `src/dependency.rs` module (full implementation)
- CLI `--dependencies` flag (`src/cli.rs:205-206`)
- HTTP API `dependencies` parameter (`src/cli.rs:984`)
- MCP `dependencies` parameter (`src/mcp.rs:244-247`)

**CLI Evidence:**
```rust
/// Include dependency information (imports) in results
/// Currently only available for Rust files
#[arg(long)]
dependencies: bool,
```

**Database Schema:**
```sql
CREATE TABLE IF NOT EXISTS file_dependencies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_id INTEGER NOT NULL,
    imported_path TEXT NOT NULL,
    resolved_file_id INTEGER,
    import_type TEXT NOT NULL,
    line_number INTEGER NOT NULL,
    imported_symbols TEXT,
    ...
)
```

**Usage Example (from CLI):**
```bash
rfx query "MyStruct" --dependencies  # Returns results with dependency information
```

**Recommendation:** Document the dependency tracking feature:
- Add to main features list
- Document the `--dependencies` flag
- Explain what dependency information is returned
- Note current limitation: "Currently only available for Rust files"

---

## 6. MCP Tool Names and Descriptions

**Location:** README.md § "MCP (Model Context Protocol)" § "Available MCP Tools"

**Issue:** README lists 4 MCP tools, but implementation has 5 tools with different naming

**README Lists:**
1. `search_code`
2. `search_regex`
3. `search_ast`
4. `index_project`

**Actual Implementation (`src/mcp.rs:100-368`):**
1. **`list_locations`** - Fast location discovery (file + line only, minimal tokens)
2. **`count_occurrences`** - Quick statistics (total count + file count)
3. `search_code` - Full-text or symbol-only search with detailed results
4. `search_regex` - Regex pattern matching
5. `search_ast` - AST pattern matching (structure-aware)
6. `index_project` - Rebuild or update the index

**Missing from README:**
- `list_locations` - Optimized for "where is X used?" queries (returns only paths + line numbers)
- `count_occurrences` - Optimized for "how many times is X used?" queries (returns counts only)

**Tool Descriptions:** The implementation has extensive documentation in the MCP schema that's not reflected in README

**Recommendation:**
1. Update README to list all 6 MCP tools
2. Add descriptions explaining when to use each tool
3. Document the token-efficiency optimizations (list_locations vs search_code)

---

## 7. Swift in CLI Language Lists

**Location:** Various CLI error messages

**Issue:** Swift is excluded from CLI language validation but still appears in some documentation

**CLI Language Validation (`src/cli.rs:594-634`):**
Supported languages in CLI error messages do **NOT include Swift**:
```rust
"rust, rs"
"python, py"
"javascript, js"
"typescript, ts"
"vue"
"svelte"
"go"
"java"
"php"
"c"
"c++, cpp"
"c#, csharp, cs"
"ruby, rb"
"kotlin, kt"
"zig"
// No Swift!
```

**CLAUDE.md Documentation:**
Still mentions Swift in the supported languages table but with a disclaimer.

**Recommendation:** Ensure Swift is consistently excluded or marked as disabled across all documentation

---

## 8. AST Query Performance Warning

**Location:** README.md § "AST Pattern Matching" § "How It Works"

**Issue:** Performance warning is present but could be more prominent

**README States:**
```
**Performance:** AST queries add 2-224ms overhead (parsing only candidate files, not entire codebase)
```

**Actual Behavior (from CLI help and MCP descriptions):**
```
WARNING: AST queries are SLOW (500ms-2s+). Use --symbols instead for 95% of cases.
```

**CLI Warning (`src/cli.rs:93-106`):**
```rust
/// WARNING: AST queries bypass trigram optimization and scan the entire codebase.
/// In 95% of cases, use --symbols instead which is 10-100x faster.
```

**Discrepancy:** README downplays the performance impact ("2-224ms overhead") vs. implementation warnings ("500ms-2s+, scan entire codebase")

**Recommendation:** Update README to match the stronger warnings in the CLI and MCP descriptions. Make it clear that AST queries are significantly slower and should be used sparingly.

---

## 9. Symbol Cache Implementation

**Location:** README.md § "Architecture" (missing)

**Issue:** Symbol cache is a major component but not documented in README architecture section

**What Exists:**
- `src/symbol_cache.rs` - 803-line implementation
- SQLite-based persistent symbol cache
- Background indexing daemon that populates cache
- Branch-aware symbol caching
- Hash-based symbol reuse across branches

**Where It's Mentioned:**
- CLAUDE.md § "Components" lists "Symbol Cache"
- README § "Architecture" does NOT mention symbol cache

**Recommendation:** Add symbol cache to README architecture diagram and explain its role in query performance

---

## 10. Roadmap Completion Status

**Location:** README.md § "Roadmap" § "v1.0.0 Production Ready ✅"

**Issue:** Some completed items might be incorrectly marked

**Marked as Completed:**
- ✅ Interactive mode (TUI) - Actually completed (see #2 above)
- ✅ Background symbol indexing - Actually completed (see #3 above)

**Not Mentioned But Implemented:**
- ✅ Dependency/import tracking (see #5 above)
- ✅ Symbol cache system (see #9 above)

**Recommendation:**
1. Verify all completed items are actually marked with ✅
2. Add missing implemented features to the completed list
3. Update the "Next Phase" section to remove completed items

---

## Summary

### Critical Issues (User-Facing)
1. **Swift Support** (#4) - Documented as supported but is disabled
2. **Interactive Mode** (#2) - Fully functional but marked as "In Development"
3. **Dependencies Feature** (#5) - Implemented but completely undocumented

### Documentation Gaps (Feature Coverage)
1. **Background Symbol Indexing** (#3) - Major feature missing from README
2. **MCP Tools** (#6) - Missing tools and descriptions
3. **Symbol Cache** (#9) - Core architecture component not documented

### Technical Accuracy
1. **Cache Structure** (#1) - Outdated (hashes.json removed, indexing.status added)
2. **AST Performance Warning** (#8) - Understated in README vs. implementation

### Consistency Issues
1. **Swift in CLI** (#7) - Inconsistent exclusion across documentation
2. **Roadmap Status** (#10) - Completed items not properly marked

---

## Recommendations Priority

**High Priority (Fix Immediately):**
1. Mark Swift as disabled/temporarily unavailable in README (#4)
2. Document the dependency tracking feature (#5)
3. Update interactive mode status to completed (#2)
4. Update cache structure diagram (#1)

**Medium Priority (Next Update):**
1. Document background symbol indexing (#3)
2. Add missing MCP tools to documentation (#6)
3. Strengthen AST query performance warnings (#8)

**Low Priority (Future Cleanup):**
1. Add symbol cache to architecture documentation (#9)
2. Review and update roadmap completion status (#10)
3. Ensure Swift is consistently excluded (#7)

---

**Audit Date:** 2025-11-11
**Auditor:** Claude Code
**README Version:** v0.6.0
**Branch:** feature/dependency-graph
