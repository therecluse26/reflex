# AI Agent Integration Guide

RefLex is designed to be AI-agent-friendly with JSON output that includes index metadata. This allows agents to:

1. **Detect stale indices** automatically
2. **Re-index when needed** without user intervention
3. **Trust the results** knowing they're up-to-date

## JSON Output Format

When using the `--json` flag, RefLex returns a structured response with metadata:

```json
{
  "metadata": {
    "status": "fresh" | "branch_not_indexed" | "commit_changed" | "files_modified",
    "reason": "Human-readable explanation (if stale)",
    "current_branch": "main",
    "indexed_branch": "main",
    "current_commit": "74eb454...",
    "indexed_commit": "74eb454...",
    "action_required": "reflex index"
  },
  "results": [
    {
      "path": "./src/cache.rs",
      "lang": "rust",
      "kind": "Struct",
      "symbol": "CacheManager",
      "span": {
        "start_line": 42,
        "start_col": 0,
        "end_line": 45,
        "end_col": 1
      },
      "scope": null,
      "preview": "pub struct CacheManager {"
    }
  ]
}
```

## Metadata Fields

### `status` (required)
- `"fresh"`: Index is up-to-date
- `"branch_not_indexed"`: Current git branch has not been indexed
- `"commit_changed"`: HEAD has moved since last index
- `"files_modified"`: Files have been modified (detected via sampling)

### `reason` (optional)
Human-readable explanation of why the index is stale (omitted if fresh).

### `action_required` (optional)
Command to run to fix staleness (typically `"reflex index"`). Omitted if fresh.

### Git fields (optional)
Only present if in a git repository:
- `current_branch`: The branch you're currently on
- `indexed_branch`: The branch that was indexed
- `current_commit`: Current HEAD commit SHA
- `indexed_commit`: Commit SHA when index was created

## Example: Bash Script

```bash
#!/bin/bash
response=$(reflex query "pattern" --json)
status=$(echo "$response" | jq -r '.metadata.status')

if [ "$status" != "fresh" ]; then
    echo "⚠️  Index is stale, re-indexing..." >&2
    reflex index
    response=$(reflex query "pattern" --json)
fi

# Use results
echo "$response" | jq '.results'
```

## Example: Python

```python
import subprocess
import json

def query_with_auto_reindex(pattern: str):
    """Query RefLex, automatically re-index if stale."""
    response = json.loads(
        subprocess.check_output(["reflex", "query", pattern, "--json"])
    )

    if response["metadata"]["status"] != "fresh":
        print(f"⚠️  Index stale: {response['metadata']['reason']}")
        subprocess.run(["reflex", "index"], check=True)

        # Re-query with fresh index
        response = json.loads(
            subprocess.check_output(["reflex", "query", pattern, "--json"])
        )

    return response["results"]

# Usage
results = query_with_auto_reindex("CacheManager")
for result in results:
    print(f"{result['path']}:{result['span']['start_line']} - {result['symbol']}")
```

## Example: TypeScript/JavaScript

```typescript
import { exec } from 'child_process';
import { promisify } from 'util';

const execAsync = promisify(exec);

async function queryWithAutoReindex(pattern: string) {
  const { stdout } = await execAsync(`reflex query "${pattern}" --json`);
  let response = JSON.parse(stdout);

  if (response.metadata.status !== 'fresh') {
    console.error(`⚠️  Index stale: ${response.metadata.reason}`);
    await execAsync('reflex index');

    // Re-query
    const { stdout: newStdout } = await execAsync(`reflex query "${pattern}" --json`);
    response = JSON.parse(newStdout);
  }

  return response.results;
}

// Usage
const results = await queryWithAutoReindex('CacheManager');
results.forEach(r => {
  console.log(`${r.path}:${r.span.start_line} - ${r.symbol}`);
});
```

## Best Practices for AI Agents

1. **Always use `--json` for programmatic access**: This ensures you get structured metadata.

2. **Check `metadata.status` before trusting results**: Don't assume the index is fresh.

3. **Auto-reindex on stale**: If `status !== "fresh"`, run the `action_required` command.

4. **Handle errors gracefully**: Index might not exist at all (first run).

5. **Cache awareness**: Indexing is fast (~200ms for most projects), so re-indexing on demand is acceptable.

6. **Branch switching**: If you switch git branches, expect `status: "branch_not_indexed"` until you index the new branch.

## Performance Notes

- **Index freshness check**: <5ms overhead (lightweight git operations)
- **File sampling**: Checks up to 10 files for modifications (cheap mtime + hash)
- **Re-indexing**: ~200ms for typical projects (incremental, only changed files)

## Exit Codes

RefLex uses standard exit codes:
- `0`: Success (results found and index fresh)
- `1`: No results found or error
- (Future) `2`: Results found but index stale

## Non-JSON Fallback

Without `--json`, RefLex prints warnings to stderr:

```bash
$ reflex query "pattern"
⚠️  WARNING: Index may be stale (commit changed: abc1234 → def5678). Consider running 'reflex index'.
Found 42 results in 5ms:
...
```

This is useful for human operators but not recommended for AI agents.
