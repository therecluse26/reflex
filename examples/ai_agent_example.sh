#!/bin/bash
# Example: AI agent using RefLex with automatic re-indexing

# Query the codebase
response=$(rfx query "CacheManager" --json --limit 5)

# Parse the status field using jq
status=$(echo "$response" | jq -r '.metadata.status')

# Check if index is stale
if [ "$status" != "fresh" ]; then
    echo "⚠️  Index is stale (status: $status)" >&2

    # Get the suggested action
    action=$(echo "$response" | jq -r '.metadata.action_required // "rfx index"')

    echo "🔄 Running: $action" >&2
    $action

    # Re-run the query with fresh index
    echo "🔍 Re-running query with fresh index..." >&2
    response=$(rfx query "CacheManager" --json --limit 5)
fi

# Use the results
echo "$response" | jq -r '.results[] | "\(.path):\(.span.start_line) - \(.symbol)"'
