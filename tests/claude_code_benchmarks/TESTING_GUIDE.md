# Claude Code Testing Guide

This guide explains how to test Reflex (`rfx`) vs Claude Code's built-in tools (Grep, Glob, Read) to measure efficacy, accuracy, and token usage.

## Table of Contents

1. [Quick Start](#quick-start)
2. [Testing Methodologies](#testing-methodologies)
3. [Automated Benchmark Suite](#automated-benchmark-suite)
4. [Manual Conversation Testing](#manual-conversation-testing)
5. [Metrics to Track](#metrics-to-track)
6. [Interpreting Results](#interpreting-results)
7. [Example Test Scenarios](#example-test-scenarios)

---

## Quick Start

### Prerequisites

1. **Build and install reflex:**
   ```bash
   cd /path/to/reflex
   cargo build --release
   cargo install --path .
   ```

2. **Index the reflex codebase:**
   ```bash
   rfx index
   ```

3. **Run the automated benchmark:**
   ```bash
   cd tests/claude_code_benchmarks
   ./run_benchmark.sh
   ```

4. **Analyze results:**
   ```bash
   python3 analyze_results.py results/benchmark_TIMESTAMP.json
   ```

---

## Testing Methodologies

### Method 1: Parallel Conversation Testing (Most Accurate)

Create **two separate Claude Code conversations** with identical prompts:

**Conversation A: Built-in tools only (control)**
```
Initial prompt: "I forbid you from using rfx or any external code search tools.
Use ONLY Grep, Glob, and Read tools for all searches."

Then run test prompts from test_prompts.md
```

**Conversation B: RFX-enabled (experimental)**
```
Initial prompt: "Use rfx for code searches whenever possible. Only fall back
to Grep/Glob if rfx cannot solve the task."

Then run same test prompts
```

**How to track:**
- Keep both conversations open side-by-side
- For each prompt, run in both conversations
- Note: token count (visible in UI), number of tool calls, accuracy
- Export conversation logs if available

**Benefits:**
- Isolates tool choice as only variable
- Most objective comparison
- Real-world token usage

---

### Method 2: Single Conversation A/B Testing

In **one conversation**, explicitly test both approaches:

**Template:**
```
Prompt: "I want to test two approaches for finding X.

First, find all X using ONLY Grep, Glob, and Read tools. Show me each tool
call and the results.

Then, solve the same task using rfx. Show the command and results.

Finally, compare:
1. Which was more accurate? (precision and recall)
2. Which used fewer tool calls?
3. Which returned more concise results?
4. Which would be faster for an AI agent?"
```

**Example:**
```
User: "I want to test two approaches for finding all struct definitions.

First, find all struct definitions using ONLY Grep and Read.
Then, do the same using rfx.
Compare the approaches."

Claude Code will execute both and compare.
```

**Benefits:**
- Single conversation
- Direct side-by-side comparison
- Good for quick spot-checks

---

### Method 3: Automated Benchmark Suite

Use the provided scripts to run standardized tests:

```bash
# Run all benchmark tests
./run_benchmark.sh

# Run specific category
./run_benchmark.sh --category 2  # Symbol-aware search

# Run specific test
./run_benchmark.sh --test 2.1    # Find function definition

# Analyze results
python3 analyze_results.py results/benchmark_TIMESTAMP.json

# Generate charts
python3 analyze_results.py results/benchmark_TIMESTAMP.json --plot
```

**Benefits:**
- Reproducible
- Quantitative metrics
- Performance measurements
- Automated analysis

**Note:** This only tests `rfx` performance, not built-in tools. Use this to establish `rfx` baseline, then compare manually in conversations.

---

## Metrics to Track

### 1. **Efficacy** (Can it solve the task?)

**Score each approach:**
- ✅ **Complete**: Solves task fully in one query
- ⚠️ **Partial**: Requires multiple queries/tools
- ❌ **Failed**: Cannot solve accurately

**Example:**
- Task: "Find the definition of extract_symbols function"
- Built-in: ⚠️ Partial (Grep → filter manually → Read file)
- RFX: ✅ Complete (`rfx query "extract_symbols" --symbols`)

---

### 2. **Accuracy** (Precision & Recall)

**Precision:** % of results that are relevant
```
Precision = (Relevant Results Returned) / (Total Results Returned)
```

**Recall:** % of relevant items found
```
Recall = (Relevant Results Returned) / (All Relevant Items)
```

**Example:**
- Task: "Find all struct definitions"
- Total structs in codebase: 50
- Built-in Grep returns: 80 results (includes "destructure", comments)
  - Relevant: 45/80 (Precision: 56%)
  - Missed: 5 structs (Recall: 90%)
- RFX returns: 50 results (all struct definitions)
  - Relevant: 50/50 (Precision: 100%)
  - Missed: 0 structs (Recall: 100%)

---

### 3. **Token Count**

**Track these components:**

| Component | How to Measure |
|-----------|----------------|
| **Tool Calls** | Count number of Grep/Glob/Read/RFX calls |
| **Input Tokens** | Sum of tokens in all tool parameters |
| **Output Tokens** | Sum of tokens returned by all tools |
| **Total Tokens** | Input + Output |

**Estimation Formula:**
```
1 token ≈ 4 characters (rough average for code)
Tool call overhead ≈ 50 tokens per call
```

**Example Calculation:**

*Built-in approach:*
```
Task: Find all functions in parsers module

Step 1: Glob src/parsers/*.rs
  - Input: 20 tokens
  - Output: 100 tokens (14 file paths)

Step 2: Grep "pub fn" on each file
  - Input: 30 tokens
  - Output: 800 tokens (many results, includes calls)

Step 3: Read 3 files to verify
  - Input: 45 tokens
  - Output: 2000 tokens (full file contents)

Total: 2995 tokens, 3 tool calls
```

*RFX approach:*
```
Task: Find all functions in parsers module

Step 1: rfx query "pub fn" --glob "src/parsers/*.rs" --symbols --json
  - Input: 60 tokens (command)
  - Output: 600 tokens (only function definitions, structured)

Total: 660 tokens, 1 tool call
Savings: 78% tokens, 66% fewer tool calls
```

---

### 4. **Response Time** (for automated tests)

The benchmark script measures:
- Query execution time (milliseconds)
- Output size (bytes)
- Number of results returned

**Performance targets:**
- Simple queries: <10ms
- Medium queries: <50ms
- Complex queries: <200ms

---

## Interpreting Results

### When RFX Excels

**1. Symbol-Aware Searches (70-90% token savings)**
- Finding definitions vs usages
- Filtering by symbol type (struct, enum, function)
- Scoped searches (specific modules/files)

**Example:** "Find all enum definitions"
- Built-in: Grep → manual filtering → false positives
- RFX: `rfx query "enum" --kind Enum` → exact results

---

**2. Multi-Step Workflows (50-80% token savings)**
- Understanding feature implementation
- Tracing code paths
- Exploring module structure

**Example:** "Understand how trigram indexing works"
- Built-in: 5+ tool calls (Glob → Read → Grep → Read → Read)
- RFX: 2 commands (find code + find functions)

---

**3. Large Codebases**
- RFX trigram index enables sub-100ms searches on 10k+ files
- Built-in tools may timeout or hit limits

---

### When Built-in Tools are Comparable

**1. Simple Text Search (10-20% savings)**
- Single-word patterns
- No symbol filtering needed
- Small result sets

**Example:** "Find all TODO comments"
- Built-in: Grep "TODO" → adequate
- RFX: `rfx query "TODO"` → similar results

---

**2. One-Off File Reads**
- Reading specific known files
- No search required

**Example:** "Show me src/main.rs"
- Built-in: Read src/main.rs → direct
- RFX: Not applicable

---

### Token Savings by Category

Based on benchmark analysis:

| Category | Expected RFX Savings | Why |
|----------|---------------------|-----|
| **Symbol-aware search** | 70-90% | Eliminates manual filtering, reduces false positives |
| **Multi-step workflows** | 50-80% | Combines multiple built-in tool calls into one |
| **Glob filtering** | 30-50% | Built-in file filtering via trigram index |
| **Simple text search** | 10-20% | Comparable results, slight overhead reduction |
| **Language filtering** | 40-60% | Automatic language detection vs manual glob patterns |
| **Paths-only mode** | 60-70% | Returns paths only, eliminates content overhead |

---

## Example Test Scenarios

### Scenario 1: Symbol Definition Lookup

**Task:** "Find where the SearchResult struct is defined"

**Built-in Approach:**
```
Claude Code:
1. Grep pattern: "struct SearchResult"
   → Returns: 3 results (definition + usage in comments)
2. Read src/models.rs to verify line 138

Result: 2 tool calls, ~300 tokens
```

**RFX Approach:**
```
Claude Code:
1. rfx query "SearchResult" --kind Struct --json
   → Returns: 1 result (exact definition, line 138)

Result: 1 tool call, ~80 tokens
Token savings: 73%
```

---

### Scenario 2: Feature Exploration

**Task:** "Show me all parser implementations and their structure"

**Built-in Approach:**
```
Claude Code:
1. Glob "src/parsers/*.rs"
   → Returns: 14 files
2. Read src/parsers/rust.rs (example parser)
   → Returns: full file (~300 lines)
3. Read src/parsers/mod.rs (registry)
   → Returns: full file (~150 lines)
4. Grep "pub fn parse" to find pattern
   → Returns: 20+ results across files

Result: 4 tool calls, ~3000 tokens
```

**RFX Approach:**
```
Claude Code:
1. rfx query "pub fn parse" --glob "src/parsers/*.rs" --symbols --json
   → Returns: 14 function definitions with signatures

Result: 1 tool call, ~500 tokens
Token savings: 83%
```

---

### Scenario 3: Codebase-Wide Search

**Task:** "Find all uses of unwrap() in Rust code"

**Built-in Approach:**
```
Claude Code:
1. Grep pattern: "unwrap()", type: "rust"
   → Returns: 50+ results with context

Result: 1 tool call, ~1200 tokens
```

**RFX Approach:**
```
Claude Code:
1. rfx query "unwrap()" --lang rust --json
   → Returns: 50+ results with context

Result: 1 tool call, ~1100 tokens
Token savings: 8% (minimal for simple full-text search)
```

---

## Running Your Own Tests

### Step-by-Step Test Protocol

1. **Choose a test prompt** from `test_prompts.md`

2. **Open two Claude Code conversations:**
   - Conversation A: Restrict to built-in tools
   - Conversation B: Allow rfx usage

3. **Run the same prompt in both conversations**

4. **Record metrics:**
   ```
   Test: [Test ID and name]
   Date: [YYYY-MM-DD]

   Built-in Approach:
   - Tool calls: [count]
   - Tools used: [Grep, Glob, Read, etc.]
   - Output tokens: [estimate]
   - Accuracy: [Precision %, Recall %]
   - Time: [if measurable]

   RFX Approach:
   - Tool calls: [count]
   - Commands: [rfx commands used]
   - Output tokens: [estimate]
   - Accuracy: [Precision %, Recall %]
   - Time: [if measurable]

   Comparison:
   - Token savings: [%]
   - Tool call reduction: [count]
   - Accuracy improvement: [Y/N]
   - Notes: [observations]
   ```

5. **Repeat for 10-20 tests** across different categories

6. **Analyze trends:**
   - Which categories show biggest savings?
   - Where is RFX most valuable?
   - Are there cases where built-in tools are better?

---

## Tips for Accurate Testing

### 1. **Control for AI Variability**
- Use identical prompts in both conversations
- Test at similar times (model behavior can vary)
- Run multiple trials for complex queries

### 2. **Measure Token Usage**
- Use character count as proxy: `tokens ≈ chars / 4`
- Include tool call overhead (~50 tokens per call)
- Count both input and output tokens

### 3. **Verify Accuracy**
- Cross-check results against known ground truth
- Manually verify symbol definitions
- Check for false positives/negatives

### 4. **Consider Real-World Workflows**
- Test realistic AI coding assistant tasks
- Include multi-step workflows
- Consider follow-up questions

---

## Automated Testing Script Usage

### Basic Usage

```bash
# Run all tests
./run_benchmark.sh

# Run and analyze
./run_benchmark.sh && \
  python3 analyze_results.py results/benchmark_$(ls -t results/ | head -1)
```

### Custom Test Runs

Edit `run_benchmark.sh` to add your own tests:

```bash
run_test "X.Y" "Your Test Name" \
    "rfx query 'pattern' --flags --json" \
    "Your Category" "Simple|Medium|Complex"
```

### Analyzing Results

```bash
# Basic analysis
python3 analyze_results.py results/benchmark_TIMESTAMP.json

# With charts (requires matplotlib)
python3 analyze_results.py results/benchmark_TIMESTAMP.json --plot

# Show top 20 slowest tests
python3 analyze_results.py results/benchmark_TIMESTAMP.json --top 20
```

---

## Sharing Results

### Reporting Format

When sharing benchmark results, include:

1. **Environment:**
   - Reflex version: `rfx --version`
   - Codebase size: `rfx stats`
   - Test date and duration

2. **Summary Statistics:**
   - Total tests run
   - Success rate
   - Average duration
   - Estimated token savings

3. **Category Breakdown:**
   - Performance by test category
   - Token savings by category
   - Accuracy notes

4. **Notable Findings:**
   - Best-case scenarios for RFX
   - Cases where built-in tools performed better
   - Unexpected results

### Example Report

```
Reflex vs Built-in Tools Benchmark Report
==========================================

Environment:
- Reflex version: 0.2.7
- Codebase: Reflex repository (1,514 files, 28,859 lines)
- Test date: 2025-01-15
- Tests run: 35

Summary:
- Success rate: 100%
- Avg query duration: 4.2ms
- Estimated token savings: 62%
- Tool call reduction: 58%

Top Performers:
1. Symbol-aware search: 85% token savings
2. Multi-step workflows: 73% token savings
3. Paths-only mode: 68% token savings

Comparable Performance:
- Simple full-text search: 12% token savings
- Single file reads: Not applicable
```

---

## Frequently Asked Questions

### Q: How accurate are the token estimates?

**A:** Token estimates use the formula `tokens ≈ chars / 4`, which is conservative for code. Real token counts may vary by ±20% depending on tokenizer. For precise measurements, use the Claude API token counter if available.

### Q: Should I test on my own codebase?

**A:** Yes! The Reflex repository is relatively small. Testing on larger codebases (10k+ files) will show even greater performance benefits. Index your project with `rfx index` and run the same test prompts.

### Q: How do I handle failed tests?

**A:** Failed tests indicate either:
1. Index is stale (run `rfx index` to rebuild)
2. Query syntax error (check command format)
3. Expected behavior (e.g., no results for invalid pattern)

Review the error message and expected behavior from `test_prompts.md`.

### Q: Can I compare RFX to other search tools (ag, ripgrep)?

**A:** Yes! The test prompts work for any search tool. However, note that `rfx` is optimized for AI coding workflows with structured JSON output and symbol-awareness, not just grep-style text search.

### Q: How do I test in Claude Code specifically?

**A:**
1. Open Claude Code in your terminal or IDE
2. Ensure `rfx` is in your PATH
3. Run test prompts as normal conversation messages
4. Claude Code will automatically use rfx if it's available and appropriate
5. Track tool calls in the conversation UI

---

## Next Steps

1. **Run the automated benchmark** to establish baseline RFX performance
2. **Test 5-10 prompts manually** in parallel Claude Code conversations
3. **Record metrics** using the template above
4. **Analyze results** to identify where RFX provides most value
5. **Share findings** with the Reflex community

For questions or to share results, open an issue at:
https://github.com/reflex-search/reflex/issues

---

**Happy Testing!**
