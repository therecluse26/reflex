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
| **Background Symbol Indexer** | Daemonized process that pre-caches symbols for faster queries on large codebases |
| **Symbol Cache** | Persistent storage of parsed symbols (803-line caching system for instant symbol lookups) |
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

    # Check background symbol indexing status
    rfx index --status

    # Full-text search (default - finds all occurrences)
    rfx query "extract_symbols"
    → Finds function definition + all call sites (11 total)

    # Filter to symbol definitions only (uses runtime tree-sitter parsing)
    # --symbols finds DEFINITIONS (where symbols are declared), not usages
    # Includes all scopes: global, local, function parameters, etc.
    rfx query "extract_symbols" --symbols
    → Finds only the function definition (1 result)

    rfx query "counter" --symbols
    → Finds variable declarations (global `var counter` and local `int counter = 0`)
    → Does NOT find usage sites like `counter++` or `return counter`

    # Filter to specific symbol kind (find attribute/annotation definitions)
    rfx query "test" --kind Attribute --lang rust
    → Finds #[proc_macro_attribute] test attribute definitions in Rust
    rfx query "test" --kind Attribute --lang java
    → Finds @interface Test annotation definitions in Java
    rfx query "deprecated" --kind Attribute --lang csharp
    → Finds DeprecatedAttribute class definitions in C#

    # Full-text search with language filter
    rfx query "unwrap" --lang rust

    # Export results as JSON (for AI agents)
    rfx query "format!" --json

    # Glob pattern filtering (include specific files/directories)
    rfx query "TODO" --glob "src/**/*.rs"        # Search only Rust files in src/
    rfx query "extract" --glob "**/*_test.rs"    # Search only test files

    # Exclude pattern filtering (exclude files/directories)
    rfx query "config" --exclude "target/**"     # Exclude build artifacts
    rfx query "impl" --exclude "*.gen.rs"        # Exclude generated code

    # Combine glob and exclude patterns
    rfx query "fn main" --glob "src/**/*.rs" --exclude "src/generated/**"

    # Paths-only mode (return unique file paths, not content)
    rfx query "TODO" --paths                     # One path per line (plain text)
    rfx query "extract" --paths --json           # JSON array of paths: ["file1.rs", "file2.rs"]
    vim $(rfx query "TODO" --paths)              # Open all files with TODOs in vim

    # Pagination (windowed results)
    rfx query "extract" --limit 10 --offset 0    # First 10 results
    rfx query "extract" --limit 10 --offset 10   # Next 10 results
    rfx query "extract" --all                    # All results (no limit)

    # Watch for file changes and auto-reindex
    rfx watch                    # 15s debounce (default)
    rfx watch --debounce 20000   # 20s debounce
    rfx watch --quiet            # Suppress output

    # Serve a local HTTP API (optional)
    rfx serve --port 7878

    # AST pattern matching (structure-aware search)
    # ⚠️ WARNING: AST queries are SLOW (500ms-10s+) - use --symbols instead in 95% of cases!
    # ALWAYS use --glob to limit scope and improve performance

    # Find all Rust functions (scans all indexed .rs files)
    rfx query "(function_item) @fn" --ast --lang rust --glob "src/**/*.rs"

    # Find Python async function definitions
    rfx query "(function_definition) @fn" --ast --lang python --glob "**/*.py"

    # Find TypeScript class declarations in src/
    rfx query "(class_declaration) @class" --ast --lang typescript --glob "src/**/*.ts"

    # Find Go method declarations (limit to specific package)
    rfx query "(method_declaration) @method" --ast --lang go --glob "internal/**/*.go"

---

## AST Pattern Matching

⚠️ **PERFORMANCE WARNING**: AST queries are **SLOW** (500ms-10s+) and scan the **ENTIRE codebase**. **In 95% of cases, use `--symbols` instead** (10-100x faster).

Reflex supports **structure-aware code search** using Tree-sitter S-expression queries via the `--ast` flag. This allows you to match specific code structures rather than just text patterns.

### When to Use AST Queries (RARE)

Use AST queries **ONLY when**:
1. You need to match code structure, not just text (e.g., "all async functions with try/catch blocks")
2. `--symbols` search is insufficient (e.g., need to match specific AST node types)
3. You have a very specific structural pattern that cannot be expressed as text

**DO NOT use AST queries** for simple symbol searches - use `--symbols` instead.

### How It Works

⚠️ **CRITICAL**: AST queries bypass trigram optimization and scan ALL files for the specified language:

1. **Get all files**: Scan entire codebase for matching language extension
2. **Filter by glob** (REQUIRED for performance): Reduce file set using --glob patterns
3. **Parse files**: Use tree-sitter to parse all matching files
4. **Execute query**: Run S-expression pattern on AST nodes
5. **Return**: Matched code structures with context

**Performance impact**: 500ms-10s+ depending on codebase size. **ALWAYS use `--glob` to limit scope**.

### Supported Languages

**All tree-sitter languages** support AST queries automatically:
- Rust, Python, Go, Java, C, C++, C#
- PHP, Ruby, Kotlin, Zig
- TypeScript, JavaScript

**Not supported**: Vue, Svelte (use line-based parsing, not tree-sitter)

### Architecture: Centralized Grammar Loading

AST support is **automatic** for all tree-sitter languages through a centralized grammar loader in `src/parsers/mod.rs`:

```rust
impl ParserFactory {
    /// Single source of truth for tree-sitter grammars
    /// Used by both symbol parsers AND AST query matching
    pub fn get_language_grammar(language: Language) -> Result<tree_sitter::Language> {
        match language {
            Language::Rust => Ok(tree_sitter_rust::LANGUAGE.into()),
            Language::Python => Ok(tree_sitter_python::LANGUAGE.into()),
            // ... all other languages
        }
    }
}
```

**Result**: Adding a new language to Reflex automatically enables AST queries. No separate maintenance required.

### Examples

**IMPORTANT**: Always use `--glob` to limit scope for better performance.

**Rust: Find all functions in src/**
```bash
rfx query "(function_item) @fn" --ast --lang rust --glob "src/**/*.rs"
```

**Python: Find all class definitions in specific directory**
```bash
rfx query "(class_declaration) @class" --ast --lang python --glob "app/**/*.py"
```

**Go: Find method declarations in specific package**
```bash
rfx query "(method_declaration) @method" --ast --lang go --glob "internal/**/*.go"
```

**TypeScript: Find interface declarations in src/**
```bash
rfx query "(interface_declaration) @interface" --ast --lang typescript --glob "src/**/*.ts"
```

**C: Find struct definitions in headers**
```bash
rfx query "(struct_specifier) @struct" --ast --lang c --glob "include/**/*.h"
```

### S-Expression Syntax

Tree-sitter queries use S-expression patterns:

- `(node_type)` - Match a node type
- `(parent (child))` - Match nested structure
- `@name` - Capture the match (required)
- Field names: `name:`, `type:`, `body:`

**Example**: Match async functions with specific name
```
(function_item
  (async)
  name: (identifier) @name) @function
```

### Performance

⚠️ **AST queries are SLOW** (500ms-10s+) because they:
- Bypass trigram optimization (no text pre-filtering)
- Scan ALL files for the specified language
- Parse every matching file with tree-sitter

**Performance comparison:**
| Query Type | Time (small codebase) | Time (Linux kernel - 62K files) |
|------------|----------------------|----------------------------------|
| **Full-text search** | 2-5ms | 124ms |
| **--symbols search** | 3-10ms | 224ms (parses ~10 files) |
| **--ast query (no glob)** | 500ms-2s | 5-10s+ (parses ALL files) |
| **--ast query (with glob)** | 50-200ms | 500ms-2s (parses filtered files) |

**ALWAYS use `--glob`** to limit scope and reduce parse time.

### Use Cases (RARE - prefer --symbols in most cases)

Use AST queries only for structural matching that cannot be done with `--symbols`:

1. **Complex structural patterns**: Find async functions containing try/catch blocks
2. **Nested structures**: Find classes with specific method signatures
3. **AST node filtering**: Match specific tree-sitter node types not exposed via `--symbols`

**For simple searches, use `--symbols` instead:**
- ❌ AST: `rfx query "(function_item) @fn" --ast --lang rust --glob "src/**/*.rs"` (500ms)
- ✅ Symbols: `rfx query "my_function" --symbols --lang rust` (5ms)

---

## Supported Languages & Frameworks

Reflex currently supports symbol extraction for the following languages and frameworks:

### Fully Supported (Tree-sitter parsers implemented)

| Language/Framework | Extensions | Symbol Extraction | Notes |
|-------------------|------------|------------------|-------|
| **Rust** | `.rs` | Functions, structs, enums, traits, impls, modules, methods, constants, local variables (let bindings), type aliases, macros (macro_rules!), static variables, attribute proc macros (#[proc_macro_attribute]) | Complete Rust support |
| **Python** | `.py` | Functions, classes, methods, constants, local variables, global variables (non-uppercase), lambdas, decorators (@property, etc.) | Full Python support including async/await |
| **TypeScript** | `.ts`, `.tsx`, `.mts`, `.cts` | Functions, classes, interfaces, types, enums, methods, local variables (const, let, var) | Full TypeScript + JSX support |
| **JavaScript** | `.js`, `.jsx`, `.mjs`, `.cjs` | Functions, classes, constants, methods, local variables (const, let, var) | Includes React/JSX support via TSX grammar |
| **Go** | `.go` | Functions, structs, interfaces, methods, constants, variables (global + local var/`:=`), packages | Full Go support |
| **Java** | `.java` | Classes, interfaces, enums, methods, fields, local variables, constructors, annotation definitions (@interface) | Full Java support including generics and annotations |
| **C** | `.c`, `.h` | Functions, structs, enums, unions, typedefs, variables (global + local), macros | Complete C support |
| **C++** | `.cpp`, `.cc`, `.cxx`, `.hpp`, `.hxx`, `.C`, `.H` | Functions, classes, structs, namespaces, templates, methods, constructors, destructors, local variables, type aliases | Full C++ support including templates |
| **C#** | `.cs` | Classes, interfaces, structs, enums, records, delegates, methods, properties, events, indexers (this[]), local variables, namespaces, attribute classes (Attribute suffix or base class) | Full C# support (C# 1-13) |
| **PHP** | `.php` | Functions, classes, interfaces, traits, methods, properties, constants, local variables, namespaces, enums, attribute classes (#[Attribute]) | Full PHP support including PHP 8.0+ attributes |
| **Ruby** | `.rb`, `.rake`, `.gemspec` | Classes, modules, methods, singleton methods, constants, local variables, instance variables (@var), class variables (@@var), attr_accessor/reader/writer, blocks | Full Ruby support including Rails patterns |
| **Kotlin** | `.kt`, `.kts` | Classes, objects, interfaces, functions, properties, local variables (val/var), data classes, sealed classes, annotation classes | Full Kotlin support including Android development |
| **Zig** | `.zig` | Functions, structs, enums, constants, variables (global + local var/const), tests, error sets | Full Zig support |
| **~~Swift~~** | `.swift` | ~~Classes, structs, enums, protocols, functions, extensions, properties, actors~~ | **Temporarily disabled** - requires tree-sitter 0.23 (Reflex uses 0.24) |
| **Vue** | `.vue` | Functions, constants, local variables (const, let, var), methods from `<script>` blocks | Supports both Options API and Composition API |
| **Svelte** | `.svelte` | Functions, constants, local variables (const, let, var), reactive declarations (`$:`), module context | Full Svelte component support |

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
- **Attributes**: PHP 8.0+ attribute class definitions (classes decorated with `#[Attribute]`)

### Attribute/Annotation Support Details
Reflex supports finding attribute, annotation, and decorator definitions across multiple languages using the `--kind Attribute` filter:

- **Rust**: Attribute proc macros (functions with `#[proc_macro_attribute]`)
- **Java**: `@interface` annotation definitions (e.g., `@interface Test { ... }`)
- **Kotlin**: `annotation class` definitions (e.g., `annotation class Entity`)
- **PHP**: PHP 8.0+ attribute classes (classes with `#[Attribute]` decorator)
- **C#**: Attribute classes (classes ending with "Attribute" suffix or inheriting from System.Attribute)

**Example queries:**
```bash
# Find all test attribute proc macros in Rust
rfx query "test" --kind Attribute --lang rust

# Find all test annotations in Java
rfx query "test" --kind Attribute --lang java

# Find all composable annotations in Kotlin (Jetpack Compose)
rfx query "composable" --kind Attribute --lang kotlin

# Find all route attributes in PHP
rfx query "route" --kind Attribute --lang php

# Find all validation attributes in C#
rfx query "validation" --kind Attribute --lang csharp
```

**Note**: This finds attribute/annotation **definitions**, not their usage sites. For usage, use full-text search without `--kind`.

**Coverage**: Reflex supports **90%+ of all codebases** across web, mobile, systems, enterprise, and AI/ML development (18 languages: Rust, Python, TypeScript, JavaScript, Go, Java, C, C++, C#, PHP, Ruby, Kotlin, Zig, Vue, Svelte, plus experimental Swift support once tree-sitter compatibility is resolved).

**Note on Swift**: Swift support is temporarily disabled due to tree-sitter version incompatibility. The tree-sitter-swift grammar requires tree-sitter 0.23, while Reflex uses tree-sitter 0.24 for better performance and compatibility with other languages. Swift support will be restored when the grammar is updated to 0.24+.

**Note**: Full-text trigram search works for **all file types** regardless of parser support. Symbol filtering (`symbol:` queries) requires a language parser.

---

## Dependency/Import Extraction

Reflex includes **experimental support for dependency extraction** across multiple languages. This feature analyzes import statements to understand codebase structure, but operates under strict design constraints to ensure accuracy and performance.

### Static-Only Import Resolution

**IMPORTANT**: Reflex **intentionally** extracts **only static imports** (string literals) and filters out dynamic imports.

This is **not a limitation** - it's a deliberate design decision enforced by tree-sitter query patterns:

```rust
// Python: Only matches static import statements with dotted_name nodes
(import_statement
    name: (dotted_name) @import_path)

// TypeScript/JavaScript: Only matches static string literal nodes
(import_statement
    source: (string) @import_path)

// Ruby: Only matches static string or symbol nodes
(call
    method: (identifier) @method_name
    arguments: (argument_list
        [(string (string_content) @import_path)
         (simple_symbol) @import_path]))

// PHP: Only matches static string literal nodes
(require_expression
    (string) @require_path)

// C: Only matches static string_literal or system_lib_string nodes
(preproc_include
    path: (string_literal) @include_path)
```

**Why static-only?**
- **Deterministic results**: Same codebase → same dependency graph
- **Performance**: No runtime code evaluation or complex pattern matching required
- **Accuracy**: Avoids false positives from computed imports that may never execute
- **Maintainability**: Simple tree-sitter queries are easy to understand and test

### What Gets Filtered Out

Dynamic imports are automatically filtered because they use different AST node types:

**Python**
```python
# ✅ Captured (static imports)
import os
from json import loads

# ❌ Filtered (dynamic imports - use identifier/call_expression nodes, not dotted_name)
module = importlib.import_module("some_module")
pkg = __import__("package")
exec("import dynamic")
```

**TypeScript/JavaScript**
```typescript
// ✅ Captured (static imports)
import { Button } from './Button';
const fs = require('fs');

// ❌ Filtered (dynamic imports - use template_string or identifier nodes, not string)
import(moduleName);
import(`./templates/${template}`);
require(variable);
require(`${CONFIG_PATH}/settings.js`);
```

**Ruby**
```ruby
# ✅ Captured (static requires)
require 'json'
require_relative '../models/user'

# ❌ Filtered (dynamic requires - use identifier or expression nodes, not string/symbol)
require variable
require CONSTANT
require File.join('path', 'to', 'file')
require_relative File.dirname(__FILE__) + '/dynamic'
```

**PHP**
```php
// ✅ Captured (static use/require)
use App\Models\User;
require 'config.php';

// ❌ Filtered (dynamic requires - use identifier or binary_expression nodes, not string)
require $variable;
require CONSTANT . '/file.php';
require_once $path;
```

**C**
```c
// ✅ Captured (static includes)
#include <stdio.h>
#include "config.h"

// ❌ Filtered (macro-based includes - use identifier nodes, not string_literal)
#define HEADER_NAME "dynamic.h"
#include HEADER_NAME
#include STRINGIFY(runtime_header.h)
```

### Import Classification

Reflex classifies imports into three categories:

1. **Internal**: Project code (relative paths, quoted C includes, tsconfig aliases)
2. **External**: Third-party packages (npm modules, gems, composer packages)
3. **Stdlib**: Standard library (Node.js built-ins, Python stdlib, C stdlib, etc.)

### Monorepo Support

Reflex supports monorepo structures for languages with multi-project config files:

- **TypeScript/JavaScript**: Multiple `tsconfig.json` files (path aliases resolved per-project)
- **Go**: Multiple `go.mod` files (module names extracted from each file)
- **Java**: Multiple `pom.xml` or `build.gradle` files (group IDs extracted)
- **PHP**: Multiple `composer.json` files (PSR-4 autoloading per-project)
- **Python**: Multiple `pyproject.toml`, `setup.py`, or `setup.cfg` files (package names extracted)
- **Ruby**: Multiple `.gemspec` files (gem names extracted, hyphen/underscore variants handled)
- **Rust**: Multiple `Cargo.toml` files (crate names extracted)
- **C#**: Multiple `.csproj` files (assembly names extracted)
- **Kotlin**: Multiple `build.gradle.kts` or `pom.xml` files (group/artifact IDs extracted)

### Limitations

**Partial Accuracy** (by design):
- **C/C++**: Build system integration required for full accuracy (include paths from CMakeLists.txt, Makefiles)
- **Dynamic imports**: Not supported (filtered by tree-sitter queries)
- **Computed paths**: Template literals, string concatenation, variable-based imports are filtered
- **Runtime-only imports**: Conditional requires, lazy loading, plugin systems are filtered

**Not Implemented** (low priority, high complexity):
- **PHP classmap autoloading**: Deprecated in modern PHP, PSR-4 preferred
- **Python namespace packages**: Very rare pattern (`__init__.py`-less packages)
- **C/C++ complex includes**: Macro expansion, conditional compilation

### Use Case

Dependency extraction is designed for **codebase structure analysis**, not exhaustive accuracy:

- Understanding module boundaries
- Identifying internal vs external dependencies
- Analyzing import patterns and coupling
- Supporting limited call graph queries

**Not suitable for**:
- Build system replacement
- Package manager functionality
- Runtime dependency resolution
- Dynamic code analysis

### Dependency Analysis Commands

Reflex provides two commands for analyzing dependencies:

#### `rfx deps` - Single File Analysis

Show dependencies for a specific file:

```bash
# Show dependencies of a file (depth 1, default)
rfx deps src/main.rs

# Show what depends on this file (reverse lookup)
rfx deps src/config.rs --reverse

# Show transitive dependencies (depth 3)
rfx deps src/api.rs --depth 3

# Output as JSON
rfx deps src/main.rs --json

# Filter by dependency type
rfx deps src/main.rs --only-internal   # Show only internal dependencies
rfx deps src/main.rs --only-external   # Show only external dependencies
rfx deps src/main.rs --only-stdlib     # Show only standard library imports

# Different output formats
rfx deps src/main.rs --format tree     # ASCII tree (default)
rfx deps src/main.rs --format table    # Table format
rfx deps src/main.rs --format dot      # Graphviz DOT format
```

**Example output (tree format):**
```
src/main.rs
├── src/config.rs (internal)
│   └── src/env.rs (internal)
├── src/api.rs (internal)
│   ├── reqwest (external)
│   └── src/models/user.rs (internal)
└── std::collections (stdlib)
```

#### `rfx analyze` - Codebase-Wide Analysis

Analyze dependency patterns across the entire codebase:

```bash
# Summary report (default - shows all analysis types)
rfx analyze

# Find circular dependencies
rfx analyze --circular

# Find most-imported files (hotspots)
rfx analyze --hotspots
rfx analyze --hotspots --min-dependents 5  # Filter by minimum import count

# Find unused/orphaned files
rfx analyze --unused

# Find disconnected components (islands)
rfx analyze --islands
rfx analyze --islands --min-island-size 3

# Combine with filters
rfx analyze --circular --glob "src/**/*.rs"     # Only analyze Rust files
rfx analyze --hotspots --exclude "target/**"    # Exclude build artifacts

# Output as JSON
rfx analyze --circular --json

# Just show count (no detailed results)
rfx analyze --hotspots --count
```

**Example output (summary report):**
```
Dependency Analysis Summary
===========================

Total Files: 1,234
Total Dependencies: 5,678
Average Dependencies per File: 4.6

Circular Dependencies: 2 cycles found
  → Use 'rfx analyze --circular' for details

Hotspots: 156 files with 3+ dependents
  Top 5:
    - src/config.rs (47 dependents) ⚠️
    - src/utils.rs (33 dependents)
    - src/types.rs (28 dependents)
  → Use 'rfx analyze --hotspots' for full list

Unused Files: 5 found
  → Use 'rfx analyze --unused' for details

Islands: 1 disconnected component (3 files)
  → Use 'rfx analyze --islands' for details
```

**Use cases:**
- **Refactoring**: Understand impact before changes (`rfx deps --reverse`)
- **Architecture review**: Find circular dependencies (`rfx analyze --circular`)
- **Code cleanup**: Identify unused files (`rfx analyze --unused`)
- **Hotspot detection**: Find over-imported files (`rfx analyze --hotspots`)
- **Monorepo analysis**: Understand component boundaries (`rfx analyze --islands`)

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

Reflex follows **semantic versioning** (SemVer) with a simple manual release workflow powered by cargo-dist.

### Semantic Versioning

Version format: `MAJOR.MINOR.PATCH` (e.g., `0.2.7`)

- **MAJOR**: Breaking changes (incompatible API changes)
- **MINOR**: New features (backward-compatible functionality)
- **PATCH**: Bug fixes (backward-compatible bug fixes)

**Examples:**
- `0.2.6 → 0.2.7`: Bug fix (PATCH bump)
- `0.2.7 → 0.3.0`: New feature like `--timeout` flag (MINOR bump)
- `0.3.0 → 1.0.0`: Breaking change or stable release (MAJOR bump)

### Creating a Release

**Simple 3-step process:**

```bash
# 1. Update version in Cargo.toml
vim Cargo.toml  # Change version = "0.2.6" to "0.2.7"

# 2. Commit and push
git add Cargo.toml
git commit -m "chore: bump version to 0.2.7"
git push origin main

# 3. Create and push tag
git tag v0.2.7
git push origin v0.2.7
```

**That's it!** When you push the tag, GitHub Actions automatically:
- Builds binaries for all platforms (Linux, macOS, Windows, ARM, x86_64)
- Extracts raw executables from cargo-dist archives
- Creates a GitHub Release with:
  - Raw binaries (e.g., `rfx-x86_64-unknown-linux-gnu`, `rfx-x86_64-pc-windows-msvc.exe`)
  - Shell and PowerShell installer scripts
  - Auto-generated release notes

### What Gets Released

The GitHub Release will contain:

**Binaries (raw executables, no archives):**
- `rfx-aarch64-apple-darwin` - macOS ARM (Apple Silicon)
- `rfx-aarch64-unknown-linux-gnu` - Linux ARM64
- `rfx-x86_64-apple-darwin` - macOS Intel
- `rfx-x86_64-unknown-linux-gnu` - Linux x64 (glibc)
- `rfx-x86_64-unknown-linux-musl` - Linux x64 (static, no libc)
- `rfx-x86_64-pc-windows-msvc.exe` - Windows x64

**Installers:**
- `reflex-installer.sh` - Shell install script (`curl | sh`)
- `reflex-installer.ps1` - PowerShell install script

### Workflow Configuration

Releases are configured in:
- **`dist-workspace.toml`** - cargo-dist configuration (platforms, installers)
- **`.github/workflows/release.yml`** - GitHub Actions workflow (builds binaries, extracts archives)

**Key settings:**
```toml
# dist-workspace.toml
[dist]
targets = ["aarch64-apple-darwin", "aarch64-unknown-linux-gnu",
           "x86_64-apple-darwin", "x86_64-unknown-linux-gnu",
           "x86_64-unknown-linux-musl", "x86_64-pc-windows-msvc"]
installers = ["shell", "powershell"]
auto-includes = false  # Don't bundle README/CHANGELOG in archives
allow-dirty = ["ci"]   # Allow custom workflow modifications
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

---
