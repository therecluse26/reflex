# Claude Code Benchmark Test Prompts

This file contains 45 curated test prompts for evaluating `rfx` vs built-in Claude Code tools (Grep, Glob, Read).

## Test Metadata Format

Each test includes:
- **Prompt**: Exact text to send to Claude Code
- **Category**: Type of search task
- **Complexity**: Simple/Medium/Complex
- **Expected Approach (Built-in)**: Tool calls needed with Grep/Glob/Read
- **Expected Approach (RFX)**: Command(s) needed with rfx
- **Success Criteria**: What constitutes accurate results

---

## Category 1: Simple Text Search (Full-text)

### Test 1.1: Find Function Occurrences
**Prompt:** "Find all occurrences of 'extract_symbols' in the codebase"

**Complexity:** Simple

**Expected Approach (Built-in):**
- Grep: `pattern: "extract_symbols"`, `output_mode: "content"`
- Result: ~10-15 matches across multiple files

**Expected Approach (RFX):**
```bash
rfx query "extract_symbols" --json
```

**Success Criteria:**
- Finds both definition AND all usage sites
- Includes line numbers and context
- No false negatives

---

### Test 1.2: Find TODO Comments
**Prompt:** "Search for all TODO comments in the codebase"

**Complexity:** Simple

**Expected Approach (Built-in):**
- Grep: `pattern: "TODO"`, `-i: true`, `output_mode: "content"`

**Expected Approach (RFX):**
```bash
rfx query "TODO" --json
```

**Success Criteria:**
- Finds all TODO/Todo/todo variants
- Includes context showing what the TODO is about

---

### Test 1.3: Find Method Calls
**Prompt:** "Find all uses of 'unwrap()' in Rust code"

**Complexity:** Simple

**Expected Approach (Built-in):**
- Grep: `pattern: "unwrap()"`, `type: "rust"`, `output_mode: "content"`

**Expected Approach (RFX):**
```bash
rfx query "unwrap()" --lang rust --json
```

**Success Criteria:**
- Filtered to Rust files only
- Finds all unwrap() calls (including .unwrap_or(), .unwrap_or_else())

---

### Test 1.4: Find Public Functions
**Prompt:** "Search for the pattern 'pub fn' to find public functions"

**Complexity:** Simple

**Expected Approach (Built-in):**
- Grep: `pattern: "pub fn"`, `type: "rust"`, `output_mode: "content"`

**Expected Approach (RFX):**
```bash
rfx query "pub fn" --lang rust --json
```

**Success Criteria:**
- Finds all public function declarations
- Multi-word pattern handled correctly

---

### Test 1.5: Find Type References
**Prompt:** "Find all occurrences of 'SymbolKind' enum usage"

**Complexity:** Simple

**Expected Approach (Built-in):**
- Grep: `pattern: "SymbolKind"`, `output_mode: "content"`

**Expected Approach (RFX):**
```bash
rfx query "SymbolKind" --json
```

**Success Criteria:**
- Finds definition + all usage sites
- Includes type annotations, match arms, etc.

---

## Category 2: Symbol-Aware Search (Definitions Only)

### Test 2.1: Find Function Definition
**Prompt:** "Find the definition of the function 'extract_symbols'"

**Complexity:** Medium

**Expected Approach (Built-in):**
- Grep: `pattern: "fn extract_symbols"` OR `pattern: "pub fn extract_symbols"`
- Manual filtering of results to find definition vs calls
- May require Read on multiple files

**Expected Approach (RFX):**
```bash
rfx query "extract_symbols" --symbols --json
```

**Success Criteria:**
- Returns ONLY the definition, not call sites
- Single result (or few results if overloaded)
- Precise line number

---

### Test 2.2: Find All Struct Definitions
**Prompt:** "Find all struct definitions in the codebase"

**Complexity:** Medium

**Expected Approach (Built-in):**
- Grep: `pattern: "struct"`, `type: "rust"`, `output_mode: "content"`
- Manual filtering to exclude struct usages
- False positives: "destructure", comments, etc.

**Expected Approach (RFX):**
```bash
rfx query "struct" --kind Struct --json
```

**Success Criteria:**
- Only struct definitions, not usages
- No false positives from word "struct" in comments
- Includes all structs (pub, crate, private)

---

### Test 2.3: Find Specific Struct
**Prompt:** "Locate the SearchResult struct definition"

**Complexity:** Simple

**Expected Approach (Built-in):**
- Grep: `pattern: "struct SearchResult"` or `pattern: "SearchResult"`
- Read file to confirm it's the struct definition

**Expected Approach (RFX):**
```bash
rfx query "SearchResult" --kind Struct --json
```

**Success Criteria:**
- Single result with exact location
- Includes struct body or span

---

### Test 2.4: Find Functions in Module
**Prompt:** "Find all function definitions in the parsers module"

**Complexity:** Medium

**Expected Approach (Built-in):**
- Glob: `src/parsers/*.rs`
- Grep on each file: `pattern: "fn"` or `pattern: "pub fn"`
- Filter to definitions only (hard without symbol awareness)

**Expected Approach (RFX):**
```bash
rfx query "fn" --symbols --glob "src/parsers/*.rs" --json
```

**Success Criteria:**
- All parser function definitions
- Scoped to parsers module only
- No function calls, only definitions

---

### Test 2.5: Find All Enums
**Prompt:** "Find all enum definitions"

**Complexity:** Simple

**Expected Approach (Built-in):**
- Grep: `pattern: "enum"`, `type: "rust"`, `output_mode: "content"`
- Manual filtering

**Expected Approach (RFX):**
```bash
rfx query "enum" --kind Enum --json
```

**Success Criteria:**
- Only enum definitions
- No false positives from word "enum" elsewhere

---

### Test 2.6: Find All Traits
**Prompt:** "Find all trait definitions"

**Complexity:** Simple

**Expected Approach (Built-in):**
- Grep: `pattern: "trait"`, `type: "rust"`
- Filter manually

**Expected Approach (RFX):**
```bash
rfx query "trait" --kind Trait --json
```

**Success Criteria:**
- Only trait definitions
- Excludes trait bounds and impl Trait usage

---

### Test 2.7: Find Specific Enum
**Prompt:** "Find where the 'Language' enum is defined"

**Complexity:** Simple

**Expected Approach (Built-in):**
- Grep: `pattern: "enum Language"`
- Read file to verify

**Expected Approach (RFX):**
```bash
rfx query "Language" --kind Enum --json
```

**Success Criteria:**
- Single result: src/models.rs
- Exact line number

---

## Category 3: Glob Pattern Filtering

### Test 3.1: Search in Test Files
**Prompt:** "Search for 'test' only in test files"

**Complexity:** Medium

**Expected Approach (Built-in):**
- Glob: `**/*test*.rs`
- Grep on matched files: `pattern: "test"`

**Expected Approach (RFX):**
```bash
rfx query "test" --glob "**/*test*.rs" --json
```

**Success Criteria:**
- Results only from test files
- No matches from main source code

---

### Test 3.2: Search in Specific Directory
**Prompt:** "Find 'IndexConfig' only in src/ directory"

**Complexity:** Simple

**Expected Approach (Built-in):**
- Glob: `src/**/*.rs`
- Grep: `pattern: "IndexConfig"`

**Expected Approach (RFX):**
```bash
rfx query "IndexConfig" --glob "src/**/*.rs" --json
```

**Success Criteria:**
- Results from src/ only
- Excludes tests/, examples/, etc.

---

### Test 3.3: Search in Parser Files
**Prompt:** "Search for 'parser' in all parser implementation files"

**Complexity:** Simple

**Expected Approach (Built-in):**
- Glob: `src/parsers/*.rs`
- Grep: `pattern: "parser"`

**Expected Approach (RFX):**
```bash
rfx query "parser" --glob "src/parsers/*.rs" --json
```

**Success Criteria:**
- Results from parsers module only
- Includes all parser files

---

### Test 3.4: Search in Examples
**Prompt:** "Find 'cache' in example files only"

**Complexity:** Simple

**Expected Approach (Built-in):**
- Glob: `examples/*.rs`
- Grep: `pattern: "cache"`

**Expected Approach (RFX):**
```bash
rfx query "cache" --glob "examples/*.rs" --json
```

**Success Criteria:**
- Results from examples/ only

---

### Test 3.5: Exclude Test Files
**Prompt:** "Search for 'trigram' excluding test files"

**Complexity:** Medium

**Expected Approach (Built-in):**
- Grep: `pattern: "trigram"`
- Manual filtering of results to remove test files

**Expected Approach (RFX):**
```bash
rfx query "trigram" --exclude "**/*test*.rs" --json
```

**Success Criteria:**
- No results from test files
- Includes main source code only

---

## Category 4: Language-Specific Filtering

### Test 4.1: Python Classes
**Prompt:** "Find all Python class definitions in test corpus"

**Complexity:** Medium

**Expected Approach (Built-in):**
- Glob: `**/*.py`
- Grep: `pattern: "class "`, `type: "py"`
- Manual filtering to definitions only

**Expected Approach (RFX):**
```bash
rfx query "class" --lang python --kind Class --glob "tests/corpus/**/*.py" --json
```

**Success Criteria:**
- Only Python class definitions
- No JavaScript/TypeScript classes

---

### Test 4.2: JavaScript Functions
**Prompt:** "Search for JavaScript functions in the corpus"

**Complexity:** Medium

**Expected Approach (Built-in):**
- Glob: `**/*.js`
- Grep: `pattern: "function"`, `type: "js"`

**Expected Approach (RFX):**
```bash
rfx query "function" --lang javascript --json
```

**Success Criteria:**
- Only JavaScript files
- Includes function declarations and expressions

---

### Test 4.3: TypeScript Interfaces
**Prompt:** "Find TypeScript interfaces"

**Complexity:** Medium

**Expected Approach (Built-in):**
- Glob: `**/*.ts`
- Grep: `pattern: "interface"`, `type: "ts"`
- Manual filtering

**Expected Approach (RFX):**
```bash
rfx query "interface" --kind Interface --lang typescript --json
```

**Success Criteria:**
- Only TypeScript interface definitions
- No Java interfaces

---

## Category 5: Paths-Only Mode

### Test 5.1: Files with TODOs
**Prompt:** "List all files that contain 'TODO' comments"

**Complexity:** Simple

**Expected Approach (Built-in):**
- Grep: `pattern: "TODO"`, `output_mode: "files_with_matches"`

**Expected Approach (RFX):**
```bash
rfx query "TODO" --paths --json
```

**Success Criteria:**
- Returns unique file paths only
- No duplicate paths
- No line numbers or content

---

### Test 5.2: Files Mentioning Trigram
**Prompt:** "Get file paths for all files mentioning 'trigram'"

**Complexity:** Simple

**Expected Approach (Built-in):**
- Grep: `pattern: "trigram"`, `output_mode: "files_with_matches"`

**Expected Approach (RFX):**
```bash
rfx query "trigram" --paths --json
```

**Success Criteria:**
- File paths only
- Deduplicated

---

### Test 5.3: Files with Public Functions
**Prompt:** "Find which files define public functions"

**Complexity:** Simple

**Expected Approach (Built-in):**
- Grep: `pattern: "pub fn"`, `output_mode: "files_with_matches"`

**Expected Approach (RFX):**
```bash
rfx query "pub fn" --paths --json
```

**Success Criteria:**
- List of file paths
- No content preview

---

## Category 6: Complex Multi-Step Workflows

### Test 6.1: Find and Trace Usage
**Prompt:** "Find the IndexConfig struct and all places where it's used"

**Complexity:** Complex

**Expected Approach (Built-in):**
1. Grep: `pattern: "struct IndexConfig"` (find definition)
2. Read the file
3. Grep: `pattern: "IndexConfig"` (find all usages)
4. Read multiple files for context
Total: 4+ tool calls

**Expected Approach (RFX):**
```bash
# Step 1: Find definition
rfx query "IndexConfig" --kind Struct --json

# Step 2: Find all usages
rfx query "IndexConfig" --json
```
Total: 2 commands

**Success Criteria:**
- Finds definition
- Finds all usage sites
- Provides context for each

---

### Test 6.2: Understand Feature Implementation
**Prompt:** "Understand how the trigram indexing works"

**Complexity:** Complex

**Expected Approach (Built-in):**
1. Glob: `trigram*.rs`
2. Read: `src/trigram.rs`
3. Grep: `pattern: "trigram"` (find usages)
4. Read related files
Total: 4+ tool calls

**Expected Approach (RFX):**
```bash
# Step 1: Find trigram-related code
rfx query "trigram" --glob "src/**/*.rs" --json

# Step 2: Find main functions
rfx query "extract_trigrams" --symbols --json
```
Total: 2 commands

**Success Criteria:**
- Finds core implementation
- Finds usage patterns
- Provides architectural overview

---

### Test 6.3: Explore Module Structure
**Prompt:** "Find all parser implementations and their structure"

**Complexity:** Complex

**Expected Approach (Built-in):**
1. Glob: `src/parsers/*.rs`
2. Read each parser file (10+ reads)
3. Extract function names manually
Total: 11+ tool calls

**Expected Approach (RFX):**
```bash
rfx query "pub fn" --glob "src/parsers/*.rs" --symbols --json
```
Total: 1 command

**Success Criteria:**
- Lists all parser modules
- Shows parser function signatures
- Reveals module structure

---

### Test 6.4: Trace Query Execution
**Prompt:** "Trace how query execution works from CLI to results"

**Complexity:** Complex

**Expected Approach (Built-in):**
1. Read: `src/main.rs`
2. Grep: `pattern: "query"` (command handling)
3. Read: `src/query.rs`
4. Grep: related functions
5. Read multiple files
Total: 5+ tool calls

**Expected Approach (RFX):**
```bash
# Step 1: Find query-related code
rfx query "query" --glob "src/**/*.rs" --json

# Step 2: Find query execution function
rfx query "execute_query" --symbols --json
```
Total: 2 commands

**Success Criteria:**
- Maps CLI → QueryEngine flow
- Identifies key functions
- Shows call graph

---

### Test 6.5: Find Error Types
**Prompt:** "Find all error types and where they're used"

**Complexity:** Complex

**Expected Approach (Built-in):**
1. Grep: `pattern: "enum.*Error"`
2. Read files with error definitions
3. Grep: `pattern: "Result<"` (usage)
4. Aggregate results
Total: 4+ tool calls

**Expected Approach (RFX):**
```bash
# Step 1: Find error enum definitions
rfx query "Error" --kind Enum --json

# Step 2: Find Result usage
rfx query "Result<" --json
```
Total: 2 commands

**Success Criteria:**
- Finds all error types
- Shows error usage patterns
- Maps error propagation

---

## Category 7: Regex Pattern Matching

### Test 7.1: Functions Starting with test_
**Prompt:** "Find all functions starting with 'test_'"

**Complexity:** Medium

**Expected Approach (Built-in):**
- Grep: `pattern: "test_.*"` (may need -n flag for context)

**Expected Approach (RFX):**
```bash
rfx query "test_.*" --json
```

**Success Criteria:**
- Finds all test_ functions
- Regex pattern matched correctly

---

### Test 7.2: Config and Cache Variables
**Prompt:** "Search for variable names matching 'config.*cache'"

**Complexity:** Medium

**Expected Approach (Built-in):**
- Grep: `pattern: "config.*cache"`

**Expected Approach (RFX):**
```bash
rfx query "config.*cache" --json
```

**Success Criteria:**
- Multi-segment regex works
- Finds patterns like "config_cache", "config.cache"

---

### Test 7.3: Serde Imports
**Prompt:** "Find all imports from 'serde' crate"

**Complexity:** Simple

**Expected Approach (Built-in):**
- Grep: `pattern: "use serde"`

**Expected Approach (RFX):**
```bash
rfx query "use serde" --json
```

**Success Criteria:**
- All serde imports found
- Includes use serde::, use serde::{...}

---

## Category 8: Performance & Scale Testing

### Test 8.1: Large Result Set - Functions
**Prompt:** "Find all occurrences of 'fn' in the entire codebase"

**Complexity:** Simple (but large results)

**Expected Approach (Built-in):**
- Grep: `pattern: "fn"`, `type: "rust"`
- May hit result limits

**Expected Approach (RFX):**
```bash
rfx query "fn" --json
```

**Success Criteria:**
- Returns 400+ results
- Handles large result set efficiently
- Doesn't timeout or truncate

---

### Test 8.2: Large Result Set - Structs
**Prompt:** "Search for 'struct' across all Rust files"

**Complexity:** Simple (but large results)

**Expected Approach (Built-in):**
- Grep: `pattern: "struct"`, `type: "rust"`

**Expected Approach (RFX):**
```bash
rfx query "struct" --lang rust --json
```

**Success Criteria:**
- Returns 500+ results
- Performance acceptable
- Results well-formatted

---

### Test 8.3: Common Type Usage
**Prompt:** "Find all 'Result' type usages"

**Complexity:** Simple (but very large results)

**Expected Approach (Built-in):**
- Grep: `pattern: "Result"`

**Expected Approach (RFX):**
```bash
rfx query "Result" --json
```

**Success Criteria:**
- Finds all Result<T, E> usages
- Handles potentially 1000+ results

---

## Category 9: Edge Cases & Precision

### Test 9.1: Exact Enum Location
**Prompt:** "Find the exact line where 'IndexStatus' enum is defined"

**Complexity:** Simple

**Expected Approach (Built-in):**
- Grep: `pattern: "enum IndexStatus"`
- Read file to verify exact line

**Expected Approach (RFX):**
```bash
rfx query "IndexStatus" --kind Enum --json
```

**Success Criteria:**
- Single result
- Exact line number provided
- Span information accurate

---

### Test 9.2: Keyword in Code
**Prompt:** "Search for 'match' keyword (not the word in comments)"

**Complexity:** Medium

**Expected Approach (Built-in):**
- Grep: `pattern: "match"` (will include false positives)
- Manual filtering required

**Expected Approach (RFX):**
```bash
rfx query "match" --json
```

**Success Criteria:**
- Finds match expressions
- May include some comment matches (acceptable)

---

### Test 9.3: Short Identifier
**Prompt:** "Find variable 'i' declarations (short identifier)"

**Complexity:** Medium

**Expected Approach (Built-in):**
- Grep: `pattern: "\\bi\\b"` (word boundary)

**Expected Approach (RFX):**
```bash
rfx query "\\bi\\b" --json
```

**Success Criteria:**
- Finds standalone 'i', not as part of other words
- Word boundary handling correct

---

### Test 9.4: Special Characters
**Prompt:** "Search for 'pub(crate)' visibility modifiers"

**Complexity:** Simple

**Expected Approach (Built-in):**
- Grep: `pattern: "pub\\(crate\\)"`

**Expected Approach (RFX):**
```bash
rfx query "pub(crate)" --json
```

**Success Criteria:**
- Finds all pub(crate) declarations
- Parentheses handled correctly

---

## Category 10: Context Quality

### Test 10.1: Impl Blocks with Context
**Prompt:** "Find 'impl' blocks and show surrounding context"

**Complexity:** Medium

**Expected Approach (Built-in):**
- Grep: `pattern: "impl"`, `-C: 3`, `output_mode: "content"`

**Expected Approach (RFX):**
```bash
rfx query "impl" --json
```

**Success Criteria:**
- Shows context_before and context_after
- Context reveals what's being implemented
- Preview is readable

---

### Test 10.2: Function Call Context
**Prompt:** "Show me where 'parse_symbols' is called with context"

**Complexity:** Simple

**Expected Approach (Built-in):**
- Grep: `pattern: "parse_symbols"`, `-C: 3`

**Expected Approach (RFX):**
```bash
rfx query "parse_symbols" --json
```

**Success Criteria:**
- Context shows function arguments
- Shows calling context (which function/module)
- Multiple call sites found

---

## Category 11: Attribute/Annotation Discovery

### Test 11.1: Rust Attribute Macros
**Prompt:** "Find all Rust attribute proc macros"

**Complexity:** Medium

**Expected Approach (Built-in):**
- Grep: `pattern: "#\\[proc_macro_attribute\\]"`
- Manual filtering

**Expected Approach (RFX):**
```bash
rfx query "proc_macro" --kind Attribute --lang rust --json
```

**Success Criteria:**
- Finds proc_macro_attribute definitions
- No false positives

---

### Test 11.2: Derive Macros
**Prompt:** "Find derive macros in the codebase"

**Complexity:** Simple

**Expected Approach (Built-in):**
- Grep: `pattern: "#\\[derive"`

**Expected Approach (RFX):**
```bash
rfx query "#[derive" --json
```

**Success Criteria:**
- All derive macro usages
- Shows which traits are derived

---

## Category 12: Real-World AI Agent Scenarios

### Test 12.1: Understand Parser Structure for New Language
**Prompt:** "I need to add a new language parser. Show me existing parser structure."

**Complexity:** Complex

**Expected Approach (Built-in):**
1. Glob: `src/parsers/*.rs`
2. Read: `src/parsers/rust.rs` (example)
3. Read: `src/parsers/mod.rs` (registry)
4. Grep: parser trait/interface
Total: 4+ tool calls

**Expected Approach (RFX):**
```bash
# Step 1: Find parser functions
rfx query "pub fn parse" --glob "src/parsers/*.rs" --symbols --json

# Step 2: Find parser factory
rfx query "ParserFactory" --symbols --json
```
Total: 2 commands

**Success Criteria:**
- Reveals parser interface
- Shows example implementations
- Identifies registration mechanism

---

### Test 12.2: MCP Server Implementation
**Prompt:** "Find where MCP server is implemented and how it handles queries"

**Complexity:** Complex

**Expected Approach (Built-in):**
1. Glob: `mcp*.rs`
2. Read: `src/mcp.rs`
3. Grep: query handling
4. Read related files
Total: 4+ tool calls

**Expected Approach (RFX):**
```bash
# Step 1: Find MCP code
rfx query "mcp" --glob "src/**/*.rs" --json

# Step 2: Find query handler
rfx query "handle_query" --symbols --json
```
Total: 2 commands

**Success Criteria:**
- Finds MCP implementation
- Shows query handling flow
- Identifies key functions

---

### Test 12.3: Test Helper Discovery
**Prompt:** "Show me all test helper functions available"

**Complexity:** Medium

**Expected Approach (Built-in):**
1. Read: `tests/test_helpers.rs`
2. Grep: `pattern: "pub fn"` in tests

**Expected Approach (RFX):**
```bash
rfx query "pub fn" --glob "tests/**/*.rs" --symbols --json
```
Total: 1 command

**Success Criteria:**
- Lists all test helper functions
- Shows function signatures
- Provides usage context

---

## Summary Statistics

**Total Tests:** 45

**By Category:**
1. Simple Text Search: 5 tests
2. Symbol-Aware Search: 7 tests
3. Glob Pattern Filtering: 5 tests
4. Language-Specific Filtering: 3 tests
5. Paths-Only Mode: 3 tests
6. Complex Multi-Step Workflows: 5 tests
7. Regex Pattern Matching: 3 tests
8. Performance & Scale: 3 tests
9. Edge Cases & Precision: 4 tests
10. Context Quality: 2 tests
11. Attribute/Annotation Discovery: 2 tests
12. Real-World AI Agent Scenarios: 3 tests

**By Complexity:**
- Simple: 23 tests (51%)
- Medium: 14 tests (31%)
- Complex: 8 tests (18%)

**Expected Tool Call Reduction with RFX:**
- Simple queries: ~20-40% reduction (1-2 tools → 1 tool)
- Medium queries: ~50-60% reduction (3-4 tools → 1-2 tools)
- Complex queries: ~70-80% reduction (5-11 tools → 1-2 tools)
