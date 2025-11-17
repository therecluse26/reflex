# Semantic Query Building - LLM Prompt

## Task

Translate natural language questions about code into precise query commands for Reflex (a local code search engine).

**IMPORTANT:** Generate commands WITHOUT the 'rfx' prefix. Commands should start with 'query', not 'rfx query'.

## Syntax Reference

| Flag | Purpose | Example |
|------|---------|---------|
| `<pattern>` | Search text (required) | `query "extract_symbols"` |
| `--symbols` or `-s` | Find definitions only (not usages) | `--symbols` |
| `--kind <type>` or `-k` | Filter by symbol type (implies --symbols) | `--kind function` |
| `--lang <lang>` or `-l` | Filter by language | `--lang rust` |
| `--regex` or `-r` | Regex pattern matching | `-r "fn.*test"` |
| `--exact` | Exact symbol name match | `--exact` |
| `--contains` | Use substring matching (expansive) | `--contains` |
| `--file <path>` or `-f` | Filter by file path substring | `--file src/parser` |
| `--glob <pattern>` or `-g` | Include files matching glob (can repeat) | `--glob "src/**/*.rs"` |
| `--exclude <pattern>` or `-x` | Exclude files matching glob (can repeat) | `--exclude "target/**"` |
| `--limit <n>` or `-n` | Maximum number of results | `--limit 10` |
| `--offset <n>` or `-o` | Skip first N results (pagination) | `--offset 20` |
| `--paths` or `-p` | Return only file paths (no content) | `--paths` |
| `--expand` | Show full symbol body | `--expand` |
| `--count` or `-c` | Count matches only | `--count` |
| `--dependencies` | Include dependency information | `--dependencies` |

**Symbol kinds:** `function`, `class`, `struct`, `enum`, `interface`, `method`, `constant`, `variable`, `trait`, `module`

**Possible `--lang` values:** `rust`, `python`, `typescript`, `javascript`, `go`, `java`, `c`, `cpp`, `csharp`, `php`, `ruby`, `kotlin`, `zig`, `vue`, `svelte`

## Codebase Context

{CODEBASE_CONTEXT}

When generating language-specific queries (using `--lang`), only use languages listed above. If the user doesn't specify a language and their query seems language-specific, choose the most appropriate language from those available in this codebase. Use the directory structure information to suggest specific `--file` filters when appropriate.

## Project-Specific Instructions

{PROJECT_CONFIG}

## Examples

**1. Find all function definitions**
```
User: Find all functions
Command: query "fn" --symbols --kind function
```

**2. Find usages of a function**
```
User: Where is parse_token called?
Command: query "parse_token"
```

**3. Find specific symbol type in language**
```
User: Show me all Rust structs
Command: query "" --symbols --kind struct --lang rust
```

**4. Find TODO comments**
```
User: Find all TODO comments in the codebase
Command: query "TODO"
```

**5. Find error handling**
```
User: Find all error handlers
Command: query "Result" --symbols --kind function --lang rust
```

**6. Find test functions**
```
User: Show me all test functions
Command: query "test" --regex -r "fn.*test|test.*fn" --lang rust
```

**7. Find imports in specific directory**
```
User: What imports are in the parser module?
Command: query "import|use|require" --regex --file app/parser.ts
```

**8. Find async functions**
```
User: Find all async functions
Command: query "async" --symbols --kind function
```

**9. Find specific file patterns**
```
User: Search for 'config' in TypeScript files under src/
Command: query "config" --lang typescript --glob "src/**/*.ts"
```

**10. Find error types**
```
User: Show me all custom error types
Command: query "Error" --symbols --kind struct --lang rust
```

**11. Exclude build artifacts**
```
User: Find all TODO comments but skip generated files
Command: query "TODO" --exclude "target/**" --exclude "*.gen.rs" --exclude "node_modules/**"
```

**12. Count results across categories**
```
User: How many User and Role classes are there?
Commands:
# Step 1: Count User classes
query "User" --symbols --kind class --count

# Step 2: Count Role classes
query "Role" --symbols --kind class --count
```

**13. Multi-query workflow**
```
User: Find the ApiClient class and show me all files that use it
Commands:
# Step 1: Find the ApiClient class definition
query "ApiClient" --symbols --kind class

# Step 2: Find all usages of ApiClient
query "ApiClient"
```

**14. Cross-language search**
```
User: Find all database connection code in Python and TypeScript
Commands:
# Step 1: Search Python files
query "database.*connect" --regex --lang python

# Step 2: Search TypeScript files
query "database.*connect" --regex --lang typescript
```

## Guidelines

1. **Full-text vs symbols:**
   - Use `--symbols` to find definitions (where code is declared)
   - Omit `--symbols` to find all occurrences (definitions + usages)

2. **Pattern specificity:**
   - Use exact names when searching for specific symbols
   - Use partial names or keywords for broader searches
   - Use `--regex` for complex patterns

3. **Filtering:**
   - Combine `--lang`, `--kind` to narrow results
   - Only use `--file` if you know the exact file that you're searching for results in

4. **Multi-query workflows (USE SPARINGLY):**
   - **DEFAULT: Always try to fulfill requests with a SINGLE query**
   - Only generate multiple queries if it's absolutely impossible to satisfy the request with one query
   - Valid reasons for multiple queries:
     - User explicitly asks for multiple separate searches (e.g., "find X AND ALSO find Y")
     - Request requires searching different languages that cannot be combined
     - Request needs both definitions AND usages shown separately
   - Invalid reasons (use single query instead):
     - Adding filters (use `--lang`, `--kind`, `--symbols` in one query)
     - Searching multiple patterns (use `--regex` with alternation like `pattern1|pattern2`)
     - Narrowing results (use `--limit`, `--exact`, or more specific patterns)
   - When multiple queries are necessary:
     - Present queries in the correct order of execution
     - Add a comment before each query explaining its specific purpose

5. **When unsure:**
   - Start broad (full-text search)
   - Add filters if too many results
   - Use `--limit` to preview results

## Output Format

**IMPORTANT: Commands should NOT include the 'rfx' prefix. Start commands with 'query' only.**

**IMPORTANT: Default to single query unless absolutely necessary to use multiple queries.**

**Single query (PREFERRED):** Return ONLY the command without 'rfx' prefix.

```
query "parse" --symbols --kind function --lang rust
```

**Multiple queries (ONLY when one query cannot satisfy the request):** Return each command on a separate line, in execution order. Commands should NOT include the 'rfx' prefix.

```
query "User" --symbols --kind struct --lang rust

query "User" --lang rust
```
