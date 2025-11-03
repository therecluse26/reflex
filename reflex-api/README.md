# RefLex API - Bruno Collection

This is a [Bruno](https://www.usebruno.com/) API collection for testing the RefLex HTTP server.

## Prerequisites

1. **Start the RefLex server:**
   ```bash
   rfx serve --port 7878 --host 127.0.0.1
   ```

2. **Ensure you have an index:**
   ```bash
   rfx index
   ```

3. **Install Bruno** (optional - collection works with any HTTP client):
   - Download from https://www.usebruno.com/
   - Or use curl/httpie with the .bru files as reference

## Using This Collection

### With Bruno CLI (bru)
```bash
# Install Bruno CLI
npm install -g @usebruno/cli

# Run all requests
bru run reflex-api --env Local

# Run a specific request
bru run reflex-api --env Local -r "Health Check"
```

### With Bruno Desktop
1. Open Bruno
2. Click "Open Collection"
3. Navigate to this directory (`reflex-api/`)
4. Select the "Local" environment
5. Run any request

### With curl
Each `.bru` file contains a curl-compatible request. Example:
```bash
curl http://localhost:7878/health
curl 'http://localhost:7878/query?q=QueryEngine&limit=5'
curl http://localhost:7878/stats
```

## Included Requests

### Basic Operations
1. **Health Check** - Verify server is running
2. **Get Stats** - View index statistics

### Query Variants
3. **Query - Full Text Search** - Find all occurrences
4. **Query - Symbol Search** - Find definitions only
5. **Query - Language Filter** - Search specific language
6. **Query - Regex Search** - Pattern matching
7. **Query - File Filter** - Filter by file path
8. **Query - Exact Match** - Exact symbol names
9. **Query - Expanded Results** - Full symbol bodies
10. **Query - Combined Filters** - Multiple filters at once

### Indexing Operations
11. **Index - Trigger Full Reindex** - Force rebuild
12. **Index - Incremental with Language Filter** - Update specific languages

## API Endpoints

### GET /health
Simple health check.

### GET /stats
Get index statistics (files, lines, languages).

### GET /query
Search the codebase.

**Query Parameters:**
- `q` (required) - Search pattern
- `lang` - Filter by language (rust, typescript, python, etc.)
- `kind` - Filter by symbol kind (function, class, struct, etc.)
- `limit` - Maximum number of results
- `symbols` - Symbol-only search (boolean)
- `regex` - Enable regex mode (boolean)
- `exact` - Exact match only (boolean)
- `expand` - Show full symbol body (boolean)
- `file` - Filter by file path substring

**Example:**
```bash
curl 'http://localhost:7878/query?q=parse&lang=rust&kind=function&symbols=true&limit=10'
```

### POST /index
Trigger reindexing.

**Request Body:**
```json
{
  "force": false,
  "languages": ["rust", "typescript"]
}
```

**Example:**
```bash
curl -X POST http://localhost:7878/index \
  -H "Content-Type: application/json" \
  -d '{"force": false, "languages": ["rust"]}'
```

## Response Format

### QueryResponse
```json
{
  "status": "fresh",
  "can_trust_results": true,
  "warning": null,
  "results": [
    {
      "path": "./src/query.rs",
      "lang": "rust",
      "kind": "Struct",
      "symbol": "QueryEngine",
      "span": {
        "start_line": 42,
        "start_col": 0,
        "end_line": 45,
        "end_col": 1
      },
      "scope": null,
      "preview": "pub struct QueryEngine {\n    cache: CacheManager,\n}"
    }
  ]
}
```

### IndexStats
```json
{
  "total_files": 35,
  "index_size_bytes": 2745787,
  "last_updated": "2025-11-03T06:20:12+00:00",
  "files_by_language": {
    "Rust": 29,
    "TypeScript": 2,
    "Python": 2,
    "JavaScript": 1,
    "PHP": 1
  },
  "lines_by_language": {
    "Rust": 14736,
    "TypeScript": 18
  }
}
```

## Testing

Each request includes automated tests that verify:
- HTTP status codes
- Response structure
- Data types
- Filter application

Run all tests:
```bash
bru run reflex-api --env Local
```

## Environment Variables

The collection uses the `Local` environment with:
- `baseUrl`: `http://localhost:7878`

You can create additional environments (e.g., `Production`, `Development`) by copying `environments/Local.bru` and modifying the `baseUrl`.

## CORS

The RefLex HTTP server has CORS enabled, so you can use these endpoints from browser-based tools and applications.

## Notes

- All query operations are synchronous
- Indexing operations return after completion (not async)
- Results are deterministic (same query → same results)
- JSON responses are compatible with AI agents and automation tools

## Troubleshooting

**Server not responding?**
```bash
# Check if server is running
curl http://localhost:7878/health

# Restart server
rfx serve --port 7878
```

**No results?**
```bash
# Ensure index exists
rfx stats

# Rebuild index
rfx index --force
```

**Want verbose logging?**
```bash
# Start server with debug logging
RUST_LOG=debug rfx serve --port 7878
```
