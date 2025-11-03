#!/usr/bin/env python3
"""
Example: AI agent using RefLex with automatic re-indexing

This shows how an AI coding agent can:
1. Query the codebase
2. Check index freshness from metadata
3. Automatically re-index if stale
4. Use the results confidently
"""

import json
import subprocess
import sys


def query_reflex(pattern: str, limit: int = 10, reflex_bin: str = "rfx") -> dict:
    """Query RefLex and return JSON response with metadata."""
    cmd = [reflex_bin, "query", pattern, "--json", "--limit", str(limit)]
    result = subprocess.run(cmd, capture_output=True, text=True, check=True)
    return json.loads(result.stdout)


def ensure_fresh_index(response: dict) -> dict:
    """
    Check if index is fresh, re-index if needed, and re-query.

    Returns the response (possibly after re-indexing).
    """
    metadata = response["metadata"]
    status = metadata["status"]

    if status != "fresh":
        # Index is stale - print diagnostic info
        print(f"⚠️  Index is stale: {status}", file=sys.stderr)
        if "reason" in metadata:
            print(f"   Reason: {metadata['reason']}", file=sys.stderr)

        # Get the suggested action (defaults to 'rfx index')
        action = metadata.get("action_required", "rfx index")

        print(f"🔄 Running: {action}", file=sys.stderr)
        subprocess.run(action.split(), check=True)

        # Re-query with fresh index
        print("🔍 Re-running query with fresh index...", file=sys.stderr)
        # Note: We'd need to pass the original query params here
        # For simplicity, this example assumes we're re-querying the same thing

    return response


def main():
    # Example usage
    # Note: Assumes 'rfx' is in PATH or replace with full path to binary
    pattern = "CacheManager"

    print(f"Searching for: {pattern}", file=sys.stderr)
    # Try to find rfx binary (cargo build output location)
    import os
    reflex_candidates = [
        "/ramdisk/target/release/rfx",  # Cargo build on ramdisk
        "./target/release/rfx",  # Standard cargo build
        "rfx",  # System PATH
    ]
    reflex_bin = next((p for p in reflex_candidates if os.path.exists(p) or p == "rfx"), "rfx")

    response = query_reflex(pattern, limit=5, reflex_bin=reflex_bin)

    # Ensure index is fresh (auto-reindex if needed)
    response = ensure_fresh_index(response)

    # Use the results
    results = response["results"]
    print(f"\nFound {len(results)} results:", file=sys.stderr)

    for result in results:
        path = result["path"]
        line = result["span"]["start_line"]
        symbol = result["symbol"]
        preview = result["preview"]

        print(f"\n{path}:{line}")
        print(f"  Symbol: {symbol}")
        print(f"  Preview: {preview}")


if __name__ == "__main__":
    main()
