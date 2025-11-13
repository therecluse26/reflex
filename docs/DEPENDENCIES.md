# Dependency Tracking in Reflex

**Status:** Specification (Not Yet Implemented)
**Version:** 1.0
**Last Updated:** 2025-11-11

## Table of Contents

1. [Overview](#overview)
2. [Motivation](#motivation)
3. [Architecture](#architecture)
4. [Storage Schema](#storage-schema)
5. [Command Interface](#command-interface)
6. [Language Support](#language-support)
7. [Implementation Phases](#implementation-phases)
8. [Performance Characteristics](#performance-characteristics)
9. [Use Cases & Examples](#use-cases--examples)
10. [Future Enhancements](#future-enhancements)

---

## Overview

Reflex will support **file dependency tracking** to help developers and AI coding agents understand relationships between files in a codebase. This feature enables:

- **Impact Analysis**: "What breaks if I change this file?"
- **Context Understanding**: "What does this code depend on?"
- **Reverse Lookup**: "What uses this file/module?"
- **Architecture Analysis**: Finding circular dependencies, hotspots, and orphaned files

### Key Design Principles

1. **Index Shallow, Traverse Deep**: Store only direct (depth-1) dependencies; compute deeper relationships on demand
2. **Dual Command Approach**: Separate search augmentation (`rfx query --dependencies`) from graph analysis (`rfx deps`)
3. **Lazy Evaluation**: Compute expensive operations only when explicitly requested
4. **Cross-Language Consistency**: Single API works across all 18 supported languages
5. **AI-First Design**: Structured output optimized for AI coding agents

---

## Motivation

### Problem: AI Agents Need Dependency Context

When AI coding agents work with code, they need to understand:
- What a file imports (to understand its context)
- What imports a file (to assess change impact)
- Whether code is safe to delete (no incoming dependencies)
- How modules are interconnected (architectural understanding)

### Why Not Use grep/awk?

Traditional tools have limitations:
- **Slow**: Full codebase scan on every query (1-5 seconds)
- **Inaccurate**: False positives from comments, strings, logs
- **Language-Specific**: Different regex per language (error-prone)
- **No Resolution**: Returns raw import strings, not resolved paths
- **Manual Parsing**: Agent must parse and classify results

### Reflex Advantages

- **Fast**: Indexed lookups in <5ms (10-100x faster than grep)
- **Accurate**: Tree-sitter parsing eliminates false positives
- **Universal**: Same API across all 18 languages
- **Resolved**: Import paths resolved to actual files
- **Structured**: Clean JSON output, no manual parsing

---

## Architecture

### Core Concept: Depth-1 Storage

Reflex stores **only direct dependencies** (depth-1) in the database. Deeper relationships are computed on-demand by traversing the index.

```
Example Codebase:
  file_a.rs → [file_b.rs, file_c.rs]     (stored)
  file_b.rs → [file_d.rs, file_e.rs]     (stored)
  file_d.rs → [file_f.rs]                 (stored)

Depth-3 Query from file_a.rs:
  1. Lookup file_a.rs → get [file_b.rs, file_c.rs]
  2. Lookup file_b.rs → get [file_d.rs, file_e.rs]
  3. Lookup file_d.rs → get [file_f.rs]

Result: 6 lookups, all indexed (O(1) each)
```

### Why This Works

- **Storage**: O(n) instead of O(n²) for full closure
- **Updates**: Change one file, update one row
- **Flexibility**: Compute any depth on-demand
- **Performance**: Most files have 5-20 deps, not hundreds

### Component Integration

```
Existing Components:
  - Indexer: Parses files with tree-sitter
  - CacheManager: Manages SQLite database
  - QueryEngine: Executes searches

New Components:
  - DependencyExtractor: Extract imports from AST
  - DependencyIndex: Store/retrieve dependency relationships
  - DependencyTraverser: Compute transitive closures
  - GraphAnalyzer: Detect cycles, hotspots, islands
```

---

## Storage Schema

### Database Tables

#### file_dependencies Table

```sql
CREATE TABLE file_dependencies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_id INTEGER NOT NULL,           -- Source file (FK to files.id)
    imported_path TEXT NOT NULL,        -- Import as written in source
    resolved_file_id INTEGER,           -- Target file (FK to files.id), NULL if external
    import_type TEXT NOT NULL,          -- 'internal', 'external', 'stdlib'
    line_number INTEGER NOT NULL,       -- Line where import appears
    imported_symbols TEXT,              -- JSON array of specific symbols (if selective import)
    FOREIGN KEY (file_id) REFERENCES files(id) ON DELETE CASCADE,
    FOREIGN KEY (resolved_file_id) REFERENCES files(id) ON DELETE SET NULL
);

CREATE INDEX idx_deps_file ON file_dependencies(file_id);
CREATE INDEX idx_deps_resolved ON file_dependencies(resolved_file_id);
CREATE INDEX idx_deps_type ON file_dependencies(import_type);
```

### Import Types

- **internal**: Import references another file in the project
- **external**: Import from external package/library (node_modules, pip, cargo, etc.)
- **stdlib**: Standard library import (depends on language)

### Example Data

```sql
-- Rust: use std::collections::HashMap;
INSERT INTO file_dependencies (file_id, imported_path, resolved_file_id, import_type, line_number, imported_symbols)
VALUES (42, 'std::collections::HashMap', NULL, 'stdlib', 5, '["HashMap"]');

-- Rust: use crate::models::User;
INSERT INTO file_dependencies (file_id, imported_path, resolved_file_id, import_type, line_number, imported_symbols)
VALUES (42, 'crate::models::User', 15, 'internal', 7, '["User"]');

-- Python: import requests
INSERT INTO file_dependencies (file_id, imported_path, resolved_file_id, import_type, line_number, imported_symbols)
VALUES (55, 'requests', NULL, 'external', 3, NULL);

-- Python: from .utils import format_date
INSERT INTO file_dependencies (file_id, imported_path, resolved_file_id, import_type, line_number, imported_symbols)
VALUES (55, '.utils', 62, 'internal', 5, '["format_date"]');
```

---

## Command Interface

### Dual-Command Approach

Reflex provides **two separate commands** for different use cases:

#### 1. `rfx query --dependencies` (Search Augmentation)

Add dependency context to search results.

```bash
# Find all API-related code and see what it depends on
rfx query "ApiClient" --dependencies

# Output: Each search result includes immediate dependencies
{
  "path": "src/api/client.rs",
  "symbol": "ApiClient",
  "span": { "start_line": 15, "end_line": 45 },
  "dependencies": [
    {"path": "reqwest", "type": "external"},
    {"path": "src/config.rs", "type": "internal"},
    {"path": "src/models/user.rs", "type": "internal"}
  ]
}
```

**Characteristics:**
- Operates on search results (10-100 matches)
- Depth-1 only (immediate dependencies)
- Simple list format
- Answers: "What does this code need?"
- **Use case:** AI agents understanding code context

#### 2. `rfx deps` (Graph Analysis)

Dedicated command for dependency analysis and graph operations.

**Single File Operations:**
```bash
# Show dependency tree
rfx deps src/main.rs

# Show who depends on this file (reverse lookup)
rfx deps src/config.rs --reverse

# Deep traversal (compute transitive closure)
rfx deps src/api.rs --depth 3

# Show as tree visualization
rfx deps src/main.rs --tree

# Filter to internal dependencies only
rfx deps src/api.rs --only-internal

# Filter to external dependencies only
rfx deps src/api.rs --only-external
```

**Graph-Wide Operations:**
```bash
# Find circular dependencies
rfx deps --circular

# Find most imported files (hotspots)
rfx deps --hotspots

# Find files that nothing imports (orphans)
rfx deps --unused

# Find disconnected components
rfx deps --islands

# Full analysis report
rfx deps --analyze
```

**Output Formats:**
```bash
# JSON (default, for programmatic use)
rfx deps src/main.rs --json

# Tree visualization (for terminal)
rfx deps src/main.rs --tree

# Graphviz DOT format (for visualization tools)
rfx deps src/main.rs --format dot > deps.dot
dot -Tpng deps.dot -o deps.png
```

**Characteristics:**
- Operates on specific files or entire graph
- Any depth (traversal-based)
- Rich output formats
- Graph algorithms (cycles, hotspots, etc.)
- **Use case:** Architecture analysis, refactoring planning

---

## Language Support

Reflex will extract dependencies for all 18 supported languages using Tree-sitter.

### Import/Dependency Syntax by Language

#### Rust
```rust
// Module imports
use std::collections::HashMap;           // stdlib
use crate::models::User;                 // internal (absolute)
use super::utils;                        // internal (relative)
use self::nested::Module;                // internal (self-relative)
mod my_module;                           // module declaration
extern crate serde;                      // external crate (Rust 2015)

// Tree-sitter patterns:
// - use_declaration
// - mod_item
// - extern_crate_declaration
```

#### Python
```python
import os                                 # stdlib
import requests                           # external
from typing import List, Optional         # stdlib
from .utils import format_date            # internal (relative)
from ..models import User                 # internal (parent)
from mypackage.module import func         # internal (absolute)

# Tree-sitter patterns:
# - import_statement
# - import_from_statement
```

#### JavaScript / TypeScript
```javascript
import React from 'react';                // external
import { useState } from 'react';         // external (named)
import * as utils from './utils';         // internal
import type { User } from './types';      // TS type import
const fs = require('fs');                 // CommonJS (stdlib)
const express = require('express');       // CommonJS (external)

// Tree-sitter patterns:
// - import_statement
// - import_clause
// - call_expression (for require)
```

#### Go
```go
import "fmt"                              // stdlib
import "github.com/user/repo/pkg"         // external
import . "math"                           // dot import
import _ "database/sql"                   // blank import
import (
    "context"
    "myapp/internal/models"               // internal
)

// Tree-sitter patterns:
// - import_declaration
// - import_spec
```

#### Java
```java
import java.util.List;                    // stdlib
import java.util.*;                       // wildcard
import com.example.myapp.User;            // internal
import org.springframework.boot.*;         // external

// Tree-sitter patterns:
// - import_declaration
```

#### C / C++
```c
#include <stdio.h>                        // stdlib (angle brackets)
#include "my_header.h"                    // internal (quotes)
#include <vector>                         // stdlib (C++)
#include "../common/utils.h"              // internal (relative)

// Tree-sitter patterns:
// - preproc_include
```

#### C#
```csharp
using System;                             // stdlib
using System.Collections.Generic;         // stdlib
using MyApp.Models;                       // internal
using static System.Math;                 // static using

// Tree-sitter patterns:
// - using_directive
```

#### PHP
```php
<?php
use PDO;                                  // stdlib
use Symfony\Component\HttpFoundation\Request;  // external
use App\Models\User;                      // internal
require_once 'config.php';                // include (internal)
include __DIR__ . '/utils.php';          // include (internal)

// Tree-sitter patterns:
// - namespace_use_declaration
// - require_expression
// - include_expression
```

#### Ruby
```ruby
require 'json'                            # stdlib
require 'rails'                           # external (gem)
require_relative '../lib/utils'          # internal (relative)
load 'config.rb'                          # load (internal)

# Tree-sitter patterns:
# - call (require/require_relative/load)
```

#### Kotlin
```kotlin
import java.util.Date                     // stdlib
import kotlin.collections.*               // stdlib (wildcard)
import com.example.myapp.User             // internal
import androidx.appcompat.app.AppCompatActivity  // external

// Tree-sitter patterns:
// - import_header
```

#### Zig
```zig
const std = @import("std");               // stdlib
const utils = @import("utils.zig");       // internal
const lib = @import("external_lib");      // external

// Tree-sitter patterns:
// - call_expression (@import)
```

#### Vue
```vue
<script>
import { ref } from 'vue'                 // external
import MyComponent from './MyComponent.vue'  // internal
import { api } from '@/services/api'      // internal (alias)
</script>

<script setup lang="ts">
import type { User } from '@/types'       // internal type import
</script>

// Parsing: Extract from <script> blocks using line-based parsing
```

#### Svelte
```svelte
<script>
import { onMount } from 'svelte'          // external
import Component from './Component.svelte'  // internal
import { store } from '../stores'         // internal
</script>

// Parsing: Extract from <script> blocks using line-based parsing
```

### Path Resolution Strategies

#### Simple (No Resolution Needed)
- **Go**: Import paths are fully qualified
- **Java**: Package names are absolute
- **Kotlin**: Import paths are absolute

#### Relative Path Resolution
- **JavaScript/TypeScript**: Resolve `./`, `../`, `@/` aliases
- **Python**: Resolve relative imports (`.`, `..`)
- **Rust**: Resolve `super::`, `self::`, `crate::`
- **C/C++**: Resolve `#include` with header search paths

#### Package Manager Integration
- **JavaScript/TypeScript**: Check `node_modules/`, `package.json`
- **Python**: Check virtual environment, `site-packages`
- **Rust**: Check `Cargo.toml` for workspace members
- **Ruby**: Check `Gem` paths
- **PHP**: Check `vendor/` (Composer)

---

## Implementation Phases

### Phase 1: Core Infrastructure (P1 - High Priority)

**Goal:** Basic dependency extraction and storage

**Tasks:**
1. Add `file_dependencies` table to SQLite schema (src/cache.rs)
2. Create `DependencyExtractor` trait (src/parsers/mod.rs)
3. Implement `DependencyIndex` for storage/retrieval (src/dependency.rs)
4. Add `--dependencies` flag to `rfx query` command
5. Extend `SearchResult` model to include dependencies

**Deliverables:**
- `rfx query "pattern" --dependencies` works for at least one language (Rust)
- Dependencies stored in SQLite
- JSON output includes dependency list

**Estimated Time:** 3-5 days

### Phase 2: Language Parser Updates (P1 - High Priority)

**Goal:** Extract imports for all 18 languages

**Tier 1 - Simple Syntax (1-2 days each):**
1. **Rust**: `use`, `mod`, `extern crate`
2. **Python**: `import`, `from...import`
3. **JavaScript/TypeScript**: `import`, `require`
4. **Go**: `import`
5. **Java**: `import`
6. **C#**: `using`

**Tier 2 - Path Resolution (2-3 days each):**
7. **Ruby**: `require`, `require_relative`
8. **PHP**: `use`, `require`, `include`
9. **C/C++**: `#include` with search paths
10. **Kotlin**: `import`

**Tier 3 - Framework-Specific (2-3 days each):**
11. **Vue**: Component imports in `<script>` blocks
12. **Svelte**: Component imports
13. **Zig**: `@import`

**Deliverables:**
- Each language parser extracts import statements
- Tree-sitter queries identify import nodes
- Basic path classification (internal vs external)
- Unit tests for each parser

**Estimated Time:** 12-15 days total

### Phase 3: Query Engine Integration (P1 - High Priority)

**Goal:** Augment search results with dependency info

**Tasks:**
1. Add dependency loading to `QueryEngine`
2. Implement `enrich_with_dependencies()` for results
3. Add `--only-internal` / `--only-external` filters
4. Add `--imported-by` flag (reverse lookup)
5. Optimize queries with indexes

**Deliverables:**
- `rfx query "pattern" --dependencies` returns enriched results
- `rfx query "pattern" --dependencies --only-internal` filters correctly
- Fast indexed lookups (<5ms overhead per result)

**Estimated Time:** 2-3 days

### Phase 4: Dependency Command (P2 - Medium Priority)

**Goal:** Dedicated `rfx deps` command for graph analysis

**Tasks:**
1. Create `rfx deps` CLI command (src/cli.rs)
2. Implement single-file operations:
   - `rfx deps <file>` (show dependencies)
   - `rfx deps <file> --reverse` (show dependents)
   - `rfx deps <file> --depth N` (traversal)
3. Implement tree visualization format
4. Add JSON output for programmatic use
5. Implement `--only-internal` / `--only-external` filters

**Deliverables:**
- `rfx deps src/main.rs` shows dependency tree
- `rfx deps src/config.rs --reverse` shows all importers
- `rfx deps src/api.rs --depth 3` traverses correctly
- Tree format renders in terminal

**Estimated Time:** 3-4 days

### Phase 5: Graph Algorithms (P2 - Medium Priority)

**Goal:** Advanced graph analysis features

**Tasks:**
1. Implement circular dependency detection (DFS-based)
2. Implement hotspot detection (most imported files)
3. Implement unused file detection (no incoming edges)
4. Implement island detection (connected components)
5. Create `--analyze` flag (run all analyses)

**Deliverables:**
- `rfx deps --circular` finds cycles
- `rfx deps --hotspots` ranks by import count
- `rfx deps --unused` finds orphaned files
- `rfx deps --islands` identifies components
- `rfx deps --analyze` generates comprehensive report

**Estimated Time:** 3-4 days

### Phase 6: Advanced Features (P3 - Low Priority)

**Goal:** Visualization and advanced path resolution

**Tasks:**
1. Graphviz DOT format output (`--format dot`)
2. Advanced path resolution (package managers)
3. Workspace/monorepo support
4. Selective symbol imports tracking
5. Performance optimizations (session caching)

**Deliverables:**
- `rfx deps src/main.rs --format dot | dot -Tpng > graph.png`
- Accurate resolution for complex projects
- Sub-millisecond repeated queries (cached)

**Estimated Time:** 5-7 days

**Total Estimated Time:** 28-38 days (5-7 weeks)

---

## Performance Characteristics

### Storage Overhead

**Per 10k-file project:**
- Average 10 imports per file = 100k dependency entries
- Each entry: ~100 bytes (path, type, line, symbols)
- **Total:** ~10MB in SQLite

**Compared to:**
- Trigram index: ~50MB
- Content store: ~200MB
- **Dependency storage:** 5% overhead

### Indexing Impact

**Without dependency extraction:**
- Parse file with tree-sitter: 5ms per file
- Extract symbols: 2ms per file
- **Total:** 7ms per file

**With dependency extraction:**
- Parse file with tree-sitter: 5ms per file (same)
- Extract symbols: 2ms per file (same)
- Extract imports: 1ms per file (additional)
- **Total:** 8ms per file

**Impact:** ~15% slower indexing (acceptable tradeoff)

### Query Performance

#### rfx query --dependencies
```
Base query: 2-5ms (trigram search)
Enrich with deps: +0.5-2ms per result (indexed lookup)
Total: 3-7ms for 10 results
```

#### rfx deps <file>
```
Depth 1: 1-2ms (single indexed query)
Depth 2: 5-15ms (recursive lookups, ~10 files)
Depth 3: 10-30ms (exponential, ~50 files)
```

#### rfx deps --circular
```
Full graph traversal: 50-200ms (10k files)
DFS-based cycle detection
Worst case: O(V + E) where V = files, E = imports
```

#### rfx deps --hotspots
```
SQL COUNT(*) GROUP BY: 10-50ms
Already indexed, very fast
```

### Comparison to grep

**Reverse dependency lookup:**
```
grep -r "config.rs" src/  # Full scan
→ 1-5 seconds (10k files)
→ Many false positives

rfx deps src/config.rs --reverse  # Indexed
→ 2-5ms (exact matches only)
→ No false positives
```

**Speedup:** 200-1000x faster than grep

---

## Use Cases & Examples

### Use Case 1: Safe Refactoring

**Scenario:** Rename `User` interface to `UserProfile`

**Without deps:**
```bash
# Agent searches and renames blindly
rfx query "interface User"
# Might miss indirect usages, breaks 30 files
```

**With deps:**
```bash
# Agent checks impact first
rfx query "interface User" --dependencies
# Sees imported by: auth.ts, api.ts, profile.ts, etc. (30 files)

rfx deps src/models/user.ts --reverse
# Gets complete list of all importers
# Makes informed decision about scope
```

### Use Case 2: Understanding Unfamiliar Code

**Scenario:** "How does authentication work?"

**Without deps:**
```bash
# Agent reads each file blindly
rfx query "auth" --symbols
# Finds 20 functions, reads all
```

**With deps:**
```bash
# Agent understands the stack immediately
rfx query "authenticateUser" --dependencies --json
{
  "symbol": "authenticateUser",
  "dependencies": [
    {"path": "jsonwebtoken", "type": "external"},
    {"path": "src/models/user.ts", "type": "internal"},
    {"path": "src/cache/redis.ts", "type": "internal"}
  ]
}
# "Ah, JWT tokens stored in Redis with User model"
```

### Use Case 3: Dead Code Detection

**Scenario:** "Can I delete this old module?"

```bash
rfx deps src/legacy/old-api.ts --reverse
# Returns: No files import this
# Safe to delete!
```

### Use Case 4: Architecture Audit

**Scenario:** Check for problematic patterns

```bash
# Find circular dependencies
rfx deps --circular
→ Found 2 cycles:
  src/user.ts ↔ src/profile.ts
  src/api.ts → src/db.ts → src/models.ts → src/api.ts

# Find hotspots (files with too many dependents)
rfx deps --hotspots --limit 10
→ src/config.ts (47 imports) ⚠️ God object
  src/utils/format.ts (33 imports)
  src/types.ts (28 imports)

# Find unused files
rfx deps --unused
→ 5 orphaned files:
  src/old/unused.ts
  src/temp/test.ts
  ...
```

### Use Case 5: Debugging Import Errors

**Scenario:** Build failing due to import error

```bash
# Find what imports the problematic module
rfx query "UserService" --dependencies
→ Shows: imports './models/User'
→ But User.ts was renamed to UserModel.ts
→ "Found the issue!"
```

### Use Case 6: Find Usage Examples

**Scenario:** "How do I use the Logger class?"

```bash
# Find all files that import Logger
rfx deps src/logger.ts --reverse --limit 5
→ Returns 5 example files
→ Agent reads them to understand usage patterns
```

---

## Future Enhancements

### Phase 7: Potential Additions (Not Planned)

#### Call Graph (Limited)
Track function calls within files for impact analysis.
- **Complexity:** High (requires full AST traversal)
- **Value:** Medium (most impact already covered by file-level deps)
- **Verdict:** Defer until user demand

#### Monorepo / Workspace Support
Handle multi-package repositories (npm workspaces, Cargo workspaces).
- **Complexity:** Medium (resolve cross-package imports)
- **Value:** High for monorepo users
- **Verdict:** Consider for Phase 6

#### Dependency Change Tracking
Show dependency diffs between git branches.
- **Complexity:** Medium (requires git integration)
- **Value:** Medium (nice for PRs)
- **Verdict:** Future enhancement

#### Interactive Dependency Explorer (TUI)
Terminal UI for browsing dependency graph.
- **Complexity:** Medium (ratatui integration)
- **Value:** Medium (exploratory analysis)
- **Verdict:** Aligns with existing Interactive Mode TODO

---

## Testing Strategy

### Unit Tests

**Per-Language Parser Tests:**
- Extract imports correctly
- Handle multiline imports
- Classify internal vs external
- Resolve relative paths
- Edge cases (comments, strings, etc.)

**Dependency Index Tests:**
- Store and retrieve dependencies
- Handle cycles gracefully
- Performance (indexed lookups <5ms)

**Graph Algorithm Tests:**
- Circular dependency detection
- Transitive closure computation
- Hotspot ranking
- Island detection

### Integration Tests

**End-to-End Workflows:**
1. Index project with deps → Query with --dependencies → Verify results
2. Index project → `rfx deps` traversal → Verify depth correctness
3. Index project → Modify file → Incremental reindex → Verify dep updates
4. Multi-language project → Verify cross-language deps work

**Performance Tests:**
- Indexing overhead: <20% slowdown
- Query enrichment: <2ms per result
- Reverse lookup: <5ms for 50 dependents
- Graph traversal: <200ms for 10k files

### Real-World Validation

Test on actual projects:
- **Reflex itself** (Rust, ~5k LOC)
- **Next.js project** (TypeScript/JavaScript, ~10k LOC)
- **Django project** (Python, ~15k LOC)
- **Large monorepo** (Mixed languages, ~50k LOC)

---

## Implementation Checklist

### Phase 1: Core Infrastructure
- [ ] Add `file_dependencies` table to schema
- [ ] Create `DependencyExtractor` trait
- [ ] Implement `DependencyIndex` struct
- [ ] Add `--dependencies` flag to `rfx query`
- [ ] Extend `SearchResult` with dependencies field
- [ ] Tests: Basic storage and retrieval

### Phase 2: Language Parsers (18 languages)
- [ ] Rust: `use`, `mod`, `extern crate`
- [ ] Python: `import`, `from...import`
- [ ] JavaScript: `import`, `require`
- [ ] TypeScript: `import`, `require`, type imports
- [ ] Go: `import`
- [ ] Java: `import`
- [ ] C: `#include`
- [ ] C++: `#include`
- [ ] C#: `using`
- [ ] PHP: `use`, `require`, `include`
- [ ] Ruby: `require`, `require_relative`
- [ ] Kotlin: `import`
- [ ] Zig: `@import`
- [ ] Vue: Extract from `<script>` blocks
- [ ] Svelte: Extract from `<script>` blocks
- [ ] Tests: Per-language import extraction

### Phase 3: Query Integration
- [ ] Load dependencies in `QueryEngine`
- [ ] Implement `enrich_with_dependencies()`
- [ ] Add `--only-internal` filter
- [ ] Add `--only-external` filter
- [ ] Add `--imported-by` flag
- [ ] Tests: Query enrichment correctness

### Phase 4: Dependency Command
- [ ] Create `rfx deps` CLI command
- [ ] Implement `rfx deps <file>`
- [ ] Implement `rfx deps <file> --reverse`
- [ ] Implement `rfx deps <file> --depth N`
- [ ] Implement `--tree` format
- [ ] Implement `--json` format
- [ ] Tests: Single-file operations

### Phase 5: Graph Algorithms
- [ ] Implement circular dependency detection
- [ ] Implement `rfx deps --circular`
- [ ] Implement `rfx deps --hotspots`
- [ ] Implement `rfx deps --unused`
- [ ] Implement `rfx deps --islands`
- [ ] Implement `rfx deps --analyze`
- [ ] Tests: Graph algorithms correctness

### Phase 6: Advanced Features
- [ ] Implement `--format dot` (Graphviz)
- [ ] Advanced path resolution (node_modules, etc.)
- [ ] Session caching for repeated queries
- [ ] Workspace/monorepo support
- [ ] Tests: Advanced features

---

## Conclusion

Dependency tracking is a **high-value addition** to Reflex that significantly enhances its usefulness for AI coding agents and human developers. The depth-1 storage model with on-demand traversal provides an optimal balance of:

- **Performance**: Fast indexed lookups with minimal storage overhead
- **Flexibility**: Compute any depth without re-indexing
- **Simplicity**: Clean architecture, easy to maintain
- **Completeness**: Works across all 18 supported languages

**Recommended Implementation Path:**
1. Start with Phase 1 (Core Infrastructure) for quick wins
2. Roll out Phase 2 (Language Parsers) incrementally
3. Ship Phase 3 (Query Integration) for AI agent value
4. Consider Phase 4-5 (Advanced Analysis) based on user feedback

**Total Effort:** 5-7 weeks for complete implementation (all phases)

**Expected Impact:** 10-100x faster dependency lookups compared to grep, with accurate results and structured output optimized for AI agents.

---

## IMPLEMENTATION PLAN (2025-11-12)

### Current Status Assessment

#### ✅ COMPLETE (100%)
- **Database schema** (`file_dependencies` table) - fully implemented in `src/cache.rs`
- **Dependency storage** (`DependencyIndex`) - complete in `src/dependency.rs` (963 lines)
- **Graph algorithms** - all core algorithms implemented:
  - Direct dependencies: `get_dependencies(file_id)`
  - Reverse lookup: `get_dependents(file_id)`
  - Transitive deps: `get_transitive_deps(file_id, max_depth)` (BFS)
  - Circular detection: `detect_circular_dependencies()` (DFS)
  - Hotspot detection: `find_hotspots(limit)`
  - Unused files: `find_unused_files()`
  - Path resolution: `get_file_paths()`, `get_file_path()`
  - Batch operations: `batch_insert_dependencies()`, `clear_dependencies()`
- **Import extraction** - working for all languages:
  - Rust: `src/parsers/rust.rs::extract_rust_imports()`
  - Python: `src/parsers/python.rs::extract_python_imports()`
  - TypeScript/JavaScript: `src/parsers/typescript.rs::extract_typescript_imports()`
  - Go: `src/parsers/go.rs::extract_go_imports()`
  - Java: `src/parsers/java.rs::extract_java_imports()`
  - Kotlin: `src/parsers/kotlin.rs::extract_kotlin_imports()`
  - Zig: `src/parsers/zig.rs::extract_zig_imports()`
  - Vue: `src/parsers/vue.rs::extract_vue_imports()`
  - Svelte: `src/parsers/svelte.rs::extract_svelte_imports()`
- **Dependency classification** - working for all languages:
  - Internal/External/Stdlib classification
  - Package-based reclassification (Rust, Python, Java, Kotlin, Ruby)
  - Indexer integration in `src/indexer.rs`
- **Query integration** - `--dependencies` flag implemented:
  - CLI support: `rfx query "pattern" --dependencies`
  - HTTP API support: `GET /query?dependencies=true`
  - MCP support (via query filter)

#### ❌ MISSING (0%)
- **`rfx deps` CLI command** - NOT in `src/cli.rs::Command` enum
- **Output formatters** - no tree/DOT/table formatters for deps
- **Command handlers** - no `handle_deps()` function
- **Testing** - no integration tests for full workflow

---

### Implementation Strategy

#### Phase 1: Basic `rfx deps` Command (MVP)
**Goal:** Get `rfx deps <file>` working with JSON output

**Tasks:**
1. Add `Deps` variant to `Command` enum in `src/cli.rs`
2. Create `handle_deps()` function
3. Implement basic file operations:
   - `rfx deps <file>` - show direct dependencies
   - `rfx deps <file> --reverse` - show dependents
   - `rfx deps <file> --depth N` - transitive traversal
4. JSON output only (reuse existing models)

**Files to modify:**
- `src/cli.rs` (lines 34-297: Command enum, lines 299-346: execute() match)

**Estimated time:** 2-3 hours

#### Phase 2: Output Formatters
**Goal:** Add tree, table, and DOT format output

**Tasks:**
1. Create `src/dependency_formatter.rs` module
2. Implement formatters:
   - `format_tree()` - tree visualization with Unicode box drawing
   - `format_table()` - tabular output for hotspots/unused
   - `format_dot()` - Graphviz DOT format
3. Add `--tree` and `--format` flags to deps command
4. Pretty-print for human readability

**New file:**
- `src/dependency_formatter.rs` (~300 lines)

**Files to modify:**
- `src/cli.rs` (add format flags)
- `src/lib.rs` (add `pub mod dependency_formatter;`)

**Estimated time:** 3-4 hours

#### Phase 3: Graph-Wide Analysis Commands
**Goal:** Implement `--circular`, `--hotspots`, `--unused`, `--analyze`

**Tasks:**
1. Extend `handle_deps()` with graph-wide operations
2. Implement subcommands:
   - `rfx deps --circular` - detect and display cycles
   - `rfx deps --hotspots` - show most-imported files
   - `rfx deps --unused` - find orphaned files
   - `rfx deps --analyze` - comprehensive report
3. Add `--limit` flag for controlling result count
4. Pretty formatting for each operation

**Files to modify:**
- `src/cli.rs` (add graph-wide flags)

**Estimated time:** 2-3 hours

#### Phase 4: Testing & Documentation
**Goal:** Ensure correctness and usability

**Tasks:**
1. Add integration tests in `tests/deps_test.rs`
2. Test on real codebases (Reflex itself, test projects)
3. Update CLAUDE.md with `rfx deps` examples
4. Add `--help` examples to CLI

**New file:**
- `tests/deps_test.rs` (~200 lines)

**Files to modify:**
- `CLAUDE.md` (add deps command documentation)
- `src/cli.rs` (improve help text)

**Estimated time:** 2-3 hours

---

### Detailed Implementation Specification

#### 1. CLI Command Structure (`src/cli.rs`)

**Add to Command enum (after Line 289):**

```rust
/// Analyze file dependencies and imports
///
/// Show dependencies, dependents, and perform graph analysis
/// to understand code relationships and architecture.
///
/// Examples:
///   rfx deps src/main.rs                  # Show dependencies
///   rfx deps src/config.rs --reverse      # Show dependents
///   rfx deps src/api.rs --depth 3         # Transitive deps
///   rfx deps --circular                   # Find cycles
///   rfx deps --hotspots                   # Most-imported files
///   rfx deps --unused                     # Orphaned files
Deps {
    /// File path to analyze (omit for graph-wide operations)
    file: Option<PathBuf>,

    /// Show files that depend on this file (reverse lookup)
    #[arg(short, long)]
    reverse: bool,

    /// Traversal depth for transitive dependencies (default: 1)
    #[arg(short, long, default_value = "1")]
    depth: usize,

    /// Output format: json (default), tree, table, dot
    #[arg(short = 'f', long, default_value = "json")]
    format: String,

    /// Pretty-print JSON output (only with --format json)
    #[arg(long)]
    pretty: bool,

    /// Filter to internal dependencies only
    #[arg(long)]
    only_internal: bool,

    /// Filter to external dependencies only
    #[arg(long)]
    only_external: bool,

    /// Filter to stdlib dependencies only
    #[arg(long)]
    only_stdlib: bool,

    /// Find circular dependencies (graph-wide)
    #[arg(long, conflicts_with = "file")]
    circular: bool,

    /// Find most-imported files (hotspots)
    #[arg(long, conflicts_with = "file")]
    hotspots: bool,

    /// Find unused files (no incoming dependencies)
    #[arg(long, conflicts_with = "file")]
    unused: bool,

    /// Find disconnected components (islands)
    #[arg(long, conflicts_with = "file")]
    islands: bool,

    /// Full analysis report (runs all analyses)
    #[arg(long, conflicts_with = "file")]
    analyze: bool,

    /// Maximum number of results to return
    #[arg(short = 'n', long)]
    limit: Option<usize>,
}
```

**Add to execute() match (after Line 342):**

```rust
Some(Command::Deps { file, reverse, depth, format, pretty, only_internal, only_external, only_stdlib, circular, hotspots, unused, islands, analyze, limit }) => {
    handle_deps(file, reverse, depth, format, pretty, only_internal, only_external, only_stdlib, circular, hotspots, unused, islands, analyze, limit)
}
```

#### 2. Command Handler (`src/cli.rs`, after Line 1427)

**Add handler function:**

```rust
/// Handle the `deps` subcommand
fn handle_deps(
    file: Option<PathBuf>,
    reverse: bool,
    depth: usize,
    format: String,
    pretty_json: bool,
    only_internal: bool,
    only_external: bool,
    only_stdlib: bool,
    circular: bool,
    hotspots: bool,
    unused: bool,
    islands: bool,
    analyze: bool,
    limit: Option<usize>,
) -> Result<()> {
    use crate::dependency::DependencyIndex;
    use crate::models::ImportType;

    log::info!("Starting deps command");

    let cache = CacheManager::new(".");

    if !cache.exists() {
        anyhow::bail!(
            "No index found in current directory.\n\
             \n\
             Run 'rfx index' to build the code search index first.\n\
             \n\
             Example:\n\
             $ rfx index          # Index current directory\n\
             $ rfx deps <file>    # Analyze dependencies"
        );
    }

    let deps_index = DependencyIndex::new(cache);

    // Determine operation mode
    if analyze {
        // Run full analysis
        return handle_deps_analyze(&deps_index, &format, pretty_json, limit);
    } else if circular {
        return handle_deps_circular(&deps_index, &format, pretty_json);
    } else if hotspots {
        return handle_deps_hotspots(&deps_index, &format, pretty_json, limit);
    } else if unused {
        return handle_deps_unused(&deps_index, &format, pretty_json, limit);
    } else if islands {
        return handle_deps_islands(&deps_index, &format, pretty_json);
    }

    // File-based operations require a file path
    let file_path = file.ok_or_else(|| {
        anyhow::anyhow!(
            "No file specified.\n\
             \n\
             Usage:\n\
             $ rfx deps <file>              # Show dependencies\n\
             $ rfx deps <file> --reverse    # Show dependents\n\
             $ rfx deps --circular          # Find cycles\n\
             $ rfx deps --hotspots          # Most-imported files"
        )
    })?;

    // Convert file path to string
    let file_str = file_path.to_string_lossy().to_string();

    // Get file ID
    let file_id = deps_index.get_file_id_by_path(&file_str)?
        .ok_or_else(|| anyhow::anyhow!("File '{}' not found in index", file_str))?;

    // Filter function based on flags
    let import_filter = move |import_type: &ImportType| -> bool {
        if only_internal && *import_type != ImportType::Internal {
            return false;
        }
        if only_external && *import_type != ImportType::External {
            return false;
        }
        if only_stdlib && *import_type != ImportType::Stdlib {
            return false;
        }
        true
    };

    if reverse {
        // Show dependents (who imports this file)
        let dependents = deps_index.get_dependents(file_id)?;
        let paths = deps_index.get_file_paths(&dependents)?;

        match format.as_str() {
            "json" => {
                let output: Vec<_> = dependents.iter()
                    .filter_map(|id| paths.get(id).map(|path| serde_json::json!({
                        "file_id": id,
                        "path": path,
                    })))
                    .collect();

                let json_str = if pretty_json {
                    serde_json::to_string_pretty(&output)?
                } else {
                    serde_json::to_string(&output)?
                };
                println!("{}", json_str);
                eprintln!("Found {} files that import {}", dependents.len(), file_str);
            }
            "tree" => {
                println!("Files that import {}:", file_str);
                for (id, path) in &paths {
                    if dependents.contains(id) {
                        println!("  └─ {}", path);
                    }
                }
                eprintln!("\nFound {} dependents", dependents.len());
            }
            "table" => {
                println!("ID     Path");
                println!("-----  ----");
                for id in &dependents {
                    if let Some(path) = paths.get(id) {
                        println!("{:<5}  {}", id, path);
                    }
                }
                eprintln!("\nFound {} dependents", dependents.len());
            }
            _ => {
                anyhow::bail!("Unknown format '{}'. Supported: json, tree, table, dot", format);
            }
        }
    } else {
        // Show dependencies (what this file imports)
        if depth == 1 {
            // Direct dependencies only
            let deps = deps_index.get_dependencies(file_id)?;
            let filtered_deps: Vec<_> = deps.into_iter()
                .filter(|d| import_filter(&d.import_type))
                .collect();

            match format.as_str() {
                "json" => {
                    let output: Vec<_> = filtered_deps.iter()
                        .map(|dep| serde_json::json!({
                            "imported_path": dep.imported_path,
                            "resolved_file_id": dep.resolved_file_id,
                            "import_type": match dep.import_type {
                                ImportType::Internal => "internal",
                                ImportType::External => "external",
                                ImportType::Stdlib => "stdlib",
                            },
                            "line": dep.line_number,
                            "symbols": dep.imported_symbols,
                        }))
                        .collect();

                    let json_str = if pretty_json {
                        serde_json::to_string_pretty(&output)?
                    } else {
                        serde_json::to_string(&output)?
                    };
                    println!("{}", json_str);
                    eprintln!("Found {} dependencies for {}", filtered_deps.len(), file_str);
                }
                "tree" => {
                    println!("Dependencies of {}:", file_str);
                    for dep in &filtered_deps {
                        let type_label = match dep.import_type {
                            ImportType::Internal => "[internal]",
                            ImportType::External => "[external]",
                            ImportType::Stdlib => "[stdlib]",
                        };
                        println!("  └─ {} {} (line {})", dep.imported_path, type_label, dep.line_number);
                    }
                    eprintln!("\nFound {} dependencies", filtered_deps.len());
                }
                "table" => {
                    println!("Path                          Type       Line");
                    println!("----------------------------  ---------  ----");
                    for dep in &filtered_deps {
                        let type_str = match dep.import_type {
                            ImportType::Internal => "internal",
                            ImportType::External => "external",
                            ImportType::Stdlib => "stdlib",
                        };
                        println!("{:<28}  {:<9}  {}", dep.imported_path, type_str, dep.line_number);
                    }
                    eprintln!("\nFound {} dependencies", filtered_deps.len());
                }
                _ => {
                    anyhow::bail!("Unknown format '{}'. Supported: json, tree, table, dot", format);
                }
            }
        } else {
            // Transitive dependencies (depth > 1)
            let transitive = deps_index.get_transitive_deps(file_id, depth)?;
            let file_ids: Vec<_> = transitive.keys().copied().collect();
            let paths = deps_index.get_file_paths(&file_ids)?;

            match format.as_str() {
                "json" => {
                    let output: Vec<_> = transitive.iter()
                        .filter_map(|(id, d)| {
                            paths.get(id).map(|path| serde_json::json!({
                                "file_id": id,
                                "path": path,
                                "depth": d,
                            }))
                        })
                        .collect();

                    let json_str = if pretty_json {
                        serde_json::to_string_pretty(&output)?
                    } else {
                        serde_json::to_string(&output)?
                    };
                    println!("{}", json_str);
                    eprintln!("Found {} transitive dependencies (depth {})", transitive.len(), depth);
                }
                "tree" => {
                    println!("Transitive dependencies of {} (depth {}):", file_str, depth);
                    // Group by depth for tree display
                    let mut by_depth: std::collections::HashMap<usize, Vec<i64>> = std::collections::HashMap::new();
                    for (id, d) in &transitive {
                        by_depth.entry(*d).or_insert_with(Vec::new).push(*id);
                    }

                    for depth_level in 0..=depth {
                        if let Some(ids) = by_depth.get(&depth_level) {
                            let indent = "  ".repeat(depth_level);
                            for id in ids {
                                if let Some(path) = paths.get(id) {
                                    if depth_level == 0 {
                                        println!("{}{} (self)", indent, path);
                                    } else {
                                        println!("{}└─ {}", indent, path);
                                    }
                                }
                            }
                        }
                    }
                    eprintln!("\nFound {} transitive dependencies", transitive.len());
                }
                "table" => {
                    println!("Depth  File ID  Path");
                    println!("-----  -------  ----");
                    let mut sorted: Vec<_> = transitive.iter().collect();
                    sorted.sort_by_key(|(_, d)| *d);
                    for (id, d) in sorted {
                        if let Some(path) = paths.get(id) {
                            println!("{:<5}  {:<7}  {}", d, id, path);
                        }
                    }
                    eprintln!("\nFound {} transitive dependencies", transitive.len());
                }
                _ => {
                    anyhow::bail!("Unknown format '{}'. Supported: json, tree, table, dot", format);
                }
            }
        }
    }

    Ok(())
}

/// Handle --circular flag (detect cycles)
fn handle_deps_circular(
    deps_index: &DependencyIndex,
    format: &str,
    pretty_json: bool,
) -> Result<()> {
    let cycles = deps_index.detect_circular_dependencies()?;

    if cycles.is_empty() {
        println!("No circular dependencies found.");
        return Ok(());
    }

    match format {
        "json" => {
            let file_ids: Vec<i64> = cycles.iter().flat_map(|c| c.iter()).copied().collect();
            let paths = deps_index.get_file_paths(&file_ids)?;

            let output: Vec<_> = cycles.iter()
                .map(|cycle| {
                    let cycle_paths: Vec<_> = cycle.iter()
                        .filter_map(|id| paths.get(id).cloned())
                        .collect();
                    serde_json::json!({
                        "file_ids": cycle,
                        "paths": cycle_paths,
                    })
                })
                .collect();

            let json_str = if pretty_json {
                serde_json::to_string_pretty(&output)?
            } else {
                serde_json::to_string(&output)?
            };
            println!("{}", json_str);
            eprintln!("Found {} circular dependencies", cycles.len());
        }
        "tree" => {
            println!("Circular Dependencies Found:");
            let file_ids: Vec<i64> = cycles.iter().flat_map(|c| c.iter()).copied().collect();
            let paths = deps_index.get_file_paths(&file_ids)?;

            for (idx, cycle) in cycles.iter().enumerate() {
                println!("\nCycle {}:", idx + 1);
                for id in cycle {
                    if let Some(path) = paths.get(id) {
                        println!("  → {}", path);
                    }
                }
                // Show cycle completion
                if let Some(first_id) = cycle.first() {
                    if let Some(path) = paths.get(first_id) {
                        println!("  → {} (cycle completes)", path);
                    }
                }
            }
            eprintln!("\nFound {} cycles", cycles.len());
        }
        "table" => {
            println!("Cycle  Files in Cycle");
            println!("-----  --------------");
            let file_ids: Vec<i64> = cycles.iter().flat_map(|c| c.iter()).copied().collect();
            let paths = deps_index.get_file_paths(&file_ids)?;

            for (idx, cycle) in cycles.iter().enumerate() {
                let cycle_str = cycle.iter()
                    .filter_map(|id| paths.get(id).map(|p| p.as_str()))
                    .collect::<Vec<_>>()
                    .join(" → ");
                println!("{:<5}  {}", idx + 1, cycle_str);
            }
            eprintln!("\nFound {} cycles", cycles.len());
        }
        _ => {
            anyhow::bail!("Unknown format '{}'. Supported: json, tree, table", format);
        }
    }

    Ok(())
}

/// Handle --hotspots flag (most-imported files)
fn handle_deps_hotspots(
    deps_index: &DependencyIndex,
    format: &str,
    pretty_json: bool,
    limit: Option<usize>,
) -> Result<()> {
    let hotspots = deps_index.find_hotspots(limit)?;

    if hotspots.is_empty() {
        println!("No hotspots found.");
        return Ok(());
    }

    let file_ids: Vec<i64> = hotspots.iter().map(|(id, _)| *id).collect();
    let paths = deps_index.get_file_paths(&file_ids)?;

    match format {
        "json" => {
            let output: Vec<_> = hotspots.iter()
                .filter_map(|(id, count)| {
                    paths.get(id).map(|path| serde_json::json!({
                        "file_id": id,
                        "path": path,
                        "import_count": count,
                    }))
                })
                .collect();

            let json_str = if pretty_json {
                serde_json::to_string_pretty(&output)?
            } else {
                serde_json::to_string(&output)?
            };
            println!("{}", json_str);
            eprintln!("Found {} hotspots", hotspots.len());
        }
        "tree" => {
            println!("Hotspots (Most-Imported Files):");
            for (idx, (id, count)) in hotspots.iter().enumerate() {
                if let Some(path) = paths.get(id) {
                    println!("  {}. {} ({} imports)", idx + 1, path, count);
                }
            }
            eprintln!("\nFound {} hotspots", hotspots.len());
        }
        "table" => {
            println!("Rank  Imports  File");
            println!("----  -------  ----");
            for (idx, (id, count)) in hotspots.iter().enumerate() {
                if let Some(path) = paths.get(id) {
                    println!("{:<4}  {:<7}  {}", idx + 1, count, path);
                }
            }
            eprintln!("\nFound {} hotspots", hotspots.len());
        }
        _ => {
            anyhow::bail!("Unknown format '{}'. Supported: json, tree, table", format);
        }
    }

    Ok(())
}

/// Handle --unused flag (orphaned files)
fn handle_deps_unused(
    deps_index: &DependencyIndex,
    format: &str,
    pretty_json: bool,
    limit: Option<usize>,
) -> Result<()> {
    let mut unused = deps_index.find_unused_files()?;

    // Apply limit if specified
    if let Some(lim) = limit {
        unused.truncate(lim);
    }

    if unused.is_empty() {
        println!("No unused files found (all files have incoming dependencies).");
        return Ok(());
    }

    let paths = deps_index.get_file_paths(&unused)?;

    match format {
        "json" => {
            let output: Vec<_> = unused.iter()
                .filter_map(|id| {
                    paths.get(id).map(|path| serde_json::json!({
                        "file_id": id,
                        "path": path,
                    }))
                })
                .collect();

            let json_str = if pretty_json {
                serde_json::to_string_pretty(&output)?
            } else {
                serde_json::to_string(&output)?
            };
            println!("{}", json_str);
            eprintln!("Found {} unused files", unused.len());
        }
        "tree" => {
            println!("Unused Files (No Incoming Dependencies):");
            for (idx, id) in unused.iter().enumerate() {
                if let Some(path) = paths.get(id) {
                    println!("  {}. {}", idx + 1, path);
                }
            }
            eprintln!("\nFound {} unused files", unused.len());
        }
        "table" => {
            println!("File ID  Path");
            println!("-------  ----");
            for id in &unused {
                if let Some(path) = paths.get(id) {
                    println!("{:<7}  {}", id, path);
                }
            }
            eprintln!("\nFound {} unused files", unused.len());
        }
        _ => {
            anyhow::bail!("Unknown format '{}'. Supported: json, tree, table", format);
        }
    }

    Ok(())
}

/// Handle --islands flag (disconnected components)
fn handle_deps_islands(
    _deps_index: &DependencyIndex,
    _format: &str,
    _pretty_json: bool,
) -> Result<()> {
    // TODO: Implement connected components analysis using Union-Find or DFS
    anyhow::bail!("--islands is not yet implemented. Coming soon!")
}

/// Handle --analyze flag (full report)
fn handle_deps_analyze(
    deps_index: &DependencyIndex,
    format: &str,
    pretty_json: bool,
    limit: Option<usize>,
) -> Result<()> {
    println!("Running comprehensive dependency analysis...\n");

    // Run all analyses
    println!("1. Circular Dependencies:");
    handle_deps_circular(deps_index, format, pretty_json)?;

    println!("\n2. Hotspots (Most-Imported Files):");
    handle_deps_hotspots(deps_index, format, pretty_json, limit)?;

    println!("\n3. Unused Files:");
    handle_deps_unused(deps_index, format, pretty_json, limit)?;

    println!("\nAnalysis complete!");
    Ok(())
}
```

---

### Testing Plan

#### Manual Testing Steps
1. **Basic operations:**
   ```bash
   rfx deps src/main.rs
   rfx deps src/main.rs --reverse
   rfx deps src/main.rs --depth 2
   rfx deps src/main.rs --format tree
   rfx deps src/main.rs --format table
   ```

2. **Graph-wide operations:**
   ```bash
   rfx deps --circular
   rfx deps --hotspots --limit 10
   rfx deps --unused
   rfx deps --analyze
   ```

3. **Filters:**
   ```bash
   rfx deps src/main.rs --only-internal
   rfx deps src/main.rs --only-external
   rfx deps src/main.rs --reverse --format tree
   ```

4. **Edge cases:**
   ```bash
   rfx deps nonexistent.rs  # Should error gracefully
   rfx deps src/main.rs --depth 0  # Should show self only
   rfx deps --circular --format json --pretty
   ```

#### Integration Test Outline (`tests/deps_test.rs`)
```rust
#[test]
fn test_deps_direct() {
    // Create test project with known dependencies
    // Index it
    // Run rfx deps and verify output
}

#[test]
fn test_deps_reverse() {
    // Verify reverse lookups return correct dependents
}

#[test]
fn test_deps_transitive() {
    // Verify transitive traversal with depth=3
}

#[test]
fn test_deps_circular() {
    // Create project with circular deps
    // Verify detection works
}

#[test]
fn test_deps_hotspots() {
    // Verify hotspot ranking
}

#[test]
fn test_deps_unused() {
    // Create orphaned file
    // Verify it's detected
}
```

---

### Success Criteria

**The implementation is complete when:**
1. ✅ `rfx deps <file>` shows direct dependencies in JSON format
2. ✅ `rfx deps <file> --reverse` shows dependents
3. ✅ `rfx deps <file> --depth N` traverses to depth N
4. ✅ `rfx deps <file> --format tree` displays tree visualization
5. ✅ `rfx deps <file> --format table` displays tabular output
6. ✅ `rfx deps --circular` detects and displays cycles
7. ✅ `rfx deps --hotspots` ranks most-imported files
8. ✅ `rfx deps --unused` finds orphaned files
9. ✅ `rfx deps --analyze` runs comprehensive report
10. ✅ All filters work: `--only-internal`, `--only-external`, `--only-stdlib`
11. ✅ Error handling is graceful (file not found, no index, etc.)
12. ✅ Integration tests pass
13. ✅ Documentation updated in CLAUDE.md

---

### Estimated Time to Completion

**Phase 1:** 2-3 hours (basic command + JSON output)
**Phase 2:** 3-4 hours (formatters: tree, table, dot)
**Phase 3:** 2-3 hours (graph-wide operations)
**Phase 4:** 2-3 hours (testing + docs)

**Total:** 9-13 hours (~1-2 days of focused work)

---

### Open Questions

1. **DOT format support:** Should we implement Graphviz DOT format in Phase 2, or defer to Phase 6?
   - **Recommendation:** Defer to Phase 6 (low priority, requires additional dependencies)

2. **--islands implementation:** Connected components analysis requires Union-Find or multi-source BFS
   - **Recommendation:** Implement in Phase 3 or defer to Phase 6

3. **Caching strategy:** Should we cache graph traversal results for repeated queries?
   - **Recommendation:** Not needed for MVP - queries are fast enough (<50ms)

4. **MCP integration:** Should `rfx deps` be exposed via MCP server?
   - **Recommendation:** Yes, add in Phase 3 after core implementation is stable

---

### Next Steps

**To start implementation:**
1. Create a new git branch: `git checkout -b feature/rfx-deps-command`
2. Begin with Phase 1: Add `Deps` command to CLI
3. Test incrementally after each phase
4. Document as you go (update CLAUDE.md examples)
5. Open PR when Phases 1-3 are complete and tested
