#!/usr/bin/env python3
"""
Claude Code Benchmark Results Analyzer

This script analyzes benchmark results from run_benchmark.sh and generates:
1. Summary statistics (avg duration, success rate, etc.)
2. Category-wise breakdowns
3. Comparison metrics
4. Performance charts (optional, requires matplotlib)

Usage:
    python3 analyze_results.py results/benchmark_TIMESTAMP.json
    python3 analyze_results.py results/benchmark_TIMESTAMP.json --plot
"""

import json
import sys
import argparse
from collections import defaultdict
from pathlib import Path
from typing import Dict, List, Any


def load_results(filepath: str) -> List[Dict[str, Any]]:
    """Load benchmark results from JSON file."""
    with open(filepath, 'r') as f:
        return json.load(f)


def calculate_summary_stats(results: List[Dict[str, Any]]) -> Dict[str, Any]:
    """Calculate overall summary statistics."""
    total_tests = len(results)
    passed_tests = sum(1 for r in results if r['status'] == 'PASS')
    failed_tests = total_tests - passed_tests

    durations = [r['duration_ms'] for r in results if r['status'] == 'PASS']
    output_sizes = [r['output_size_bytes'] for r in results if r['status'] == 'PASS']

    return {
        'total_tests': total_tests,
        'passed': passed_tests,
        'failed': failed_tests,
        'success_rate': (passed_tests / total_tests * 100) if total_tests > 0 else 0,
        'avg_duration_ms': sum(durations) / len(durations) if durations else 0,
        'min_duration_ms': min(durations) if durations else 0,
        'max_duration_ms': max(durations) if durations else 0,
        'avg_output_size_bytes': sum(output_sizes) / len(output_sizes) if output_sizes else 0,
        'total_output_size_bytes': sum(output_sizes),
    }


def group_by_category(results: List[Dict[str, Any]]) -> Dict[str, List[Dict[str, Any]]]:
    """Group results by category."""
    grouped = defaultdict(list)
    for result in results:
        grouped[result['category']].append(result)
    return dict(grouped)


def group_by_complexity(results: List[Dict[str, Any]]) -> Dict[str, List[Dict[str, Any]]]:
    """Group results by complexity level."""
    grouped = defaultdict(list)
    for result in results:
        grouped[result['complexity']].append(result)
    return dict(grouped)


def calculate_category_stats(results: List[Dict[str, Any]]) -> Dict[str, Any]:
    """Calculate statistics for a category."""
    total = len(results)
    passed = sum(1 for r in results if r['status'] == 'PASS')
    durations = [r['duration_ms'] for r in results if r['status'] == 'PASS']
    output_sizes = [r['output_size_bytes'] for r in results if r['status'] == 'PASS']

    return {
        'total_tests': total,
        'passed': passed,
        'success_rate': (passed / total * 100) if total > 0 else 0,
        'avg_duration_ms': sum(durations) / len(durations) if durations else 0,
        'avg_output_size_bytes': sum(output_sizes) / len(output_sizes) if output_sizes else 0,
    }


def print_summary(stats: Dict[str, Any]) -> None:
    """Print summary statistics."""
    print("=" * 60)
    print("BENCHMARK SUMMARY")
    print("=" * 60)
    print(f"Total Tests:        {stats['total_tests']}")
    print(f"Passed:             {stats['passed']} ({stats['success_rate']:.1f}%)")
    print(f"Failed:             {stats['failed']}")
    print()
    print(f"Average Duration:   {stats['avg_duration_ms']:.2f} ms")
    print(f"Min Duration:       {stats['min_duration_ms']:.2f} ms")
    print(f"Max Duration:       {stats['max_duration_ms']:.2f} ms")
    print()
    print(f"Average Output Size: {stats['avg_output_size_bytes']:,.0f} bytes")
    print(f"Total Output Size:   {stats['total_output_size_bytes']:,.0f} bytes")
    print("=" * 60)
    print()


def print_category_breakdown(category_results: Dict[str, List[Dict[str, Any]]]) -> None:
    """Print category-wise breakdown."""
    print("=" * 60)
    print("CATEGORY BREAKDOWN")
    print("=" * 60)

    for category, results in sorted(category_results.items()):
        stats = calculate_category_stats(results)
        print(f"\n{category}:")
        print(f"  Tests:          {stats['total_tests']}")
        print(f"  Success Rate:   {stats['success_rate']:.1f}%")
        print(f"  Avg Duration:   {stats['avg_duration_ms']:.2f} ms")
        print(f"  Avg Output:     {stats['avg_output_size_bytes']:,.0f} bytes")

    print()


def print_complexity_breakdown(complexity_results: Dict[str, List[Dict[str, Any]]]) -> None:
    """Print complexity-wise breakdown."""
    print("=" * 60)
    print("COMPLEXITY BREAKDOWN")
    print("=" * 60)

    complexity_order = ['Simple', 'Medium', 'Complex']
    for complexity in complexity_order:
        if complexity not in complexity_results:
            continue

        results = complexity_results[complexity]
        stats = calculate_category_stats(results)
        print(f"\n{complexity}:")
        print(f"  Tests:          {stats['total_tests']}")
        print(f"  Success Rate:   {stats['success_rate']:.1f}%")
        print(f"  Avg Duration:   {stats['avg_duration_ms']:.2f} ms")
        print(f"  Avg Output:     {stats['avg_output_size_bytes']:,.0f} bytes")

    print()


def print_slowest_tests(results: List[Dict[str, Any]], top_n: int = 10) -> None:
    """Print slowest tests."""
    print("=" * 60)
    print(f"TOP {top_n} SLOWEST TESTS")
    print("=" * 60)

    sorted_results = sorted(
        [r for r in results if r['status'] == 'PASS'],
        key=lambda x: x['duration_ms'],
        reverse=True
    )[:top_n]

    for i, result in enumerate(sorted_results, 1):
        print(f"{i}. {result['test_id']}: {result['test_name']}")
        print(f"   Duration: {result['duration_ms']} ms")
        print(f"   Category: {result['category']}")
        print()


def print_largest_outputs(results: List[Dict[str, Any]], top_n: int = 10) -> None:
    """Print tests with largest outputs."""
    print("=" * 60)
    print(f"TOP {top_n} LARGEST OUTPUTS")
    print("=" * 60)

    sorted_results = sorted(
        [r for r in results if r['status'] == 'PASS'],
        key=lambda x: x['output_size_bytes'],
        reverse=True
    )[:top_n]

    for i, result in enumerate(sorted_results, 1):
        size_kb = result['output_size_bytes'] / 1024
        print(f"{i}. {result['test_id']}: {result['test_name']}")
        print(f"   Output Size: {size_kb:.2f} KB ({result['output_lines']} lines)")
        print(f"   Category: {result['category']}")
        print()


def estimate_token_savings(results: List[Dict[str, Any]]) -> None:
    """Estimate token count and savings vs built-in tools (TOOL OUTPUT ONLY)."""
    print("=" * 60)
    print("TOOL OUTPUT TOKEN IMPACT (Partial View)")
    print("=" * 60)
    print()

    # Rough estimation: 1 token ≈ 4 characters
    # Tool call overhead: ~50 tokens per tool call
    CHARS_PER_TOKEN = 4
    TOOL_CALL_OVERHEAD = 50

    # Calculate RFX token usage
    rfx_tool_calls = len(results)
    rfx_output_chars = sum(r['output_size_bytes'] for r in results if r['status'] == 'PASS')
    rfx_output_tokens = rfx_output_chars / CHARS_PER_TOKEN
    rfx_overhead_tokens = rfx_tool_calls * TOOL_CALL_OVERHEAD
    rfx_total_tokens = rfx_output_tokens + rfx_overhead_tokens

    # Estimate built-in tool usage (conservative: 2-3x more tool calls for complex tasks)
    # Category-based multipliers
    complexity_multipliers = {
        'Simple': 1.5,      # Simple tasks: 1 rfx call → 1.5 built-in calls
        'Medium': 2.5,      # Medium tasks: 1 rfx call → 2.5 built-in calls
        'Complex': 5.0,     # Complex tasks: 1 rfx call → 5 built-in calls
    }

    complexity_groups = group_by_complexity(results)
    builtin_tool_calls = 0
    for complexity, complexity_results in complexity_groups.items():
        multiplier = complexity_multipliers.get(complexity, 2.0)
        builtin_tool_calls += len(complexity_results) * multiplier

    # Built-in tools often return more verbose output (full file reads, etc.)
    builtin_output_chars = rfx_output_chars * 1.8  # Conservative 1.8x estimate
    builtin_output_tokens = builtin_output_chars / CHARS_PER_TOKEN
    builtin_overhead_tokens = builtin_tool_calls * TOOL_CALL_OVERHEAD
    builtin_total_tokens = builtin_output_tokens + builtin_overhead_tokens

    # Calculate savings
    tool_output_savings = builtin_total_tokens - rfx_total_tokens
    tool_output_savings_pct = (tool_output_savings / builtin_total_tokens * 100) if builtin_total_tokens > 0 else 0

    print(f"RFX Tool Output:")
    print(f"  Tool Calls:       {rfx_tool_calls}")
    print(f"  Output Tokens:    {rfx_output_tokens:,.0f}")
    print(f"  Overhead Tokens:  {rfx_overhead_tokens:,.0f}")
    print(f"  Total Tokens:     {rfx_total_tokens:,.0f}")
    print()

    print(f"Built-in Tool Output (Estimated):")
    print(f"  Tool Calls:       {builtin_tool_calls:.0f}")
    print(f"  Output Tokens:    {builtin_output_tokens:,.0f}")
    print(f"  Overhead Tokens:  {builtin_overhead_tokens:,.0f}")
    print(f"  Total Tokens:     {builtin_total_tokens:,.0f}")
    print()

    print(f"Tool Output Savings:")
    print(f"  Token Reduction:  {tool_output_savings:,.0f} tokens ({tool_output_savings_pct:.1f}%)")
    print(f"  Tool Call Reduction: {builtin_tool_calls - rfx_tool_calls:.0f} calls")
    print()

    print("⚠️  IMPORTANT: This only shows TOOL OUTPUT savings!")
    print("    Real conversations also include:")
    print("    - User prompts (~20-100 tokens per message)")
    print("    - Claude's reasoning/explanations (~50-200 tokens)")
    print("    - Conversation context (accumulates over turns)")
    print()


def estimate_realistic_token_savings(results: List[Dict[str, Any]]) -> None:
    """Estimate REALISTIC total token savings including all conversation components."""
    print("=" * 60)
    print("REALISTIC TOTAL TOKEN IMPACT")
    print("=" * 60)
    print()
    print("This estimates the full conversation tokens you see in Claude Code's UI")
    print()

    # Constants
    CHARS_PER_TOKEN = 4
    TOOL_CALL_OVERHEAD = 50
    USER_PROMPT_TOKENS = 25  # Average prompt: "Find X definition"
    CLAUDE_REASONING_TOKENS = 120  # Claude explains what it's doing
    CLAUDE_RESPONSE_TOKENS = 80  # Claude summarizes results

    # Get tool output tokens (from benchmark)
    rfx_tool_calls = len([r for r in results if r['status'] == 'PASS'])
    rfx_output_chars = sum(r['output_size_bytes'] for r in results if r['status'] == 'PASS')
    rfx_tool_output_tokens = rfx_output_chars / CHARS_PER_TOKEN
    rfx_tool_overhead = rfx_tool_calls * TOOL_CALL_OVERHEAD

    # Calculate TOTAL conversation tokens for RFX
    rfx_user_tokens = rfx_tool_calls * USER_PROMPT_TOKENS
    rfx_claude_reasoning = rfx_tool_calls * CLAUDE_REASONING_TOKENS
    rfx_claude_response = rfx_tool_calls * CLAUDE_RESPONSE_TOKENS
    rfx_total_tokens = (rfx_user_tokens + rfx_claude_reasoning +
                        rfx_tool_output_tokens + rfx_tool_overhead +
                        rfx_claude_response)

    # Estimate built-in tool usage
    complexity_multipliers = {'Simple': 1.5, 'Medium': 2.5, 'Complex': 5.0}
    complexity_groups = group_by_complexity([r for r in results if r['status'] == 'PASS'])

    builtin_tool_calls = 0
    for complexity, complexity_results in complexity_groups.items():
        multiplier = complexity_multipliers.get(complexity, 2.0)
        builtin_tool_calls += len(complexity_results) * multiplier

    builtin_output_chars = rfx_output_chars * 1.8
    builtin_tool_output = builtin_output_chars / CHARS_PER_TOKEN
    builtin_tool_overhead = builtin_tool_calls * TOOL_CALL_OVERHEAD

    # Built-in requires same user prompts, but MORE reasoning (multiple steps)
    builtin_user_tokens = rfx_tool_calls * USER_PROMPT_TOKENS  # Same prompts
    builtin_claude_reasoning = rfx_tool_calls * (CLAUDE_REASONING_TOKENS * 1.5)  # More explanation
    builtin_claude_response = rfx_tool_calls * (CLAUDE_RESPONSE_TOKENS * 1.2)  # Slightly more summary
    builtin_total_tokens = (builtin_user_tokens + builtin_claude_reasoning +
                           builtin_tool_output + builtin_tool_overhead +
                           builtin_claude_response)

    # Calculate savings
    total_savings = builtin_total_tokens - rfx_total_tokens
    savings_pct = (total_savings / builtin_total_tokens * 100) if builtin_total_tokens > 0 else 0

    # Print breakdown
    print("RFX Approach (Full Conversation):")
    print(f"  User Prompts:           {rfx_user_tokens:,.0f} tokens")
    print(f"  Claude Reasoning:       {rfx_claude_reasoning:,.0f} tokens")
    print(f"  Tool Output:            {rfx_tool_output_tokens:,.0f} tokens")
    print(f"  Tool Overhead:          {rfx_tool_overhead:,.0f} tokens")
    print(f"  Claude Response:        {rfx_claude_response:,.0f} tokens")
    print(f"  ─────────────────────────────────")
    print(f"  TOTAL:                  {rfx_total_tokens:,.0f} tokens")
    print()

    print("Built-in Tools (Full Conversation):")
    print(f"  User Prompts:           {builtin_user_tokens:,.0f} tokens")
    print(f"  Claude Reasoning:       {builtin_claude_reasoning:,.0f} tokens")
    print(f"  Tool Output:            {builtin_tool_output:,.0f} tokens")
    print(f"  Tool Overhead:          {builtin_tool_overhead:,.0f} tokens")
    print(f"  Claude Response:        {builtin_claude_response:,.0f} tokens")
    print(f"  ─────────────────────────────────")
    print(f"  TOTAL:                  {builtin_total_tokens:,.0f} tokens")
    print()

    print("Realistic Total Savings:")
    print(f"  Token Reduction:        {total_savings:,.0f} tokens ({savings_pct:.1f}%)")
    print(f"  Tool Calls Saved:       {builtin_tool_calls - rfx_tool_calls:.0f} calls")
    print()

    # Category breakdown
    print("Expected Savings by Query Type:")
    print("  Simple queries (grep-like):     10-20% total conversation tokens")
    print("  Medium queries (symbol search): 30-50% total conversation tokens")
    print("  Complex queries (multi-step):   50-70% total conversation tokens")
    print()

    print("Why you might see 'fairly even' results:")
    print("  • Conversation context accumulates (dominates on turn 5+)")
    print("  • Prompt caching reduces input tokens (masks differences)")
    print("  • Simple queries tested (built-in tools work well)")
    print("  • Try symbol-aware queries for biggest impact!")
    print()


def plot_results(results: List[Dict[str, Any]]) -> None:
    """Generate performance charts (requires matplotlib)."""
    try:
        import matplotlib.pyplot as plt
        import numpy as np
    except ImportError:
        print("WARNING: matplotlib not installed. Skipping chart generation.")
        print("Install with: pip install matplotlib")
        return

    # Duration by category
    category_results = group_by_category(results)
    categories = list(category_results.keys())
    avg_durations = [
        sum(r['duration_ms'] for r in category_results[cat] if r['status'] == 'PASS') /
        len([r for r in category_results[cat] if r['status'] == 'PASS'])
        for cat in categories
    ]

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 6))

    # Chart 1: Average duration by category
    ax1.barh(categories, avg_durations, color='skyblue')
    ax1.set_xlabel('Average Duration (ms)')
    ax1.set_title('Average Query Duration by Category')
    ax1.grid(axis='x', alpha=0.3)

    # Chart 2: Output size distribution
    output_sizes_kb = [r['output_size_bytes'] / 1024 for r in results if r['status'] == 'PASS']
    ax2.hist(output_sizes_kb, bins=20, color='lightgreen', edgecolor='black')
    ax2.set_xlabel('Output Size (KB)')
    ax2.set_ylabel('Number of Tests')
    ax2.set_title('Output Size Distribution')
    ax2.grid(axis='y', alpha=0.3)

    plt.tight_layout()
    output_path = Path(sys.argv[1]).parent / 'benchmark_charts.png'
    plt.savefig(output_path, dpi=150)
    print(f"Charts saved to: {output_path}")
    print()


def main():
    parser = argparse.ArgumentParser(description='Analyze Claude Code benchmark results')
    parser.add_argument('results_file', help='Path to benchmark results JSON file')
    parser.add_argument('--plot', action='store_true', help='Generate performance charts')
    parser.add_argument('--top', type=int, default=10, help='Number of top results to show (default: 10)')

    args = parser.parse_args()

    if not Path(args.results_file).exists():
        print(f"ERROR: Results file not found: {args.results_file}")
        sys.exit(1)

    # Load results
    results = load_results(args.results_file)

    if not results:
        print("ERROR: No results found in file")
        sys.exit(1)

    # Calculate and print statistics
    summary_stats = calculate_summary_stats(results)
    print_summary(summary_stats)

    category_results = group_by_category(results)
    print_category_breakdown(category_results)

    complexity_results = group_by_complexity(results)
    print_complexity_breakdown(complexity_results)

    print_slowest_tests(results, top_n=args.top)
    print_largest_outputs(results, top_n=args.top)

    estimate_token_savings(results)
    estimate_realistic_token_savings(results)

    # Generate charts if requested
    if args.plot:
        plot_results(results)


if __name__ == '__main__':
    main()
