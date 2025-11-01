//! Integration tests for RefLex

use reflex::{CacheManager, IndexConfig, Indexer, QueryEngine};
use tempfile::TempDir;

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
    let stats = indexer.index(temp_path).unwrap();
    assert_eq!(stats.total_files, 1); // One test.rs file
    assert_eq!(stats.total_symbols, 2); // main and greet functions

    // Query the index (create new cache instance for query engine)
    let cache = CacheManager::new(temp_path);
    let engine = QueryEngine::new(cache);
    let results = engine.find_symbol("main").unwrap();
    assert_eq!(results.len(), 1); // Should find the main function
    assert_eq!(results[0].symbol, "main");
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
