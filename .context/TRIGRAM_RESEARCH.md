# Trigram Index Implementation Research

**Date:** 2025-10-31
**Status:** Architecture Design Complete, Ready for Implementation
**Related:** See CLAUDE.md for project vision, TODO.md for task breakdown

---

## Overview

RefLex is implementing **trigram-based full-text code search** modeled after Sourcegraph's Zoekt and Google Code Search.

**Goal:** Enable <100ms queries that find **every occurrence** of patterns across 10k+ files.

---

## What is Trigram Indexing?

### Core Concept

A **trigram** is a sequence of 3 consecutive characters:
- `"extract_symbols"` → trigrams: `["ext", "xtr", "tra", "rac", "act", "ct_", "t_s", "_sy", "sym", "ymb", "mbo", "bol", "ols"]`

### Inverted Index Structure

Build a mapping from each trigram to the locations where it appears:

```
Inverted Index:
"ext" → [(file1, line3), (file2, line15), (file5, line8)]
"xtr" → [(file1, line3), (file5, line8)]
"tra" → [(file1, line3), (file3, line42), (file5, line8)]
...
```

### Search Algorithm

When searching for `"extract_symbols"`:

1. **Extract trigrams** from query: `["ext", "xtr", "tra", ..., "ols"]`
2. **Lookup posting lists**: Get file/line locations for each trigram
3. **Intersect lists**: Find locations that contain ALL trigrams
4. **Verify match**: Check actual content at candidate locations
5. **Return results**: With surrounding context

**Performance:** Reduces search from thousands of files to ~10-100 candidates (100-1000x speedup).

---

## Why Trigrams Work

### Key Properties

1. **Small alphabet**: Only 256³ = 16M possible trigrams; most are rare
2. **Discriminative**: Long strings have unique trigram combinations
3. **Substring-friendly**: Any substring >3 chars has trigrams that must appear
4. **Regex-friendly**: Can extract guaranteed trigrams from many patterns
5. **Fast intersection**: Posting lists are small; intersections are quick

### Example

Search for `"extract_symbols"` (13 trigrams):
- Posting list intersection eliminates 99.9% of files
- Only verify matches in ~10 candidate files
- Total time: <10ms (vs ~100ms full scan)

---

## Implementation Details

### Data Structures

#### 1. Trigram Type
```rust
// Represent trigram as 3 bytes (compact)
pub type Trigram = [u8; 3];

// Or as u32 for faster hashing:
pub type Trigram = u32; // pack 3 bytes into 32-bit int
```

#### 2. File Location
```rust
pub struct FileLocation {
    file_id: u32,      // Index into file list
    line_no: u32,      // Line number (1-indexed)
    byte_offset: u32,  // Byte offset in file (for context extraction)
}
```

#### 3. Inverted Index
```rust
pub struct TrigramIndex {
    // Map trigram to sorted list of locations
    index: HashMap<Trigram, Vec<FileLocation>>,

    // File ID to file path mapping
    files: Vec<PathBuf>,
}
```

### Binary Format (trigrams.bin)

```
Header (32 bytes):
  magic: "RFTG" (4 bytes)
  version: 1 (u32)
  num_trigrams: N (u64)
  num_files: F (u64)
  index_offset: offset to trigram index (u64)
  reserved: 8 bytes

File List (variable):
  [F file paths, length-prefixed strings]

Trigram Index (variable):
  For each trigram:
    trigram: 3 bytes
    count: u32 (number of locations)
    locations: [count × FileLocation structs]
```

### Content Store (content.bin)

```
Header (32 bytes):
  magic: "RFCT" (4 bytes)
  version: 1 (u32)
  num_files: F (u64)
  index_offset: offset to file index (u64)
  reserved: 12 bytes

File Index:
  For each file:
    offset: u64 (byte offset to file content)
    length: u64 (file size in bytes)

File Contents:
  [Concatenated file contents]
```

**Design rationale:** Memory-map content.bin for zero-copy access to file contents.

---

## Trigram Extraction Algorithm

### Basic Extraction

```rust
fn extract_trigrams(text: &str) -> Vec<Trigram> {
    let bytes = text.as_bytes();
    let mut trigrams = Vec::new();

    for i in 0..bytes.len().saturating_sub(2) {
        let trigram = [bytes[i], bytes[i+1], bytes[i+2]];
        trigrams.push(trigram);
    }

    trigrams
}
```

### With Line/Offset Tracking

```rust
fn extract_trigrams_with_locations(text: &str, file_id: u32) -> Vec<(Trigram, FileLocation)> {
    let mut result = Vec::new();
    let bytes = text.as_bytes();

    let mut line_no = 1;
    let mut line_start = 0;

    for (i, &byte) in bytes.iter().enumerate() {
        // Track newlines
        if byte == b'\n' {
            line_no += 1;
            line_start = i + 1;
        }

        // Extract trigram
        if i + 2 < bytes.len() {
            let trigram = [bytes[i], bytes[i+1], bytes[i+2]];
            let location = FileLocation {
                file_id,
                line_no,
                byte_offset: i as u32,
            };
            result.push((trigram, location));
        }
    }

    result
}
```

---

## Query Processing

### Plain Text Query

```rust
fn search_plain_text(query: &str, index: &TrigramIndex) -> Vec<Match> {
    if query.len() < 3 {
        // Fall back to full scan for short queries
        return full_scan(query);
    }

    // Step 1: Extract trigrams from query
    let trigrams = extract_trigrams(query);

    // Step 2: Get posting lists for each trigram
    let mut posting_lists: Vec<&Vec<FileLocation>> = trigrams
        .iter()
        .filter_map(|t| index.index.get(t))
        .collect();

    if posting_lists.is_empty() {
        return vec![];
    }

    // Step 3: Sort by list size (smallest first for efficient intersection)
    posting_lists.sort_by_key(|list| list.len());

    // Step 4: Intersect posting lists
    let candidates = intersect_posting_lists(posting_lists);

    // Step 5: Verify actual matches
    let mut results = Vec::new();
    for loc in candidates {
        if verify_match_at_location(query, loc) {
            results.push(create_match_result(loc));
        }
    }

    results
}
```

### Regex Query

```rust
fn search_regex(pattern: &str, index: &TrigramIndex) -> Vec<Match> {
    // Step 1: Extract guaranteed trigrams from regex
    let trigrams = extract_trigrams_from_regex(pattern);

    if trigrams.is_empty() {
        // Regex has no literals → fall back to full scan
        return regex_full_scan(pattern);
    }

    // Step 2: Use trigrams to narrow candidates
    let candidates = search_by_trigrams(&trigrams, index);

    // Step 3: Verify with actual regex engine
    let regex = Regex::new(pattern).unwrap();
    let mut results = Vec::new();
    for loc in candidates {
        let content = get_file_content(loc.file_id);
        if regex.is_match(content) {
            results.push(create_match_result(loc));
        }
    }

    results
}
```

---

## Posting List Intersection

### Naive Intersection (O(n*m))
```rust
fn intersect_two_lists(a: &[FileLocation], b: &[FileLocation]) -> Vec<FileLocation> {
    a.iter()
        .filter(|loc| b.contains(loc))
        .cloned()
        .collect()
}
```

### Optimized Intersection (O(n+m))
```rust
fn intersect_sorted_lists(a: &[FileLocation], b: &[FileLocation]) -> Vec<FileLocation> {
    let mut result = Vec::new();
    let (mut i, mut j) = (0, 0);

    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            Ordering::Equal => {
                result.push(a[i]);
                i += 1;
                j += 1;
            }
            Ordering::Less => i += 1,
            Ordering::Greater => j += 1,
        }
    }

    result
}
```

**Optimization:** Store posting lists sorted by (file_id, line_no) for linear-time intersection.

---

## Regex Trigram Extraction

### Algorithm

Extract the longest literal substrings from the regex pattern.

### Examples

| Regex Pattern | Extracted Trigrams | Notes |
|---------------|-------------------|-------|
| `extract_symbols` | `["ext", "xtr", "tra", ...]` | All literal |
| `fn\s+extract` | `["fn ", "ext", "xtr", ...]` | Literals + space |
| `Google.*Search` | `["Goo", "oog", "gle", "Sea", "ear", "rch"]` | Both ends are literals |
| `a(bc)+d` | `["abc", "bcb", "bcd"]` | Repetition generates alternatives |
| `if\|else` | `["if ", "els", "lse"]` | Alternation → multiple options |
| `.*` | `[]` | No literals → full scan |

### Implementation Sketch

```rust
fn extract_trigrams_from_regex(pattern: &str) -> Vec<Trigram> {
    // This is simplified; real implementation needs regex parsing

    // Strategy:
    // 1. Parse regex AST
    // 2. Find all literal sequences
    // 3. Extract trigrams from literals
    // 4. Handle special cases (^, $, \b, etc.)

    // For MVP: extract longest contiguous literal substring
    extract_longest_literal(pattern)
        .and_then(|lit| Some(extract_trigrams(&lit)))
        .unwrap_or_default()
}
```

---

## Performance Characteristics

### Index Size

- **Trigram count**: ~20-30 trigrams per 100 characters of code
- **Posting list size**: Avg 10-100 locations per trigram
- **Total index size**: ~20% of source code size
- **Example**: 100MB source → 20MB trigram index

### Query Performance

| Query Type | Trigram Count | Candidates | Time |
|------------|---------------|------------|------|
| Long literal (`extract_symbols`) | 13 | ~10 files | <10ms |
| Regex with literals (`fn.*test`) | 3-5 | ~100 files | <20ms |
| Short pattern (`if`) | 0 | All files | ~100ms |
| Wildcard (`.*`) | 0 | All files | ~100ms |

### Space/Time Trade-offs

- **More trigrams indexed** → larger index, faster queries
- **Compressed posting lists** → smaller index, slower decompression
- **Memory-mapped I/O** → zero-copy, fast cold start

**Decision for RefLex:** Uncompressed posting lists for <100ms queries.

---

## References

1. **Russ Cox - Regular Expression Matching with a Trigram Index**
   - https://swtch.com/~rsc/regexp/regexp4.html
   - Describes Google Code Search implementation

2. **Zoekt - Sourcegraph's Code Search Engine**
   - https://github.com/sourcegraph/zoekt
   - Production trigram-based search

3. **PostgreSQL pg_trgm Module**
   - Uses trigrams for full-text search
   - Provides reference implementation

---

## Open Questions & TODOs

- [ ] Should we case-fold trigrams? (e.g., "Ext" → "ext")
  - **Recommendation:** No, keep case-sensitive for deterministic results

- [ ] How to handle Unicode?
  - **Recommendation:** UTF-8 bytes, trigrams can span character boundaries

- [ ] Should posting lists be compressed?
  - **Recommendation:** Not for MVP (optimize later if index too large)

- [ ] How to handle very long posting lists (common trigrams)?
  - **Recommendation:** Skip or truncate lists >10k entries (rare trigrams are more useful)

---

**END OF TRIGRAM_RESEARCH.md**
