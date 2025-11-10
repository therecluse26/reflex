# Quick Start: Testing RFX in Claude Code

This is a **5-minute guide** to start testing Reflex vs built-in tools.

## Option 1: Automated Benchmark (3 minutes - One Command!)

Test `rfx` performance on the Reflex codebase:

```bash
# 1. Index the codebase (if not already done)
cd /path/to/reflex
rfx index

# 2. Run automated tests (analysis included automatically!)
cd tests/claude_code_benchmarks
./run_benchmark.sh
```

**That's it!** The script automatically runs analysis and displays results.

**What you get:**
- 35 automated tests across 10 categories
- Performance metrics (duration, output size)
- Tool output token savings (44% typical)
- **Realistic total token savings (42% including conversation context)**
- Category breakdown
- Why real-world results might differ

---

## Option 2: Manual Test in Claude Code (5 minutes)

Test side-by-side in actual Claude Code conversations:

### Step 1: Open Two Claude Code Conversations

**Conversation A (Control):**
```
Prompt: "I forbid you from using rfx. Use ONLY Grep, Glob, and Read tools for all code searches."
```

**Conversation B (Experimental):**
```
Prompt: "Use rfx for code searches whenever possible. Prefer rfx over Grep/Glob when searching code."
```

### Step 2: Test a Simple Query

Send this to BOTH conversations:

```
Find the definition of the SearchResult struct.
```

### Step 3: Compare Results

**What to look for:**

| Metric | Conversation A (Built-in) | Conversation B (RFX) |
|--------|---------------------------|----------------------|
| Tool calls | Usually 2-3 (Grep → Read) | Usually 1 (rfx) |
| False positives | Often 2+ (usage sites, comments) | Usually 0 (definitions only) |
| Output size | Larger (full file reads) | Smaller (targeted results) |
| Time to answer | Longer (multiple steps) | Faster (single command) |

**Expected outcome:**
- RFX: 1 precise result (struct definition at line 138)
- Built-in: 3+ results (definition + usage + comments)

---

## What to Test Next

### Easy Tests (High RFX Advantage)

1. **"Find all enum definitions"**
   - Built-in: Grep "enum" → manual filter → many false positives
   - RFX: `rfx query "enum" --kind Enum` → exact results
   - Expected savings: 70-80%

2. **"Find all functions in the parsers module"**
   - Built-in: Glob → Grep → Read multiple files
   - RFX: `rfx query "fn" --glob "src/parsers/*.rs" --symbols`
   - Expected savings: 60-70%

3. **"List all files with TODO comments"**
   - Built-in: Grep "TODO" → extract file paths
   - RFX: `rfx query "TODO" --paths`
   - Expected savings: 65-75%

### Medium Tests (Moderate RFX Advantage)

4. **"Find all unwrap() calls in Rust code"**
   - Expected savings: 10-20% (simple full-text search)

5. **"Search for 'config' in src/ directory only"**
   - Expected savings: 30-40% (glob filtering)

### Hard Tests (Maximum RFX Advantage)

6. **"Show me all parser implementations and their structure"**
   - Built-in: 4+ tool calls (Glob → Read → Grep → Read)
   - RFX: 1 command
   - Expected savings: 75-85%

---

## Measuring Token Savings

### Quick Estimate

**Formula:** `tokens ≈ (output characters) / 4 + (tool calls × 50)`

**Example:**

*Built-in Approach:*
- Grep output: 1200 chars = 300 tokens
- Read output: 3200 chars = 800 tokens
- Tool calls: 2 × 50 = 100 tokens
- **Total: 1200 tokens**

*RFX Approach:*
- Output: 800 chars = 200 tokens
- Tool calls: 1 × 50 = 50 tokens
- **Total: 250 tokens**

**Savings: 79%**

### Precise Measurement

If you have access to Claude API logs:
1. Export conversation transcript
2. Check token usage in API response
3. Compare directly

---

## Understanding the Results

### When RFX Excels (60-90% savings)

✅ **Symbol-aware searches**
- "Find function X definition" → filters out call sites
- "Find all structs" → eliminates false positives

✅ **Multi-step workflows**
- "Understand feature X" → combines multiple queries
- "Explore module Y" → one command vs 4+ tool calls

✅ **Filtered searches**
- "Find X in directory Y" → built-in glob + filter
- "Find X in language Y" → automatic detection

### When Built-in Tools are Comparable (10-20% savings)

≈ **Simple full-text search**
- "Find TODO comments" → basic grep adequate

≈ **Single file operations**
- "Show me main.rs" → direct read faster

≈ **Very specific patterns**
- "Find exact string 'pub(crate)'" → grep works fine

---

## Real-World Example

See `example_manual_test.md` for a complete worked example:
- Transcript of both approaches
- Exact token counts
- Precision/recall metrics
- Why RFX won (64% token reduction)

---

## Full Testing Guide

For comprehensive testing methodology, see:
- **`TESTING_GUIDE.md`** - Complete manual testing protocol
- **`test_prompts.md`** - 45 test cases across 12 categories
- **`README.md`** - Overview and automated testing

---

## Quick Tips

1. **Start simple:** Test "Find X definition" queries first
2. **Compare side-by-side:** Use two conversations in parallel
3. **Track metrics:** Count tool calls and output size
4. **Test realistic tasks:** Use actual AI coding workflows
5. **Try your codebase:** Larger projects show bigger savings

---

## Expected Benchmark Results

After running `./run_benchmark.sh`, you should see:

```
========================================
BENCHMARK SUMMARY
========================================
Total Tests:        35
Passed:             35 (100%)
Average Duration:   4.2 ms
Average Output Size: 847 bytes

Estimated Token Savings: 48-65%
Tool Call Reduction:     58%
========================================
```

**Interpretation:**
- ✅ All tests passed (index is working correctly)
- ⚠️ Average <10ms is excellent (sub-100ms target)
- ✅ ~850 bytes per query is compact (structured output)
- ✅ 48-65% token savings is significant at scale

---

## Next Steps

1. ✅ Run `./run_benchmark.sh` to establish baseline
2. ✅ Test 3-5 manual prompts in Claude Code
3. ✅ Record metrics using the template in TESTING_GUIDE.md
4. ✅ Share findings (open GitHub issue with results)
5. ✅ Test on your own codebase for real-world validation

---

**Ready to test? Pick Option 1 (automated) or Option 2 (manual) above!**
