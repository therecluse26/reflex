# Symbol Cache Schema Migration Status

## Goal
Migrate from text-based file_path keys to integer file_id keys with junction table for fast symbol kind filtering.

## Progress

### ✅ Completed
1. **CacheManager (src/cache.rs)**
   - files table already has `id INTEGER PRIMARY KEY AUTOINCREMENT`
   - Added `get_file_id(path) -> Option<i64>`
   - Added `batch_get_file_ids(paths) -> HashMap<String, i64>`

2. **SymbolCache Schema (src/symbol_cache.rs)**
   - Updated init_schema() to create new tables:
     - symbols: uses file_id instead of file_path
     - symbol_kinds: junction table (file_id, kind) with index on kind
   - Updated extract_symbol_kinds() to return Vec<String>
   - Auto-migration: drops old tables if detected

### ⏳ In Progress
3. **SymbolCache Methods (src/symbol_cache.rs)**
   Need to update all methods to use file_ids:

   - [ ] `get(file_path, hash)` → `get(file_id, hash)`
   - [ ] `batch_get([(path, hash)])` → `batch_get([(file_id, hash)])`
   - [ ] **`batch_get_with_kind()`** ← CRITICAL for performance
   - [ ] `set(path, hash, symbols)` → `set(file_id, hash, symbols)` + junction table insert
   - [ ] `batch_set()` → update to use file_ids + junction table inserts
   - [ ] `stats()` - update queries
   - [ ] `cleanup_stale()` - update query

### 🔜 Pending
4. **QueryEngine (src/query.rs)**
   - Lookup file_ids before calling symbol_cache methods
   - Pass file_ids instead of paths to batch_get_with_kind()

5. **BackgroundIndexer (src/background_indexer.rs)**
   - Lookup file_ids before caching symbols
   - Pass file_ids to batch_set()

6. **Tests**
   - Update all symbol_cache tests to use file_ids

## New Query (Fast Path)

```sql
-- OLD (slow - 2.8s on Kubernetes):
SELECT file_path, symbols_json FROM symbols
WHERE (file_path, file_hash) IN (...)
  AND EXISTS (SELECT 1 FROM json_each(symbol_kinds) WHERE value = 'Struct')

-- NEW (fast - expected ~50-100ms):
SELECT s.symbols_json, f.path
FROM symbols s
JOIN symbol_kinds sk ON s.file_id = sk.file_id
JOIN files f ON s.file_id = f.id
WHERE s.file_id IN (?, ?, ...)     -- Integer IDs from trigram search
  AND sk.kind = 'Struct'           -- Index scan on kind column!
  AND s.file_hash = f.hash         -- Ensure cache valid
```

## Performance Expected

**Kubernetes "stream" --kind struct:**
- Current: 2,808ms (json_each on 310 rows)
- Expected: **~50-100ms** (index scan + integer joins)
- **Improvement: 30-50x faster**

**Benefits:**
- ✅ Index scan on `kind` column (O(log N))
- ✅ Integer joins 10x faster than text joins
- ✅ Only retrieves files with matching kind
- ✅ No JSON parsing at query time
- ✅ Foreign keys with CASCADE DELETE (automatic cleanup)

## Next Steps

1. **Complete batch_get_with_kind()** implementation
2. Update other SymbolCache methods
3. Update QueryEngine to lookup file_ids
4. Update BackgroundIndexer to use file_ids
5. Test on Kubernetes (expect ~50-100ms)
6. Test on Linux kernel

## Breaking Changes

**Acceptable** - No users yet, can delete .reflex/ and re-index.

All symbol_cache method signatures changed from file_path to file_id.
