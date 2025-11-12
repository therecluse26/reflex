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
