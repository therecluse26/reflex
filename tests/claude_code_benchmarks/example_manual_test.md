# Example Manual Test: Symbol Definition Lookup

This is a complete example of testing `rfx` vs built-in tools in Claude Code.

## Test Scenario

**Task:** "Find the definition of the SearchResult struct"

**Expected Behavior:**
- Should return ONE result (the struct definition in src/models.rs)
- Should NOT return usage sites, imports, or comments mentioning SearchResult
- Should include exact line number and context

---

## Approach 1: Built-in Tools (Grep + Read)

### Conversation A Transcript

**User:**
```
I forbid you from using rfx. Use ONLY Grep, Glob, and Read tools.

Find the definition of the SearchResult struct.
```

**Claude Code Response:**
```
I'll search for the SearchResult struct definition using Grep.

[Tool Call 1: Grep]
- pattern: "struct SearchResult"
- type: "rust"
- output_mode: "content"

[Results: 3 matches found]
1. src/models.rs:138 - pub struct SearchResult {
2. tests/integration_test.rs:45 - // Test SearchResult deserialization
3. src/query.rs:234 - let result = SearchResult::new(

Let me read the main definition to confirm:

[Tool Call 2: Read]
- file_path: /home/brad/Code/personal/reflex/src/models.rs
- offset: 130
- limit: 30

[Results: Full struct definition shown with 30 lines of context]
```

**Metrics:**
- Tool Calls: 2 (Grep + Read)
- Output Tokens: ~500 (3 grep results + 30 lines of Read)
- Accuracy: ✅ Found definition, but with 2 false positives
- Precision: 33% (1 relevant / 3 results)
- Recall: 100% (found the definition)

---

## Approach 2: RFX (Symbol-Aware)

### Conversation B Transcript

**User:**
```
Use rfx for code searches.

Find the definition of the SearchResult struct.
```

**Claude Code Response:**
```
I'll search for the SearchResult struct definition using rfx.

[Tool Call 1: Bash]
- command: rfx query "SearchResult" --kind Struct --json

[Results: 1 match found]
{
  "status": "fresh",
  "can_trust_results": true,
  "pagination": {
    "total": 1,
    "count": 1,
    "offset": 0,
    "has_more": false
  },
  "results": [
    {
      "path": "src/models.rs",
      "kind": "Struct",
      "symbol": "SearchResult",
      "span": {
        "start_line": 138,
        "end_line": 156
      },
      "preview": "/// A search result representing a symbol or code location\n#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct SearchResult {\n    /// Absolute or relative path to the file\n    pub path: String,\n    /// Detected programming language (internal use only, not serialized to save tokens)\n    #[serde(skip)]\n    pub lang: Language,\n    /// Type of symbol found (only included for symbol searches, not text matches)\n    #[serde(skip_serializing_if = \"is_unknown_kind\")]\n    pub kind: SymbolKind,"
    }
  ]
}

The SearchResult struct is defined in src/models.rs at line 138.
```

**Metrics:**
- Tool Calls: 1 (rfx query)
- Output Tokens: ~180 (structured JSON with single result)
- Accuracy: ✅ Found definition only, no false positives
- Precision: 100% (1 relevant / 1 result)
- Recall: 100% (found the definition)

---

## Comparison

| Metric | Built-in Tools | RFX | Improvement |
|--------|---------------|-----|-------------|
| **Tool Calls** | 2 | 1 | 50% fewer |
| **Output Tokens** | ~500 | ~180 | 64% reduction |
| **Precision** | 33% | 100% | 3x better |
| **False Positives** | 2 | 0 | Eliminated |
| **Structured Output** | No | Yes (JSON) | ✅ |
| **Context Shown** | 30 lines | 19 lines | More concise |

**Winner:** RFX

**Why RFX Excels Here:**
1. Symbol-awareness filters out usage sites automatically
2. `--kind Struct` eliminates false positives (comments, strings)
3. Structured JSON output is easier to parse programmatically
4. Single tool call vs multiple steps
5. Fewer tokens = lower cost for AI interactions

---

## Testing This Yourself

### Step 1: Open Two Claude Code Conversations

**Conversation A:**
```
Initial prompt: "I forbid you from using rfx. Use ONLY Grep, Glob, and Read tools."
```

**Conversation B:**
```
Initial prompt: "Use rfx for code searches whenever possible."
```

### Step 2: Run the Test Prompt

In BOTH conversations, send:
```
Find the definition of the SearchResult struct.
```

### Step 3: Record Metrics

Use this template:

```
Test: Symbol Definition Lookup (SearchResult)
Date: 2025-01-15

Conversation A (Built-in):
- Tool calls: [count each Grep/Glob/Read]
- Tools used: [list them]
- Results returned: [total matches]
- Relevant results: [how many were actually the definition]
- Precision: [relevant / total]
- Output estimate: [characters / 4 = tokens]
- Notes: [observations]

Conversation B (RFX):
- Tool calls: [count rfx commands]
- Commands: [list them]
- Results returned: [total matches]
- Relevant results: [how many were actually the definition]
- Precision: [relevant / total]
- Output estimate: [characters / 4 = tokens]
- Notes: [observations]

Comparison:
- Token savings: [%]
- Precision improvement: [Y/N]
- Easier to use: [Y/N]
```

### Step 4: Try More Tests

After validating this example works, try these progressively harder tests:

1. **Simple:** "Find all TODO comments" (test_prompts.md #1.2)
2. **Medium:** "Find all enum definitions" (test_prompts.md #2.5)
3. **Complex:** "Show me all parser implementations" (test_prompts.md #6.3)

---

## Expected Results Pattern

You should see RFX provide the most benefit when:

1. **Symbol-awareness matters** (definitions vs usages)
   - Example: "Find function X" → RFX returns definition only
   - Built-in: Returns definition + all call sites → manual filtering needed

2. **Multiple steps required** (Grep → Read → Grep → Read)
   - Example: "Explore module structure"
   - Built-in: 4+ tool calls
   - RFX: 1-2 commands

3. **Filtering by language or type**
   - Example: "Find Python classes"
   - Built-in: Glob *.py → Grep "class" → manual check
   - RFX: `rfx query "class" --lang python --kind Class`

Conversely, built-in tools are comparable for:
- Simple one-word searches ("find TODO")
- Reading specific known files ("show me main.rs")
- Very small result sets (1-2 files)

---

## Pro Tips

### 1. Ask Claude Code to Compare

Prompt:
```
"I want to test two approaches:
1. First, solve this using ONLY Grep and Read
2. Then, solve the same task using rfx
3. Compare the approaches and tell me which was more efficient"
```

### 2. Test on Your Own Codebase

The Reflex repository is small (1.5K files). Test on larger codebases:

```bash
cd /path/to/your/project
rfx index
# Now use the same test prompts in Claude Code
```

### 3. Track Actual Token Counts

If you have access to Claude API logs:
- Export conversation transcripts
- Count actual tokens used
- Compare against estimates

### 4. Focus on Real Workflows

Don't just test isolated queries. Test realistic multi-step tasks:
- "I need to add error handling here - show me similar patterns"
- "Where is feature X implemented and how does it work?"
- "Find all places where this function is called"

---

**This example demonstrates the testing methodology. See `test_prompts.md` for 44 more test cases!**
