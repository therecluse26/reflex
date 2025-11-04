# Reflex HTTP API Reference

This document describes the Reflex HTTP API for programmatic code search integration.

## Overview

Reflex provides a REST API for integrating code search into editors, CI/CD pipelines, AI coding assistants, and custom tools. The API mirrors the CLI functionality with structured JSON responses.

**Base URL:** `http://localhost:7878` (configurable via `--host` and `--port`)

**Transport:** HTTP/1.1 with JSON request/response bodies

**Authentication:** None (local-only API, bind to `127.0.0.1` by default)

**CORS:** Enabled for all origins (suitable for browser-based tools)

---

## Getting Started

### Start the Server

```bash
# Default configuration (localhost:7878)
rfx serve

# Custom port
rfx serve --port 8080

# Bind to all interfaces (use with caution)
rfx serve --host 0.0.0.0 --port 7878
```

The server will print available endpoints on startup:

```
Starting Reflex HTTP server...
  Address: http://127.0.0.1:7878

Endpoints:
  GET  /query?q=<pattern>&lang=<lang>&kind=<kind>&limit=<n>&symbols=true&regex=true&exact=true&expand=true&file=<pattern>&timeout=<secs>
  GET  /stats
  POST /index

Press Ctrl+C to stop.
```

### Quick Test

```bash
# Health check
curl http://localhost:7878/health
# → "Reflex is running"

# Simple query
curl 'http://localhost:7878/query?q=QueryEngine&limit=5' | jq '.'
```

---

## Endpoints

### GET /query

Search the codebase with full query capabilities.

**URL:** `/query`

**Method:** `GET`

**Query Parameters:**

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `q` | string | **Yes** | - | Search pattern (plain text, regex, or trigrams) |
| `symbols` | boolean | No | `false` | Search symbol definitions only (functions, classes, etc.) |
| `regex` | boolean | No | `false` | Treat pattern as regex |
| `exact` | boolean | No | `false` | Exact match (no substring matching) |
| `lang` | string | No | - | Filter by language (see [Supported Languages](#supported-languages)) |
| `kind` | string | No | - | Filter by symbol kind (implies `symbols=true`) |
| `file` | string | No | - | Filter by file path (substring match) |
| `limit` | integer | No | unlimited | Maximum number of results |
| `expand` | boolean | No | `false` | Show full symbol body (not just signature) |
| `timeout` | integer | No | `30` | Query timeout in seconds (0 = no timeout) |

**Response:** `application/json`

```json
{
  "status": "Fresh" | "Stale" | "Missing",
  "can_trust_results": boolean,
  "warning": string | null,
  "results": [
    {
      "file": "src/query.rs",
      "line": 145,
      "column": 8,
      "match": "pub struct QueryEngine {",
      "symbol": "QueryEngine",
      "kind": "Struct",
      "language": "Rust",
      "context_before": ["", "/// Main query execution engine"],
      "context_after": ["    cache: CacheManager,", "    config: QueryConfig,"]
    }
  ]
}
```

**Response Fields:**

- `status`: Index freshness indicator
  - `"Fresh"`: Index is up-to-date
  - `"Stale"`: Working tree has uncommitted changes since last index
  - `"Missing"`: No index found
- `can_trust_results`: Whether results can be trusted (false if index is stale/missing)
- `warning`: Human-readable warning message (null if no warning)
- `results`: Array of search results (see [SearchResult Schema](#searchresult-schema))

**HTTP Status Codes:**

- `200 OK`: Query successful
- `400 Bad Request`: Invalid query parameters
- `500 Internal Server Error`: Query execution failed

**Examples:**

```bash
# Full-text search
curl 'http://localhost:7878/query?q=extract_symbols&limit=10'

# Symbol-only search
curl 'http://localhost:7878/query?q=parse&symbols=true&kind=function'

# Regex search
curl 'http://localhost:7878/query?q=fn%20test_.*&regex=true'

# Language filter
curl 'http://localhost:7878/query?q=unwrap&lang=rust&limit=20'

# File path filter
curl 'http://localhost:7878/query?q=config&file=src/&limit=5'

# Multiple filters
curl 'http://localhost:7878/query?q=parse&lang=rust&kind=function&symbols=true&expand=true&limit=3'

# Custom timeout (10 seconds)
curl 'http://localhost:7878/query?q=complex_pattern&timeout=10'
```

**JavaScript Example:**

```javascript
async function searchCode(pattern, options = {}) {
  const params = new URLSearchParams({
    q: pattern,
    ...options
  });

  const response = await fetch(`http://localhost:7878/query?${params}`);
  const data = await response.json();

  if (!data.can_trust_results) {
    console.warn('Warning:', data.warning);
  }

  return data.results;
}

// Usage
const results = await searchCode('QueryEngine', {
  symbols: true,
  lang: 'rust',
  limit: 10
});
```

**Python Example:**

```python
import requests

def search_code(pattern, **kwargs):
    params = {'q': pattern, **kwargs}
    response = requests.get('http://localhost:7878/query', params=params)
    response.raise_for_status()

    data = response.json()
    if not data['can_trust_results']:
        print(f"Warning: {data['warning']}")

    return data['results']

# Usage
results = search_code('QueryEngine', symbols=True, lang='rust', limit=10)
```

---

### GET /stats

Get index statistics and metadata.

**URL:** `/stats`

**Method:** `GET`

**Query Parameters:** None

**Response:** `application/json`

```json
{
  "total_files": 1247,
  "index_size_bytes": 2145728,
  "last_updated": "2025-11-03T14:32:45Z",
  "files_by_language": {
    "Rust": 842,
    "TypeScript": 305,
    "Python": 100
  },
  "lines_by_language": {
    "Rust": 45230,
    "TypeScript": 18445,
    "Python": 5320
  }
}
```

**Response Fields:**

- `total_files`: Total number of indexed files
- `index_size_bytes`: Total cache size in bytes
- `last_updated`: ISO 8601 timestamp of last indexing operation
- `files_by_language`: File count breakdown by language
- `lines_by_language`: Line count breakdown by language

**HTTP Status Codes:**

- `200 OK`: Stats retrieved successfully
- `404 Not Found`: No index found (run `POST /index` first)
- `500 Internal Server Error`: Failed to read stats

**Examples:**

```bash
# Get statistics
curl http://localhost:7878/stats | jq '.'

# Check index size
curl -s http://localhost:7878/stats | jq '.index_size_bytes / 1024 / 1024 | floor'
# → Cache size in MB

# Check file count
curl -s http://localhost:7878/stats | jq '.total_files'
```

**JavaScript Example:**

```javascript
async function getStats() {
  const response = await fetch('http://localhost:7878/stats');
  if (!response.ok) {
    throw new Error(`HTTP ${response.status}: ${await response.text()}`);
  }
  return response.json();
}

// Usage
const stats = await getStats();
console.log(`Indexed ${stats.total_files} files (${stats.index_size_bytes} bytes)`);
```

---

### POST /index

Trigger indexing or reindexing of the codebase.

**URL:** `/index`

**Method:** `POST`

**Request Headers:**

- `Content-Type: application/json`

**Request Body:** (optional, default: `{}`)

```json
{
  "force": boolean,
  "languages": [string]
}
```

**Request Fields:**

- `force` (optional, default: `false`): Force full rebuild (ignore incremental cache)
- `languages` (optional, default: all): Array of language names to index (e.g., `["rust", "typescript"]`)

**Response:** `application/json`

Same as [GET /stats](#get-stats) response schema.

**HTTP Status Codes:**

- `200 OK`: Indexing completed successfully
- `500 Internal Server Error`: Indexing failed

**Examples:**

```bash
# Incremental index (only changed files)
curl -X POST http://localhost:7878/index \
  -H "Content-Type: application/json" \
  -d '{}'

# Force full rebuild
curl -X POST http://localhost:7878/index \
  -H "Content-Type: application/json" \
  -d '{"force": true}'

# Index specific languages
curl -X POST http://localhost:7878/index \
  -H "Content-Type: application/json" \
  -d '{"languages": ["rust", "typescript"]}'

# Force rebuild with language filter
curl -X POST http://localhost:7878/index \
  -H "Content-Type: application/json" \
  -d '{"force": true, "languages": ["rust"]}'
```

**JavaScript Example:**

```javascript
async function reindex(options = {}) {
  const response = await fetch('http://localhost:7878/index', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(options)
  });

  if (!response.ok) {
    throw new Error(`HTTP ${response.status}: ${await response.text()}`);
  }

  return response.json();
}

// Usage
const stats = await reindex({ force: true });
console.log(`Indexed ${stats.total_files} files`);
```

**Python Example:**

```python
import requests

def reindex(force=False, languages=None):
    payload = {'force': force}
    if languages:
        payload['languages'] = languages

    response = requests.post(
        'http://localhost:7878/index',
        json=payload
    )
    response.raise_for_status()
    return response.json()

# Usage
stats = reindex(force=True, languages=['rust', 'typescript'])
print(f"Indexed {stats['total_files']} files")
```

**Note:** Indexing is **synchronous** and may take several seconds for large codebases. The HTTP request will block until indexing completes.

---

### GET /health

Simple health check endpoint for monitoring.

**URL:** `/health`

**Method:** `GET`

**Query Parameters:** None

**Response:** `text/plain`

```
Reflex is running
```

**HTTP Status Codes:**

- `200 OK`: Server is healthy

**Examples:**

```bash
# Health check
curl http://localhost:7878/health
# → "Reflex is running"

# Use in monitoring scripts
if curl -sf http://localhost:7878/health > /dev/null; then
  echo "Reflex is healthy"
else
  echo "Reflex is down"
fi
```

---

## Data Schemas

### SearchResult Schema

```typescript
interface SearchResult {
  file: string;           // Relative file path
  line: number;           // Line number (1-indexed)
  column: number;         // Column number (1-indexed)
  match: string;          // Matched line content
  symbol?: string;        // Symbol name (symbol search only)
  kind?: SymbolKind;      // Symbol kind (symbol search only)
  language?: Language;    // Language (symbol search only)
  context_before: string[]; // Lines before match (for context)
  context_after: string[];  // Lines after match (for context)
}
```

**SymbolKind Values:**

- `"Function"`, `"Class"`, `"Struct"`, `"Enum"`, `"Trait"`, `"Interface"`, `"Type"`, `"Constant"`, `"Variable"`, `"Method"`, `"Property"`, `"Module"`, `"Namespace"`, etc.

**Language Values:**

- `"Rust"`, `"Python"`, `"JavaScript"`, `"TypeScript"`, `"Vue"`, `"Svelte"`, `"Go"`, `"Java"`, `"PHP"`, `"C"`, `"Cpp"`, `"CSharp"`, `"Ruby"`, `"Kotlin"`, `"Zig"`

---

## Supported Languages

The following language identifiers can be used with the `lang` query parameter:

| Language | Identifier | Aliases |
|----------|-----------|---------|
| Rust | `rust` | `rs` |
| Python | `python` | `py` |
| JavaScript | `javascript` | `js` |
| TypeScript | `typescript` | `ts` |
| Vue | `vue` | - |
| Svelte | `svelte` | - |
| Go | `go` | - |
| Java | `java` | - |
| PHP | `php` | - |
| C | `c` | - |
| C++ | `cpp` | `c++` |
| C# | `csharp` | `cs`, `c#` |
| Ruby | `ruby` | `rb` |
| Kotlin | `kotlin` | `kt` |
| Zig | `zig` | - |

**Example:**

```bash
# All of these work
curl 'http://localhost:7878/query?q=main&lang=rust'
curl 'http://localhost:7878/query?q=main&lang=rs'

curl 'http://localhost:7878/query?q=console&lang=typescript'
curl 'http://localhost:7878/query?q=console&lang=ts'
```

---

## Error Handling

### HTTP Status Codes

- `200 OK`: Request successful
- `400 Bad Request`: Invalid request parameters
- `404 Not Found`: Resource not found (e.g., no index exists)
- `500 Internal Server Error`: Server-side error

### Error Response Format

Errors return plain text error messages:

```
HTTP/1.1 400 Bad Request
Content-Type: text/plain

Unknown language 'foobar'. Supported languages: rust, javascript (js), typescript (ts), vue, svelte, php, python (py), go, java, c, cpp (c++)
```

**Error Handling in Clients:**

```javascript
// JavaScript
try {
  const response = await fetch('http://localhost:7878/query?q=test');
  if (!response.ok) {
    const error = await response.text();
    throw new Error(`HTTP ${response.status}: ${error}`);
  }
  return response.json();
} catch (error) {
  console.error('Query failed:', error.message);
}
```

```python
# Python
try:
    response = requests.get('http://localhost:7878/query', params={'q': 'test'})
    response.raise_for_status()
    return response.json()
except requests.HTTPError as e:
    print(f"Query failed: HTTP {e.response.status_code}: {e.response.text}")
```

---

## Integration Patterns

### Editor Plugins

**VSCode Extension Example:**

```javascript
import * as vscode from 'vscode';

class ReflexSearchProvider implements vscode.TreeDataProvider<SearchResult> {
  async search(pattern: string): Promise<SearchResult[]> {
    const response = await fetch(
      `http://localhost:7878/query?q=${encodeURIComponent(pattern)}&limit=50`
    );
    const data = await response.json();
    return data.results;
  }
}
```

**Neovim Plugin Example:**

```lua
local function reflex_search(pattern)
  local url = string.format('http://localhost:7878/query?q=%s', vim.fn.escape(pattern, '&?'))
  local result = vim.fn.system(string.format('curl -s "%s"', url))
  local data = vim.fn.json_decode(result)
  return data.results
end

vim.api.nvim_create_user_command('ReflexSearch', function(opts)
  local results = reflex_search(opts.args)
  -- Display results in quickfix list
  vim.fn.setqflist(results, 'r')
  vim.cmd('copen')
end, { nargs = 1 })
```

### CI/CD Integration

**Enforce Code Standards:**

```bash
#!/bin/bash
# Check for TODO comments in production code

TODOS=$(curl -s 'http://localhost:7878/query?q=TODO&file=src/&limit=100' | jq '.results | length')

if [ "$TODOS" -gt 0 ]; then
  echo "❌ Found $TODOS TODO comments in src/. Please resolve before merging."
  exit 1
fi

echo "✅ No TODO comments found"
```

**Security Scanning:**

```bash
#!/bin/bash
# Check for potential security issues

PATTERNS=("unwrap(" "expect(" "unsafe" ".clone()")

for pattern in "${PATTERNS[@]}"; do
  COUNT=$(curl -s "http://localhost:7878/query?q=$pattern&count=true" | jq '.results | length')
  echo "$pattern: $COUNT occurrences"
done
```

### AI Agent Integration

**LangChain Tool:**

```python
from langchain.tools import BaseTool
import requests

class ReflexCodeSearchTool(BaseTool):
    name = "reflex_search"
    description = "Search codebase for patterns, functions, or symbols"

    def _run(self, query: str, symbols: bool = False, lang: str = None) -> str:
        params = {'q': query, 'symbols': symbols, 'limit': 10}
        if lang:
            params['lang'] = lang

        response = requests.get('http://localhost:7878/query', params=params)
        data = response.json()

        if not data['results']:
            return "No results found"

        # Format results for LLM
        results_text = []
        for r in data['results']:
            results_text.append(f"{r['file']}:{r['line']} - {r['match']}")

        return "\n".join(results_text)
```

### Monitoring Dashboard

**Health Check Endpoint:**

```javascript
// Check if Reflex server is running
async function checkHealth() {
  try {
    const response = await fetch('http://localhost:7878/health', {
      method: 'GET',
      timeout: 5000
    });
    return response.ok;
  } catch {
    return false;
  }
}

// Monitor index freshness
async function checkIndexStatus() {
  const stats = await fetch('http://localhost:7878/stats').then(r => r.json());
  const lastUpdated = new Date(stats.last_updated);
  const hoursSinceUpdate = (Date.now() - lastUpdated) / (1000 * 60 * 60);

  if (hoursSinceUpdate > 24) {
    console.warn(`Index is ${hoursSinceUpdate.toFixed(1)} hours old. Consider reindexing.`);
  }
}
```

---

## Performance Considerations

### Query Optimization

- **Use filters**: Narrow results with `lang`, `kind`, `file` parameters
- **Set limits**: Use `limit` parameter to avoid large result sets
- **Use timeouts**: Set `timeout` parameter for expensive queries
- **Prefer symbols mode**: `symbols=true` is faster for definition lookups

### Caching Strategy

The Reflex server does not cache query results. Each query recomputes from the index. For high-frequency queries:

1. Cache results in your client application
2. Invalidate cache when files change (use file watcher or polling)
3. Use `GET /stats` to detect index updates via `last_updated` timestamp

### Concurrent Requests

The HTTP server handles concurrent requests safely. All endpoints can be called in parallel.

**Example: Parallel Queries**

```javascript
// Search multiple patterns concurrently
const patterns = ['QueryEngine', 'IndexStats', 'SearchResult'];
const results = await Promise.all(
  patterns.map(p => fetch(`http://localhost:7878/query?q=${p}&symbols=true`))
);
const data = await Promise.all(results.map(r => r.json()));
```

---

## Security Considerations

### Local-Only Deployment

The Reflex HTTP server is designed for **local-only use**. By default, it binds to `127.0.0.1` (localhost) and is not accessible from the network.

**Do not expose to public networks:**

```bash
# ✅ Safe: localhost only
rfx serve --host 127.0.0.1

# ⚠️  Caution: accessible on LAN
rfx serve --host 0.0.0.0

# ❌ Never: public internet
# Do not expose port 7878 to the internet
```

### No Authentication

The API has **no authentication or authorization**. Anyone with network access to the server can:

- Read your entire codebase via queries
- Trigger reindexing operations
- Access file paths and contents

**Mitigation strategies:**

1. Bind to `127.0.0.1` only (default)
2. Use firewall rules to restrict access
3. Run behind a reverse proxy with authentication (nginx, Caddy)
4. Use SSH tunneling for remote access

### CORS Configuration

CORS is enabled for all origins (`Access-Control-Allow-Origin: *`) to support browser-based tools running on `localhost`.

For production use, consider configuring a reverse proxy with stricter CORS policies.

---

## Troubleshooting

### Server Won't Start

**Error:** `Failed to bind to 127.0.0.1:7878`

**Solution:** Port is already in use. Check for existing Reflex instances:

```bash
# Check if port is in use
lsof -i :7878

# Kill existing server
pkill rfx

# Use a different port
rfx serve --port 8080
```

### Index Not Found (404)

**Error:** `GET /stats` returns `404 Not Found`

**Solution:** No index exists. Run indexing first:

```bash
# Via API
curl -X POST http://localhost:7878/index

# Or via CLI
rfx index
```

### Query Timeout

**Error:** Query takes too long and times out

**Solution:**

1. Increase timeout: `GET /query?q=pattern&timeout=60`
2. Narrow search with filters: `&lang=rust&file=src/`
3. Use symbols mode: `&symbols=true`
4. Reindex for better performance: `POST /index`

### Empty Results

**Error:** Query returns `results: []` but you expect matches

**Troubleshooting:**

1. Check if pattern is too specific
2. Try full-text search instead of symbols: remove `symbols=true`
3. Check language filter matches file types: `&lang=rust`
4. Verify files are indexed: `GET /stats`

---

## API Versioning

The Reflex API currently has **no versioning**. The API is stable for the v1.x release series.

**Breaking changes** will be introduced in major version bumps (e.g., v2.0.0) and will be documented in the CHANGELOG.md.

**Compatibility promise:**

- Existing endpoints will not be removed in minor/patch versions
- New optional parameters may be added in minor versions
- Response schema may be extended (new fields added) in minor versions
- Clients should ignore unknown fields for forward compatibility

---

## Further Reading

- [README.md](README.md): Quick start guide and CLI reference
- [ARCHITECTURE.md](ARCHITECTURE.md): System design and internals
- [CLAUDE.md](CLAUDE.md): Development workflow and project philosophy
- [CHANGELOG.md](CHANGELOG.md): Version history and release notes

---

**Questions or issues?** Open an issue at [github.com/therecluse26/reflex/issues](https://github.com/therecluse26/reflex/issues)
