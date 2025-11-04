# `.context/` Directory

This directory contains planning documents, research notes, and decision logs to maintain context across development sessions.

## Purpose

The `.context/` directory serves as a **persistent knowledge base** for Reflex development, enabling:
- Continuity across development sessions (human and AI)
- Decision tracking and rationale
- Research caching to avoid duplicate work
- Onboarding for new contributors
- AI assistant collaboration and handoff

## Core Files

### `TODO.md` (Required)
**Primary task tracking and implementation roadmap**

Contains:
- Executive summary of project status
- MVP goals and success criteria
- Task breakdown by module (P0/P1/P2/P3 priorities)
- Implementation phases and timeline
- Open questions and design decisions
- Performance targets
- Maintenance strategy

**Usage:**
- Read at start of every session
- Update task statuses as work progresses
- Add new tasks as discovered
- Document architectural decisions

### Research Files (Optional)

Create `*_RESEARCH.md` files as needed to cache important findings:

- **`TREE_SITTER_RESEARCH.md`** - Grammar investigation, node types, query patterns
- **`PERFORMANCE_RESEARCH.md`** - Benchmarks, optimizations, bottleneck analysis
- **`BINARY_FORMAT_RESEARCH.md`** - Serialization design, format comparisons
- **`LANGUAGE_SPECIFIC_NOTES.md`** - Per-language implementation details
- **`{TOPIC}_RESEARCH.md`** - Any other focused investigation

## Workflow for AI Assistants

See the **"Context Management & AI Workflow"** section in `CLAUDE.md` for detailed instructions.

### Quick Reference:

1. **Start session:** Read `CLAUDE.md` → Read `TODO.md`
2. **During work:** Update `TODO.md` task statuses, create/update RESEARCH.md files
3. **End session:** Ensure all statuses accurate, document blockers
4. **Research:** Create focused RESEARCH.md files with examples and version numbers

## File Naming Conventions

- `TODO.md` - Main task tracker (required)
- `{TOPIC}_RESEARCH.md` - Research findings (uppercase, descriptive)
- `README.md` - This file (explains the directory)

## Version Control

All `.context/` files **should be committed to git**. They are part of the project documentation and help maintain development continuity.

## Examples

### Good RESEARCH.md Structure

```markdown
# Tree-sitter Rust Grammar Research

**Last Updated:** 2025-10-31
**Grammar Version:** tree-sitter-rust 0.23

## Node Types for Symbol Extraction

### Functions
- Node kind: `function_item`
- Name field: `name` (identifier)
- Parameters: `parameters` (parameter_list)
- Example AST: ...

### Structs
- Node kind: `struct_item`
- Fields: ...

## Edge Cases

### Procedural Macros
...

## References
- https://github.com/tree-sitter/tree-sitter-rust
- Grammar docs: ...
```

### Good TODO.md Update

```markdown
#### P0: Tree-sitter Integration (CRITICAL PATH)
- [x] Set up Tree-sitter grammar dependencies (Line 83)
- [x] Add tree-sitter-rust to Cargo.toml - COMPLETED 2025-10-31
- [ ] Add tree-sitter-python to Cargo.toml
- [in_progress] Implement Rust parser (src/parsers/rust.rs)
  - Started: 2025-10-31
  - Status: Completed basic node traversal, working on macro handling
  - See: .context/RUST_PARSER_RESEARCH.md for findings
  - Blocker: Need to understand proc_macro AST representation
```

## Maintenance

- Keep TODO.md up to date with actual project state
- Archive completed RESEARCH.md files or integrate findings into docs
- Update timestamps when making significant changes
- Cross-reference between TODO.md and RESEARCH.md files

---

**Remember:** The `.context/` directory is your memory between sessions. Use it well!
