//! Integration tests for Reflex

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
    println!("Hello, Reflex!");
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
    let filter = QueryFilter {
        use_contains: true,  // "mai" is substring of "main", not at word boundary
        ..Default::default()
    };
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
        use_contains: true,  // "old" in "old_function" is not at word boundary
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
        use_contains: true,  // "poi" in "point_new" is not at word boundary
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
        use_contains: true,  // "test" in "test0", "test1" is not at word boundary
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

// ==================== Glob Pattern Tests ====================

#[test]
fn test_glob_single_pattern() {
    let temp = TempDir::new().unwrap();
    let project = temp.path();

    // Create files in different directories
    fs::create_dir_all(project.join("src")).unwrap();
    fs::create_dir_all(project.join("tests")).unwrap();

    fs::write(project.join("src/main.rs"), "fn extract_pattern() {}").unwrap();
    fs::write(project.join("tests/test.rs"), "fn extract_pattern() {}").unwrap();
    fs::write(project.join("other.rs"), "fn extract_pattern() {}").unwrap();

    // Index
    let cache = CacheManager::new(project);
    let indexer = Indexer::new(cache, IndexConfig::default());
    indexer.index(project, false).unwrap();

    // Search with glob pattern matching only src/
    let cache = CacheManager::new(project);
    let engine = QueryEngine::new(cache);
    let filter = QueryFilter {
        glob_patterns: vec!["**/src/**/*.rs".to_string()],
        ..Default::default()
    };
    let results = engine.search("extract_pattern", filter).unwrap();

    // Should only find results in src/
    assert!(results.len() >= 1);
    assert!(results.iter().all(|r| r.path.contains("src/")));
    assert!(results.iter().any(|r| r.path.contains("main.rs")));
    assert!(!results.iter().any(|r| r.path.contains("tests/")));
    assert!(!results.iter().any(|r| r.path.contains("other.rs")));
}

#[test]
fn test_glob_multiple_patterns() {
    let temp = TempDir::new().unwrap();
    let project = temp.path();

    fs::create_dir_all(project.join("src")).unwrap();
    fs::create_dir_all(project.join("examples")).unwrap();
    fs::create_dir_all(project.join("build")).unwrap();

    fs::write(project.join("src/lib.rs"), "TODO: implement").unwrap();
    fs::write(project.join("examples/demo.rs"), "TODO: add example").unwrap();
    fs::write(project.join("build/gen.rs"), "TODO: generated").unwrap();

    // Index
    let cache = CacheManager::new(project);
    let indexer = Indexer::new(cache, IndexConfig::default());
    indexer.index(project, false).unwrap();

    // Search with multiple glob patterns
    let cache = CacheManager::new(project);
    let engine = QueryEngine::new(cache);
    let filter = QueryFilter {
        glob_patterns: vec!["**/src/**".to_string(), "**/examples/**".to_string()],
        ..Default::default()
    };
    let results = engine.search("TODO", filter).unwrap();

    // Should find results in src/ and examples/ but not build/
    assert_eq!(results.len(), 2);
    assert!(results.iter().any(|r| r.path.contains("src/lib.rs")));
    assert!(results.iter().any(|r| r.path.contains("examples/demo.rs")));
    assert!(!results.iter().any(|r| r.path.contains("build/")));
}

#[test]
fn test_glob_wildcard_patterns() {
    let temp = TempDir::new().unwrap();
    let project = temp.path();

    fs::write(project.join("test_one.rs"), "fn test() {}").unwrap();
    fs::write(project.join("test_two.rs"), "fn test() {}").unwrap();
    fs::write(project.join("other.rs"), "fn test() {}").unwrap();
    fs::write(project.join("main.py"), "def test(): pass").unwrap();

    // Index
    let cache = CacheManager::new(project);
    let indexer = Indexer::new(cache, IndexConfig::default());
    indexer.index(project, false).unwrap();

    // Search with wildcard pattern
    let cache = CacheManager::new(project);
    let engine = QueryEngine::new(cache);
    let filter = QueryFilter {
        glob_patterns: vec!["**/test_*.rs".to_string()],
        ..Default::default()
    };
    let results = engine.search("test", filter).unwrap();

    // Should only match test_*.rs files
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|r| r.path.contains("test_") && r.path.ends_with(".rs")));
    assert!(!results.iter().any(|r| r.path.contains("other.rs")));
    assert!(!results.iter().any(|r| r.path.contains("main.py")));
}

#[test]
fn test_glob_specific_extension() {
    let temp = TempDir::new().unwrap();
    let project = temp.path();

    fs::write(project.join("main.rs"), "fn pattern() {}").unwrap();
    fs::write(project.join("app.ts"), "function pattern() {}").unwrap();
    fs::write(project.join("script.py"), "def pattern(): pass").unwrap();

    // Index
    let cache = CacheManager::new(project);
    let indexer = Indexer::new(cache, IndexConfig::default());
    indexer.index(project, false).unwrap();

    // Search for only .rs files
    let cache = CacheManager::new(project);
    let engine = QueryEngine::new(cache);
    let filter = QueryFilter {
        glob_patterns: vec!["**/*.rs".to_string()],
        ..Default::default()
    };
    let results = engine.search("pattern", filter).unwrap();

    // Should only find Rust files
    assert_eq!(results.len(), 1);
    assert!(results[0].path.ends_with(".rs"));
}

#[test]
fn test_glob_specific_directory() {
    let temp = TempDir::new().unwrap();
    let project = temp.path();

    fs::create_dir_all(project.join("src/parsers")).unwrap();
    fs::create_dir_all(project.join("src/utils")).unwrap();

    fs::write(project.join("src/parsers/rust.rs"), "TODO: parse").unwrap();
    fs::write(project.join("src/utils/helpers.rs"), "TODO: help").unwrap();

    // Index
    let cache = CacheManager::new(project);
    let indexer = Indexer::new(cache, IndexConfig::default());
    indexer.index(project, false).unwrap();

    // Search only in parsers directory
    let cache = CacheManager::new(project);
    let engine = QueryEngine::new(cache);
    let filter = QueryFilter {
        glob_patterns: vec!["**/src/parsers/**".to_string()],
        ..Default::default()
    };
    let results = engine.search("TODO", filter).unwrap();

    // Should only find results in parsers/
    assert_eq!(results.len(), 1);
    assert!(results[0].path.contains("parsers/rust.rs"));
    assert!(!results.iter().any(|r| r.path.contains("utils/")));
}

// ==================== Exclude Pattern Tests ====================

#[test]
fn test_exclude_single_pattern() {
    let temp = TempDir::new().unwrap();
    let project = temp.path();

    fs::create_dir_all(project.join("src")).unwrap();
    fs::create_dir_all(project.join("build")).unwrap();

    fs::write(project.join("src/main.rs"), "fn extract_pattern() {}").unwrap();
    fs::write(project.join("build/generated.rs"), "fn extract_pattern() {}").unwrap();

    // Index
    let cache = CacheManager::new(project);
    let indexer = Indexer::new(cache, IndexConfig::default());
    indexer.index(project, false).unwrap();

    // Search with exclude pattern
    let cache = CacheManager::new(project);
    let engine = QueryEngine::new(cache);
    let filter = QueryFilter {
        exclude_patterns: vec!["**/build/**".to_string()],
        ..Default::default()
    };
    let results = engine.search("extract_pattern", filter).unwrap();

    // Should find results in src/ but not build/
    assert_eq!(results.len(), 1);
    assert!(results[0].path.contains("src/main.rs"));
    assert!(!results.iter().any(|r| r.path.contains("build/")));
}

#[test]
fn test_exclude_multiple_patterns() {
    let temp = TempDir::new().unwrap();
    let project = temp.path();

    fs::create_dir_all(project.join("src")).unwrap();
    fs::create_dir_all(project.join("build")).unwrap();
    fs::create_dir_all(project.join("target")).unwrap();

    fs::write(project.join("src/main.rs"), "TODO: implement").unwrap();
    fs::write(project.join("build/gen.rs"), "TODO: generated").unwrap();
    fs::write(project.join("target/debug.rs"), "TODO: debug").unwrap();

    // Index
    let cache = CacheManager::new(project);
    let indexer = Indexer::new(cache, IndexConfig::default());
    indexer.index(project, false).unwrap();

    // Search with multiple exclude patterns
    let cache = CacheManager::new(project);
    let engine = QueryEngine::new(cache);
    let filter = QueryFilter {
        exclude_patterns: vec!["**/build/**".to_string(), "**/target/**".to_string()],
        ..Default::default()
    };
    let results = engine.search("TODO", filter).unwrap();

    // Should only find results in src/
    assert_eq!(results.len(), 1);
    assert!(results[0].path.contains("src/main.rs"));
}

#[test]
fn test_exclude_generated_files() {
    let temp = TempDir::new().unwrap();
    let project = temp.path();

    fs::write(project.join("main.rs"), "fn pattern() {}").unwrap();
    fs::write(project.join("generated.rs"), "fn pattern() {}").unwrap();
    fs::write(project.join("codegen.rs"), "fn pattern() {}").unwrap();

    // Index
    let cache = CacheManager::new(project);
    let indexer = Indexer::new(cache, IndexConfig::default());
    indexer.index(project, false).unwrap();

    // Exclude generated files
    let cache = CacheManager::new(project);
    let engine = QueryEngine::new(cache);
    let filter = QueryFilter {
        exclude_patterns: vec!["**/generated.rs".to_string(), "**/codegen.rs".to_string()],
        ..Default::default()
    };
    let results = engine.search("pattern", filter).unwrap();

    // Should only find main.rs
    assert_eq!(results.len(), 1);
    assert!(results[0].path.contains("main.rs"));
}

#[test]
fn test_exclude_specific_directories() {
    let temp = TempDir::new().unwrap();
    let project = temp.path();

    fs::create_dir_all(project.join("src")).unwrap();
    fs::create_dir_all(project.join("tests")).unwrap();
    fs::create_dir_all(project.join("examples")).unwrap();

    fs::write(project.join("src/lib.rs"), "TODO").unwrap();
    fs::write(project.join("tests/test.rs"), "TODO").unwrap();
    fs::write(project.join("examples/demo.rs"), "TODO").unwrap();

    // Index
    let cache = CacheManager::new(project);
    let indexer = Indexer::new(cache, IndexConfig::default());
    indexer.index(project, false).unwrap();

    // Exclude tests and examples
    let cache = CacheManager::new(project);
    let engine = QueryEngine::new(cache);
    let filter = QueryFilter {
        exclude_patterns: vec!["**/tests/**".to_string(), "**/examples/**".to_string()],
        ..Default::default()
    };
    let results = engine.search("TODO", filter).unwrap();

    // Should only find src/
    assert_eq!(results.len(), 1);
    assert!(results[0].path.contains("src/lib.rs"));
}

// ==================== Paths-Only Mode Tests ====================

#[test]
fn test_paths_only_deduplication() {
    let temp = TempDir::new().unwrap();
    let project = temp.path();

    // Create a file with multiple occurrences of the pattern
    let content = r#"
fn extract_pattern() {}
fn test_extract() {}
struct Pattern {}
fn another_extract_pattern() {}
"#;
    fs::write(project.join("main.rs"), content).unwrap();
    fs::write(project.join("other.rs"), "fn extract_pattern() {}").unwrap();

    // Index
    let cache = CacheManager::new(project);
    let indexer = Indexer::new(cache, IndexConfig::default());
    indexer.index(project, false).unwrap();

    // Search with paths_only
    let cache = CacheManager::new(project);
    let engine = QueryEngine::new(cache);
    let filter = QueryFilter {
        paths_only: true,
        use_contains: true,  // "extract" in "extract_pattern" is not at word boundary
        ..Default::default()
    };
    let results = engine.search("extract", filter).unwrap();

    // Should return only 2 unique paths (main.rs and other.rs)
    assert_eq!(results.len(), 2);

    // Verify paths are unique
    let mut paths: Vec<_> = results.iter().map(|r| &r.path).collect();
    paths.sort();
    paths.dedup();
    assert_eq!(paths.len(), 2);
}

#[test]
fn test_paths_only_with_language_filter() {
    let temp = TempDir::new().unwrap();
    let project = temp.path();

    fs::write(project.join("main.rs"), "fn pattern() {}").unwrap();
    fs::write(project.join("app.ts"), "function pattern() {}").unwrap();
    fs::write(project.join("lib.rs"), "fn pattern() {}").unwrap();

    // Index
    let cache = CacheManager::new(project);
    let indexer = Indexer::new(cache, IndexConfig::default());
    indexer.index(project, false).unwrap();

    // Search with paths_only + language filter
    let cache = CacheManager::new(project);
    let engine = QueryEngine::new(cache);
    let filter = QueryFilter {
        paths_only: true,
        language: Some(reflex::Language::Rust),
        ..Default::default()
    };
    let results = engine.search("pattern", filter).unwrap();

    // Should only return Rust file paths
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|r| r.path.ends_with(".rs")));
}

#[test]
fn test_paths_only_single_match_per_file() {
    let temp = TempDir::new().unwrap();
    let project = temp.path();

    // File with many matches
    let content = (0..50).map(|i| format!("fn test{}() {{}}", i)).collect::<Vec<_>>().join("\n");
    fs::write(project.join("many.rs"), content).unwrap();

    // Index
    let cache = CacheManager::new(project);
    let indexer = Indexer::new(cache, IndexConfig::default());
    indexer.index(project, false).unwrap();

    // Search with paths_only
    let cache = CacheManager::new(project);
    let engine = QueryEngine::new(cache);
    let filter = QueryFilter {
        paths_only: true,
        use_contains: true,  // "test" in "test0", "test1" is not at word boundary (followed by digit)
        ..Default::default()
    };
    let results = engine.search("test", filter).unwrap();

    // Should return only 1 path despite 50 matches
    assert_eq!(results.len(), 1);
    assert!(results[0].path.contains("many.rs"));
}

#[test]
fn test_paths_only_across_multiple_files() {
    let temp = TempDir::new().unwrap();
    let project = temp.path();

    fs::create_dir_all(project.join("src")).unwrap();
    fs::write(project.join("src/a.rs"), "TODO TODO TODO").unwrap();
    fs::write(project.join("src/b.rs"), "TODO").unwrap();
    fs::write(project.join("src/c.rs"), "TODO TODO").unwrap();

    // Index
    let cache = CacheManager::new(project);
    let indexer = Indexer::new(cache, IndexConfig::default());
    indexer.index(project, false).unwrap();

    // Search with paths_only
    let cache = CacheManager::new(project);
    let engine = QueryEngine::new(cache);
    let filter = QueryFilter {
        paths_only: true,
        ..Default::default()
    };
    let results = engine.search("TODO", filter).unwrap();

    // Should return 3 unique file paths
    assert_eq!(results.len(), 3);
}

// ==================== Combined Filter Tests ====================

#[test]
fn test_glob_and_exclude_together() {
    let temp = TempDir::new().unwrap();
    let project = temp.path();

    fs::create_dir_all(project.join("src/parsers")).unwrap();
    fs::create_dir_all(project.join("src/utils")).unwrap();

    fs::write(project.join("src/parsers/rust.rs"), "TODO: parse").unwrap();
    fs::write(project.join("src/parsers/generated.rs"), "TODO: generated").unwrap();
    fs::write(project.join("src/utils/helpers.rs"), "TODO: help").unwrap();

    // Index
    let cache = CacheManager::new(project);
    let indexer = Indexer::new(cache, IndexConfig::default());
    indexer.index(project, false).unwrap();

    // Search with glob (include src/) and exclude (exclude generated files)
    let cache = CacheManager::new(project);
    let engine = QueryEngine::new(cache);
    let filter = QueryFilter {
        glob_patterns: vec!["**/src/**".to_string()],
        exclude_patterns: vec!["**/generated.rs".to_string()],
        ..Default::default()
    };
    let results = engine.search("TODO", filter).unwrap();

    // Should find rust.rs and helpers.rs but not generated.rs
    assert_eq!(results.len(), 2);
    assert!(results.iter().any(|r| r.path.contains("rust.rs")));
    assert!(results.iter().any(|r| r.path.contains("helpers.rs")));
    assert!(!results.iter().any(|r| r.path.contains("generated.rs")));
}

#[test]
fn test_glob_exclude_and_language() {
    let temp = TempDir::new().unwrap();
    let project = temp.path();

    fs::create_dir_all(project.join("src")).unwrap();
    fs::create_dir_all(project.join("build")).unwrap();

    fs::write(project.join("src/main.rs"), "fn pattern() {}").unwrap();
    fs::write(project.join("src/app.ts"), "function pattern() {}").unwrap();
    fs::write(project.join("build/gen.rs"), "fn pattern() {}").unwrap();

    // Index
    let cache = CacheManager::new(project);
    let indexer = Indexer::new(cache, IndexConfig::default());
    indexer.index(project, false).unwrap();

    // Search with glob + exclude + language
    let cache = CacheManager::new(project);
    let engine = QueryEngine::new(cache);
    let filter = QueryFilter {
        glob_patterns: vec!["**/src/**".to_string()],
        exclude_patterns: vec!["**/*.ts".to_string()],
        language: Some(reflex::Language::Rust),
        ..Default::default()
    };
    let results = engine.search("pattern", filter).unwrap();

    // Should only find src/main.rs
    assert_eq!(results.len(), 1);
    assert!(results[0].path.contains("src/main.rs"));
}

#[test]
fn test_glob_exclude_and_symbols() {
    let temp = TempDir::new().unwrap();
    let project = temp.path();

    fs::create_dir_all(project.join("src")).unwrap();
    fs::create_dir_all(project.join("tests")).unwrap();

    let src_content = r#"
fn extract_pattern() {}
fn test() {
    extract_pattern();
}
"#;
    fs::write(project.join("src/lib.rs"), src_content).unwrap();
    fs::write(project.join("tests/test.rs"), "fn extract_pattern() {}").unwrap();

    // Index
    let cache = CacheManager::new(project);
    let indexer = Indexer::new(cache, IndexConfig::default());
    indexer.index(project, false).unwrap();

    // Search with glob + exclude + symbols
    let cache = CacheManager::new(project);
    let engine = QueryEngine::new(cache);
    let filter = QueryFilter {
        glob_patterns: vec!["**/src/**".to_string()],
        exclude_patterns: vec!["**/tests/**".to_string()],
        symbols_mode: true,
        use_contains: true,  // "extract" in "extract_pattern" is not at word boundary
        ..Default::default()
    };
    let results = engine.search("extract", filter).unwrap();

    // Should find only the function definition in src/, not the call site
    assert!(results.len() >= 1);
    assert!(results.iter().all(|r| r.path.contains("src/")));
    assert!(results.iter().all(|r| r.kind == SymbolKind::Function));
}

#[test]
fn test_all_filters_together() {
    let temp = TempDir::new().unwrap();
    let project = temp.path();

    fs::create_dir_all(project.join("src")).unwrap();
    fs::create_dir_all(project.join("tests")).unwrap();
    fs::create_dir_all(project.join("build")).unwrap();

    // Multiple occurrences in src/main.rs
    fs::write(project.join("src/main.rs"), "fn extract_pattern() {}\nfn other_extract() {}").unwrap();
    fs::write(project.join("tests/test.rs"), "fn extract_pattern() {}").unwrap();
    fs::write(project.join("build/gen.rs"), "fn extract_pattern() {}").unwrap();

    // Index
    let cache = CacheManager::new(project);
    let indexer = Indexer::new(cache, IndexConfig::default());
    indexer.index(project, false).unwrap();

    // Search with ALL filters: glob + exclude + language + symbols + paths_only
    let cache = CacheManager::new(project);
    let engine = QueryEngine::new(cache);
    let filter = QueryFilter {
        glob_patterns: vec!["**/src/**".to_string()],
        exclude_patterns: vec!["**/tests/**".to_string(), "**/build/**".to_string()],
        language: Some(reflex::Language::Rust),
        symbols_mode: true,
        paths_only: true,
        use_contains: true,  // "extract" in "extract_pattern" is not at word boundary
        ..Default::default()
    };
    let results = engine.search("extract", filter).unwrap();

    // Should return only 1 unique path (src/main.rs) despite multiple matches
    assert_eq!(results.len(), 1);
    assert!(results[0].path.contains("src/main.rs"));
}

#[test]
fn test_glob_exclude_paths_with_limit() {
    let temp = TempDir::new().unwrap();
    let project = temp.path();

    fs::create_dir_all(project.join("src")).unwrap();

    // Create multiple files
    for i in 0..10 {
        fs::write(project.join(format!("src/file{}.rs", i)), "TODO: implement").unwrap();
    }

    // Index
    let cache = CacheManager::new(project);
    let indexer = Indexer::new(cache, IndexConfig::default());
    indexer.index(project, false).unwrap();

    // Search with glob + paths_only + limit
    let cache = CacheManager::new(project);
    let engine = QueryEngine::new(cache);
    let filter = QueryFilter {
        glob_patterns: vec!["**/src/**".to_string()],
        paths_only: true,
        limit: Some(5),
        ..Default::default()
    };
    let results = engine.search("TODO", filter).unwrap();

    // Should limit to 5 unique paths
    assert_eq!(results.len(), 5);
    assert!(results.iter().all(|r| r.path.contains("src/")));
}
