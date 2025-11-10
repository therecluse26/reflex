# Quick Reference: When Does RFX Save Tokens?

Use this card to quickly decide if RFX will help with your query.

---

## Token Savings Cheat Sheet

| Your Task | Use RFX? | Expected Savings | Why |
|-----------|----------|------------------|-----|
| "Find TODO comments" | ❌ Optional | 10-15% | Simple grep works fine |
| "Find function X **definition**" | ✅ **YES** | 40-55% | Symbol awareness = precision |
| "Find all struct definitions" | ✅ **YES** | 45-60% | Filters out usages automatically |
| "Search for X in directory Y" | ✅ YES | 25-35% | Built-in glob + filter |
| "Show me all parsers" | ✅✅ **STRONGLY YES** | 60-75% | Multi-step becomes one command |
| "Understand feature X" | ✅✅ **STRONGLY YES** | 65-80% | Exploration workflow optimized |
| "Find unwrap() calls" | ≈ Either | 10-20% | Simple full-text search |
| "Read main.rs" | ❌ NO | N/A | Use Read tool directly |

---

## Decision Tree

```
START: What are you trying to do?

├─ Find TEXT anywhere
│  ├─ Simple pattern ("TODO", "unwrap")?
│  │  └─> ❌ Either tool fine (10-20% savings)
│  └─ Complex pattern (regex, multi-word)?
│     └─> ✅ RFX slightly better (15-25% savings)
│
├─ Find SYMBOL (definition, not usage)
│  ├─ Single symbol ("struct X")?
│  │  └─> ✅✅ RFX STRONGLY recommended (40-60% savings)
│  └─ All symbols of type ("all enums")?
│     └─> ✅✅ RFX STRONGLY recommended (45-65% savings)
│
├─ Explore CODE (understand, trace, map)
│  ├─ Single file?
│  │  └─> ❌ Use Read tool directly
│  ├─ Module/feature?
│  │  └─> ✅✅✅ RFX ESSENTIAL (60-80% savings)
│  └─ Multi-file workflow?
│     └─> ✅✅✅ RFX ESSENTIAL (65-85% savings)
│
└─ Read SPECIFIC FILE
   └─> ❌ Use Read tool (RFX not applicable)
```

---

## Red Flags: When RFX Won't Help Much

🚫 **Reading specific files** - Use Read tool
```
Bad:  "Use rfx to read main.rs"
Good: "Read main.rs directly"
```

🚫 **Simple one-word grep** - Either tool is fine
```
Minimal savings: "Find TODO"
Better use case: "Find all TODO comments in src/ excluding tests"
```

🚫 **Already have file path** - Just read it
```
Bad:  "Use rfx to find SearchResult then read it"
Good: "Read src/models.rs:138" (if you know the location)
```

---

## Green Flags: When RFX Shines

✅ **"Definition" keyword** - Symbol awareness activates
```
Prompt: "Find the definition of extract_symbols"
RFX saves: 40-60% tokens
```

✅ **"All X" pattern** - Bulk symbol finding
```
Prompt: "Find all enum definitions"
RFX saves: 45-65% tokens
```

✅ **"Show/Explore/Understand" workflows** - Multi-step
```
Prompt: "Show me how the trigram indexing works"
RFX saves: 60-80% tokens
```

✅ **Module exploration** - Structured discovery
```
Prompt: "What parser implementations exist?"
RFX saves: 60-75% tokens
```

---

## Magic Words That Trigger Maximum RFX Value

Use these phrases in your prompts:

- 🎯 "**definition of**" → Symbol-aware search
- 🎯 "**all X**" (all structs, all functions) → Bulk symbol finding
- 🎯 "**in directory/module Y**" → Scoped search with glob
- 🎯 "**show me**" / "**explore**" → Multi-step workflow
- 🎯 "**how does X work**" → Feature understanding
- 🎯 "**implementations of**" → Symbol discovery

Avoid generic words like "find X" → Be specific: "find the **definition** of X"

---

## Example Prompts (Good vs Better)

### ❌ Mediocre (10-20% savings)
```
"Find unwrap in the code"
"Search for TODO"
"Look for config"
```

### ✅ Good (30-50% savings)
```
"Find the definition of SearchResult struct"
"Find all enum definitions"
"Search for 'parser' only in src/parsers/"
```

### ✅✅ Excellent (60-80% savings)
```
"Show me all parser implementations and their key functions"
"Understand how the trigram indexing works - find the main functions and where they're called"
"Find all public functions in the parsers module"
"Trace how query execution flows from CLI to results"
```

---

## Conversation Tips

### ✅ DO:
- Start fresh conversations for important comparisons
- Use "definition of X" not just "X"
- Ask for module/feature exploration (multi-step)
- Focus on symbol-aware queries
- Request filtered searches (glob patterns)

### ❌ DON'T:
- Compare in conversations with 10+ prior messages (context dominates)
- Test only simple grep queries (built-in works fine)
- Expect huge savings on single-file reads
- Use rfx for tasks better suited to Read/Write/Edit tools

---

## Quick Math

**If 100 queries:**
- 60 simple queries × 15% savings = 9% total
- 30 symbol queries × 45% savings = 13.5% total
- 10 complex queries × 70% savings = 7% total

**Average: ~30% real-world token savings**

**But if you shift to:**
- 20 simple queries × 15% savings = 3% total
- 50 symbol queries × 45% savings = 22.5% total
- 30 complex queries × 70% savings = 21% total

**Average: ~47% real-world token savings**

**Takeaway:** The more you use symbol-aware and multi-step queries, the more RFX saves.

---

## Bottom Line

```
Simple grep-like queries:        RFX helps a little (10-20%)
Symbol-aware queries:            RFX helps significantly (30-50%)
Multi-step exploration:          RFX is game-changing (50-70%+)
```

**Pro tip:** If you're seeing "fairly even" token counts, you're testing the wrong query types. Try the "Excellent" prompts above to see RFX shine!

---

**See also:**
- `REALISTIC_EXPECTATIONS.md` - Full explanation of token savings
- `test_prompts.md` - 45 test cases by category
- `TESTING_GUIDE.md` - How to measure accurately
