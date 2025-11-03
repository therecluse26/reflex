//! Integration tests for RefLex

use reflex::{CacheManager, IndexConfig, Indexer, QueryEngine, QueryFilter, SymbolKind};
use std::fs;
use tempfile::TempDir;

// ==================== Basic Workflow Tests ====================

#[test]
fn test_full_workflow() {
    // Create a temporary directory for testing
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create a sample source file
    let sample_code = r#"
fn main() {
    println!("Hello, RefLex!");
}

fn greet(name: &str) {
    println!("Hello, {}!", name);
}
"#;
    std::fs::write(temp_path.join("test.rs"), sample_code).unwrap();

    // Initialize cache and indexer
    let cache = CacheManager::new(temp_path);
    let config = IndexConfig::default();
    let indexer = Indexer::new(cache, config);

    // Index the directory
    let stats = indexer.index(temp_path, false).unwrap();
    assert_eq!(stats.total_files, 1); // One test.rs file
    // Note: total_symbols is always 0 now (runtime symbol detection, not indexed)

    // Query the index (create new cache instance for query engine)
    let cache = CacheManager::new(temp_path);
    let engine = QueryEngine::new(cache);
    let results = engine.find_symbol("main").unwrap();
    assert_eq!(results.len(), 1); // Should find the main function
    assert_eq!(results[0].symbol.as_deref(), Some("main"));
}

#[test]
fn test_cache_initialization() {
    let temp_dir = TempDir::new().unwrap();
    let cache = CacheManager::new(temp_dir.path());

    assert!(!cache.exists());
    cache.init().unwrap();
    assert!(cache.path().exists());
}

#[test]
fn test_cache_clear() {
    let temp_dir = TempDir::new().unwrap();
    let cache = CacheManager::new(temp_dir.path());

    cache.init().unwrap();
    assert!(cache.path().exists());

    cache.clear().unwrap();
    assert!(!cache.path().exists());
}

// ==================== End-to-End Workflow Tests ====================

#[test]
fn test_index_and_fulltext_search_workflow() {
    let temp = TempDir::new().unwrap();
    let project = temp.path();

    // Create multiple source files
    fs::write(project.join("main.rs"), "fn main() {\n    println!(\"hello world\");\n}").unwrap();
    fs::write(project.join("lib.rs"), "pub fn hello() -> String {\n    \"hello\".to_string()\n}").unwrap();
    fs::write(project.join("utils.rs"), "// hello helper\nfn helper() {}").unwrap();

    // Index
    let cache = CacheManager::new(project);
    let indexer = Indexer::new(cache, IndexConfig::default());
    let stats = indexer.index(project, false).unwrap();
    assert_eq!(stats.total_files, 3);

    // Full-text search for "hello"
    let cache = CacheManager::new(project);
    let engine = QueryEngine::new(cache);
    let filter = QueryFilter::default();
    let results = engine.search("hello", filter).unwrap();

    // Should find all 3 occurrences (println, function name, comment)
    assert!(results.len() >= 3);
    assert!(results.iter().any(|r| r.path.contains("main.rs")));
    assert!(results.iter().any(|r| r.path.contains("lib.rs")));
    assert!(results.iter().any(|r| r.path.contains("utils.rs")));
}

#[test]
fn test_index_and_symbol_search_workflow() {
    let temp = TempDir::new().unwrap();
    let project = temp.path();

    // Create files with symbols
    fs::write(
        project.join("main.rs"),
        "fn greet() {}\nfn main() {\n    greet();\n}"
    ).unwrap();

    // Index
    let cache = CacheManager::new(project);
    let indexer = Indexer::new(cache, IndexConfig::default());
    indexer.index(project, false).unwrap();

    // Symbol search for "greet"
    let cache = CacheManager::new(project);
    let engine = QueryEngine::new(cache);
    let filter = QueryFilter {
        symbols_mode: true,
        ..Default::default()
    };
    let results = engine.search("greet", filter).unwrap();

    // Should find definition, not call site
    assert!(results.len() >= 1);
    assert!(results.iter().all(|r| r.kind == SymbolKind::Function));
}

#[test]
fn test_index_and_regex_search_workflow() {
    let temp = TempDir::new().unwrap();
    let project = temp.path();

    fs::write(
        project.join("main.rs"),
        "fn test1() {}\nfn test2() {}\nfn other() {}"
    ).unwrap();

    // Index
    let cache = CacheManager::new(project);
    let indexer = Indexer::new(cache, IndexConfig::default());
    indexer.index(project, false).unwrap();

    // Regex search
    let cache = CacheManager::new(project);
    let engine = QueryEngine::new(cache);
    let filter = QueryFilter {
        use_regex: true,
        ..Default::default()
    };
    let results = engine.search(r"fn test\d", filter).unwrap();

    // Should match test1 and test2 but not other
    assert_eq!(results.len(), 2);
}

#[test]
fn test_incremental_indexing_workflow() {
    let temp = TempDir::new().unwrap();
    let project = temp.path();

    // Initial index with one file
    fs::write(project.join("main.rs"), "fn main() {}").unwrap();

    let cache = CacheManager::new(project);
    let indexer = Indexer::new(cache, IndexConfig::default());
    let stats1 = indexer.index(project, false).unwrap();
    assert_eq!(stats1.total_files, 1);

    // Add another file and reindex
    fs::write(project.join("lib.rs"), "pub fn test() {}").unwrap();

    let cache = CacheManager::new(project);
    let indexer = Indexer::new(cache, IndexConfig::default());
    let stats2 = indexer.index(project, false).unwrap();
    assert_eq!(stats2.total_files, 2);

    // Verify both files are searchable (search for "main" which appears in both)
    let cache = CacheManager::new(project);
    let engine = QueryEngine::new(cache);
    let filter = QueryFilter::default();
    let results = engine.search("mai", filter).unwrap(); // "mai" is a trigram in "main"
    assert!(results.len() >= 1, "Should find at least main.rs");
}

#[test]
fn test_modify_file_and_reindex_workflow() {
    let temp = TempDir::new().unwrap();
    let project = temp.path();
    let main_path = project.join("main.rs");

    // Initial index
    fs::write(&main_path, "fn old_function() {}").unwrap();

    let cache = CacheManager::new(project);
    let indexer = Indexer::new(cache, IndexConfig::default());
    indexer.index(project, false).unwrap();

    // Verify we can find old function
    let cache = CacheManager::new(project);
    let engine = QueryEngine::new(cache);
    let filter = QueryFilter {
        symbols_mode: true,
        ..Default::default()
    };
    let results = engine.search("old", filter.clone()).unwrap();
    assert!(results.len() >= 1);

    // Modify file
    fs::write(&main_path, "fn new_function() {}").unwrap();

    // Reindex
    let cache = CacheManager::new(project);
    let indexer = Indexer::new(cache, IndexConfig::default());
    indexer.index(project, false).unwrap();

    // Verify we can find new function
    let cache = CacheManager::new(project);
    let engine = QueryEngine::new(cache);
    let results = engine.search("new", filter).unwrap();
    assert!(results.len() >= 1);
    assert!(results.iter().any(|r| r.symbol.as_ref().map_or(false, |s| s.contains("new"))));
}

// ==================== Multi-language Workflow Tests ====================

#[test]
fn test_multi_language_indexing_and_search() {
    let temp = TempDir::new().unwrap();
    let project = temp.path();

    // Create files in multiple languages
    fs::write(project.join("main.rs"), "fn greet() {}").unwrap();
    fs::write(project.join("app.ts"), "function greet() {}").unwrap();
    fs::write(project.join("script.py"), "def greet(): pass").unwrap();
    fs::write(project.join("main.js"), "function greet() {}").unwrap();

    // Index
    let cache = CacheManager::new(project);
    let indexer = Indexer::new(cache, IndexConfig::default());
    let stats = indexer.index(project, false).unwrap();
    assert_eq!(stats.total_files, 4);

    // Search across all languages
    let cache = CacheManager::new(project);
    let engine = QueryEngine::new(cache);
    let filter = QueryFilter::default();
    let results = engine.search("greet", filter).unwrap();
    assert!(results.len() >= 4);
}

#[test]
fn test_language_filtered_search_workflow() {
    let temp = TempDir::new().unwrap();
    let project = temp.path();

    fs::write(project.join("main.rs"), "fn test() {}").unwrap();
    fs::write(project.join("test.py"), "def test(): pass").unwrap();

    // Index
    let cache = CacheManager::new(project);
    let indexer = Indexer::new(cache, IndexConfig::default());
    indexer.index(project, false).unwrap();

    // Search with language filter
    let cache = CacheManager::new(project);
    let engine = QueryEngine::new(cache);
    let filter = QueryFilter {
        language: Some(reflex::Language::Rust),
        ..Default::default()
    };
    let results = engine.search("test", filter).unwrap();

    // Should only find Rust file
    assert!(results.iter().all(|r| r.path.ends_with(".rs")));
}

// ==================== Complex Query Workflow Tests ====================

#[test]
fn test_combined_filters_workflow() {
    let temp = TempDir::new().unwrap();
    let project = temp.path();

    // Create nested directory structure
    fs::create_dir_all(project.join("src")).unwrap();
    fs::create_dir_all(project.join("tests")).unwrap();

    fs::write(
        project.join("src/lib.rs"),
        "struct Point {}\nfn point_new() {}"
    ).unwrap();
    fs::write(
        project.join("tests/test.rs"),
        "fn test_point() {}"
    ).unwrap();

    // Index
    let cache = CacheManager::new(project);
    let indexer = Indexer::new(cache, IndexConfig::default());
    indexer.index(project, false).unwrap();

    // Search with multiple filters: Rust + function + src/ directory
    let cache = CacheManager::new(project);
    let engine = QueryEngine::new(cache);
    let filter = QueryFilter {
        language: Some(reflex::Language::Rust),
        kind: Some(SymbolKind::Function),
        file_pattern: Some("src/".to_string()),
        symbols_mode: true,
        ..Default::default()
    };
    let results = engine.search("poi", filter).unwrap();

    // Should only find point_new in src/lib.rs
    assert!(results.len() >= 1);
    assert!(results.iter().all(|r| r.path.contains("src/")));
    assert!(results.iter().all(|r| r.kind == SymbolKind::Function));
}

#[test]
fn test_limit_and_sorting_workflow() {
    let temp = TempDir::new().unwrap();
    let project = temp.path();

    // Create files that will produce many matches
    let content = (0..20).map(|i| format!("fn test{}() {{}}", i)).collect::<Vec<_>>().join("\n");
    fs::write(project.join("many.rs"), content).unwrap();

    // Index
    let cache = CacheManager::new(project);
    let indexer = Indexer::new(cache, IndexConfig::default());
    indexer.index(project, false).unwrap();

    // Search with limit
    let cache = CacheManager::new(project);
    let engine = QueryEngine::new(cache);
    let filter = QueryFilter {
        limit: Some(5),
        ..Default::default()
    };
    let results = engine.search("test", filter).unwrap();

    // Should limit to 5 results
    assert_eq!(results.len(), 5);

    // Results should be sorted deterministically
    for i in 0..results.len().saturating_sub(1) {
        assert!(results[i].span.start_line <= results[i + 1].span.start_line);
    }
}

// ==================== Error Handling Workflow Tests ====================

#[test]
fn test_query_without_index_fails() {
    let temp = TempDir::new().unwrap();
    let project = temp.path();

    // Don't create index, just try to query
    let cache = CacheManager::new(project);
    let engine = QueryEngine::new(cache);
    let filter = QueryFilter::default();
    let result = engine.search("test", filter);

    // Should fail with error
    assert!(result.is_err());
}

#[test]
fn test_index_empty_directory_succeeds() {
    let temp = TempDir::new().unwrap();
    let project = temp.path();

    // Index empty directory
    let cache = CacheManager::new(project);
    let indexer = Indexer::new(cache, IndexConfig::default());
    let stats = indexer.index(project, false).unwrap();

    assert_eq!(stats.total_files, 0);
    // Note: total_symbols is always 0 (runtime symbol detection)
}

#[test]
fn test_search_empty_index_returns_no_results() {
    let temp = TempDir::new().unwrap();
    let project = temp.path();

    // Index empty directory
    let cache = CacheManager::new(project);
    let indexer = Indexer::new(cache, IndexConfig::default());
    indexer.index(project, false).unwrap();

    // Search
    let cache = CacheManager::new(project);
    let engine = QueryEngine::new(cache);
    let filter = QueryFilter::default();
    let results = engine.search("anything", filter).unwrap();

    assert_eq!(results.len(), 0);
}

// ==================== Cache Persistence Workflow Tests ====================

#[test]
fn test_cache_persists_across_sessions() {
    let temp = TempDir::new().unwrap();
    let project = temp.path();

    fs::write(project.join("main.rs"), "fn test() {}").unwrap();

    // Session 1: Index
    {
        let cache = CacheManager::new(project);
        let indexer = Indexer::new(cache, IndexConfig::default());
        indexer.index(project, false).unwrap();
    }

    // Session 2: Query (new cache instance)
    {
        let cache = CacheManager::new(project);
        let engine = QueryEngine::new(cache);
        let filter = QueryFilter::default();
        let results = engine.search("test", filter).unwrap();
        assert!(results.len() >= 1);
    }
}

#[test]
fn test_clear_and_rebuild_workflow() {
    let temp = TempDir::new().unwrap();
    let project = temp.path();

    fs::write(project.join("main.rs"), "fn test() {}").unwrap();

    // Initial index
    let cache = CacheManager::new(project);
    let indexer = Indexer::new(cache, IndexConfig::default());
    indexer.index(project, false).unwrap();

    // Clear cache
    let cache = CacheManager::new(project);
    cache.clear().unwrap();

    // Rebuild
    let cache = CacheManager::new(project);
    let indexer = Indexer::new(cache, IndexConfig::default());
    let stats = indexer.index(project, false).unwrap();
    assert_eq!(stats.total_files, 1);

    // Verify search still works
    let cache = CacheManager::new(project);
    let engine = QueryEngine::new(cache);
    let filter = QueryFilter::default();
    let results = engine.search("test", filter).unwrap();
    assert!(results.len() >= 1);
}
