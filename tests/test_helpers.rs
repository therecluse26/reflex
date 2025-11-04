//! Test Helper Functions for Corpus-Based Testing
//!
//! This module provides utilities for testing Reflex against the test corpus.

use reflex::{CacheManager, IndexConfig, Indexer, QueryEngine, QueryFilter, SearchResult, SymbolKind};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static CORPUS_PATH: OnceLock<PathBuf> = OnceLock::new();

/// Initialize and index the test corpus once
/// Returns the path to the indexed corpus
pub fn setup_corpus() -> &'static Path {
    CORPUS_PATH.get_or_init(|| {
        let corpus = PathBuf::from("tests/corpus");

        // Index the corpus
        let cache = CacheManager::new(&corpus);
        let indexer = Indexer::new(cache, IndexConfig::default());
        indexer.index(&corpus, false).expect("Failed to index corpus");

        corpus
    }).as_path()
}

/// Create a query engine for the corpus
pub fn query_engine() -> QueryEngine {
    let corpus = setup_corpus();
    let cache = CacheManager::new(corpus);
    QueryEngine::new(cache)
}

/// Execute a query on the corpus
pub fn query_corpus(pattern: &str, filter: QueryFilter) -> Vec<SearchResult> {
    query_engine()
        .search(pattern, filter)
        .expect("Query failed")
}

/// Assert that a symbol with the given name and kind was found
pub fn assert_symbol_found(results: &[SearchResult], name: &str, kind: SymbolKind) {
    assert!(
        results.iter().any(|r| {
            r.symbol.as_deref() == Some(name) && r.kind == kind
        }),
        "Expected to find symbol '{}' of kind {:?}, but it was not in results",
        name,
        kind
    );
}

/// Assert that results contain a file matching the path pattern
pub fn assert_file_match(results: &[SearchResult], path_contains: &str) {
    assert!(
        results.iter().any(|r| r.path.contains(path_contains)),
        "Expected to find result in file containing '{}', but no match found",
        path_contains
    );
}

/// Assert exact result count
pub fn assert_result_count(results: &[SearchResult], expected: usize) {
    assert_eq!(
        results.len(),
        expected,
        "Expected {} results, but got {}",
        expected,
        results.len()
    );
}

/// Assert result count is at least the given value
pub fn assert_result_count_at_least(results: &[SearchResult], min: usize) {
    assert!(
        results.len() >= min,
        "Expected at least {} results, but got {}",
        min,
        results.len()
    );
}

/// Assert result count is at most the given value
pub fn assert_result_count_at_most(results: &[SearchResult], max: usize) {
    assert!(
        results.len() <= max,
        "Expected at most {} results, but got {}",
        max,
        results.len()
    );
}

/// Assert all results are of the specified kind
pub fn assert_all_kind(results: &[SearchResult], kind: SymbolKind) {
    for result in results {
        assert_eq!(
            result.kind, kind,
            "Expected all results to be {:?}, but found {:?}",
            kind, result.kind
        );
    }
}

/// Assert all results are from files with the given language
pub fn assert_all_language(results: &[SearchResult], lang: reflex::Language) {
    for result in results {
        assert_eq!(
            result.lang, lang,
            "Expected all results from {:?}, but found {:?}",
            lang, result.lang
        );
    }
}

/// Assert results are sorted deterministically (by path, then line)
pub fn assert_sorted(results: &[SearchResult]) {
    for i in 0..results.len().saturating_sub(1) {
        let curr = &results[i];
        let next = &results[i + 1];

        assert!(
            curr.path < next.path ||
            (curr.path == next.path && curr.span.start_line <= next.span.start_line),
            "Results are not sorted correctly at index {}", i
        );
    }
}

/// Assert that the preview contains the pattern
pub fn assert_preview_contains(results: &[SearchResult], pattern: &str) {
    assert!(
        results.iter().any(|r| r.preview.contains(pattern)),
        "Expected at least one preview to contain '{}'",
        pattern
    );
}

/// Count results by kind
pub fn count_by_kind(results: &[SearchResult], kind: SymbolKind) -> usize {
    results.iter().filter(|r| r.kind == kind).count()
}

/// Count results by file pattern
pub fn count_by_file_pattern(results: &[SearchResult], pattern: &str) -> usize {
    results.iter().filter(|r| r.path.contains(pattern)).count()
}

/// Get all unique file paths from results
pub fn unique_files(results: &[SearchResult]) -> Vec<String> {
    use std::collections::HashSet;
    let mut files: HashSet<String> = results.iter().map(|r| r.path.clone()).collect();
    let mut vec: Vec<String> = files.drain().collect();
    vec.sort();
    vec
}

/// Assert no duplicates (same file and line)
pub fn assert_no_duplicates(results: &[SearchResult]) {
    use std::collections::HashSet;
    let mut seen = HashSet::new();

    for result in results {
        let key = (result.path.clone(), result.span.start_line);
        assert!(
            seen.insert(key.clone()),
            "Duplicate result found: {:?}",
            key
        );
    }
}
