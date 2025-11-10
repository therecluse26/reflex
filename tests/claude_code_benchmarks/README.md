# Claude Code Benchmark Suite

This directory contains a comprehensive testing framework for evaluating Reflex (`rfx`) vs Claude Code's built-in tools (Grep, Glob, Read).

## Contents

- **`test_prompts.md`** - 45 curated test prompts across 12 categories
- **`run_benchmark.sh`** - Automated test harness for running `rfx` benchmarks
- **`analyze_results.py`** - Python script for analyzing benchmark results
- **`TESTING_GUIDE.md`** - Comprehensive guide for manual and automated testing
- **`results/`** - Directory for storing benchmark results (created on first run)

## Quick Start

### 1. Run One-Shot Benchmark (Automatic Analysis)

```bash
# Ensure reflex is indexed
cd /path/to/reflex
rfx index

# Run benchmark (tests + analysis in one command)
cd tests/claude_code_benchmarks
./run_benchmark.sh
```

The script will:
1. Run 35 automated tests
2. Save results to JSON file
3. **Automatically analyze results** (no manual step!)
4. Display full report including realistic token savings

### 2. Manual Analysis (Optional)

If you want to re-analyze results or generate charts:

```bash
# Re-analyze existing results
python3 analyze_results.py results/benchmark_TIMESTAMP.json

# Generate performance charts (requires matplotlib)
python3 analyze_results.py results/benchmark_TIMESTAMP.json --plot
```

### 3. Manual Testing in Claude Code

See **`TESTING_GUIDE.md`** for detailed instructions on:
- Parallel conversation testing
- A/B comparison in single conversations
- Metrics to track
- Interpreting results

## Test Categories

1. **Simple Text Search** (5 tests) - Basic full-text pattern matching
2. **Symbol-Aware Search** (7 tests) - Finding definitions vs usages
3. **Glob Pattern Filtering** (5 tests) - Directory/file scoping
4. **Language-Specific Filtering** (3 tests) - Multi-language support
5. **Paths-Only Mode** (3 tests) - File path extraction
6. **Complex Multi-Step Workflows** (5 tests) - Real-world AI agent tasks
7. **Regex Pattern Matching** (3 tests) - Pattern-based searches
8. **Performance & Scale** (3 tests) - Large result set handling
9. **Edge Cases & Precision** (4 tests) - Accuracy validation
10. **Context Quality** (2 tests) - Result preview quality
11. **Attribute/Annotation Discovery** (2 tests) - Specialized symbol types
12. **Real-World AI Agent Scenarios** (3 tests) - Onboarding and exploration

**Total: 45 tests**

## Expected Results

Based on Reflex architecture, expected token savings:

| Scenario Type | Expected RFX Savings | Why |
|--------------|---------------------|-----|
| Symbol-aware search | 70-90% | Eliminates manual filtering |
| Multi-step workflows | 50-80% | Combines multiple tool calls |
| Glob filtering | 30-50% | Built-in file filtering |
| Simple text search | 10-20% | Comparable to Grep |
| Language filtering | 40-60% | Automatic detection |
| Paths-only mode | 60-70% | Returns paths only |

## Key Metrics

The benchmark tracks:
- **Query duration** (milliseconds)
- **Output size** (bytes and lines)
- **Success rate** (pass/fail)
- **Token usage** (estimated)
- **Tool call count** (for manual testing)

## Example Output

```
========================================
  Reflex Claude Code Benchmark Suite
========================================

Running Test 1.1: Find Function Occurrences
Command: rfx query 'extract_symbols' --json
  Status: PASS
  Duration: 3ms
  Output Size: 1,247 bytes (11 lines)

Running Test 2.1: Find Function Definition
Command: rfx query 'extract_symbols' --symbols --json
  Status: PASS
  Duration: 5ms
  Output Size: 156 bytes (1 line)

...

========================================
  Benchmark Complete!
========================================

Results saved to: results/benchmark_20250115_143022.json

Next steps:
  1. Review results: cat results/benchmark_20250115_143022.json | jq .
  2. Analyze results: python3 analyze_results.py results/benchmark_20250115_143022.json
  3. Compare with built-in tools using the testing guide
```

## Analysis Example

```
========================================
BENCHMARK SUMMARY
========================================
Total Tests:        35
Passed:             35 (100.0%)
Failed:             0

Average Duration:   4.2 ms
Min Duration:       2.0 ms
Max Duration:       18.5 ms

Average Output Size: 847 bytes
Total Output Size:   29,645 bytes
========================================

========================================
ESTIMATED TOKEN IMPACT
========================================

RFX Approach:
  Tool Calls:       35
  Output Tokens:    7,411
  Overhead Tokens:  1,750
  Total Tokens:     9,161

Built-in Tools (Estimated):
  Tool Calls:       88
  Output Tokens:    13,340
  Overhead Tokens:  4,400
  Total Tokens:     17,740

Estimated Savings:
  Token Reduction:  8,579 tokens (48.4%)
  Tool Call Reduction: 53 calls

Note: These are conservative estimates. Actual savings may be higher,
especially for complex multi-step workflows and symbol-aware searches.
========================================
```

## Files Generated

After running the benchmark:

```
tests/claude_code_benchmarks/
├── results/
│   ├── benchmark_20250115_143022.json    # Test results
│   └── benchmark_charts.png               # Performance charts (with --plot)
├── test_prompts.md                        # Test library
├── run_benchmark.sh                       # Test runner
├── analyze_results.py                     # Analysis tool
├── TESTING_GUIDE.md                       # Testing instructions
└── README.md                              # This file
```

## Contributing

To add new test cases:

1. Add test definition to `test_prompts.md`
2. Add test execution to `run_benchmark.sh`
3. Update category counts in this README
4. Run benchmark to validate

## Requirements

### For Automated Benchmark:
- Bash (Linux/macOS/WSL)
- Reflex (`rfx`) installed and in PATH
- Python 3.6+ for analysis

### For Charts (Optional):
- `matplotlib`: `pip install matplotlib`

### For Manual Testing:
- Claude Code (terminal or IDE)
- Access to tool call metrics in conversation UI

## Troubleshooting

### "rfx command not found"
```bash
# Ensure reflex is installed
cd /path/to/reflex
cargo install --path .

# Verify installation
which rfx
rfx --version
```

### "No reflex index found"
```bash
# Create index first
cd /path/to/reflex
rfx index
```

### "Permission denied"
```bash
# Make scripts executable
chmod +x run_benchmark.sh
chmod +x analyze_results.py
```

## Additional Resources

- **Reflex Documentation:** `/path/to/reflex/README.md`
- **Reflex Architecture:** `/path/to/reflex/ARCHITECTURE.md`
- **Project Instructions:** `/path/to/reflex/CLAUDE.md`

## Support

For questions or issues:
1. Check `TESTING_GUIDE.md` for detailed instructions
2. Review `test_prompts.md` for test definitions
3. Open an issue at: https://github.com/reflex-search/reflex/issues

---

**Happy Testing!**
