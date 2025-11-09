#!/usr/bin/env -S cargo +nightly -Zscript
//! Test the cache system
//!
//! Run with: cargo run --example test_cache

use reflex::CacheManager;
use std::collections::HashMap;
use tempfile::TempDir;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    println!("🧪 Testing Reflex Cache System\n");

    // Create temporary directory
    let temp = TempDir::new()?;
    println!("📁 Test directory: {:?}\n", temp.path());

    // Test 1: Initialize cache
    println!("1️⃣  Initializing cache...");
    let cache = CacheManager::new(temp.path());
    cache.init()?;
    println!("   ✅ Cache initialized");

    // Verify files exist
    let cache_path = temp.path().join(".reflex");
    assert!(cache_path.join("meta.db").exists(), "meta.db not created");
    assert!(cache_path.join("symbols.bin").exists(), "symbols.bin not created");
    assert!(cache_path.join("tokens.bin").exists(), "tokens.bin not created");
    assert!(cache_path.join("hashes.json").exists(), "hashes.json not created");
    assert!(cache_path.join("config.toml").exists(), "config.toml not created");
    println!("   ✅ All 5 cache files created\n");

    // Test 2: Hash persistence (branch-aware)
    println!("2️⃣  Testing branch-aware hash persistence...");

    // First add files to the files table
    cache.update_file("src/main.rs", "rust", 100)?;
    cache.update_file("src/lib.rs", "rust", 200)?;

    // Then record hashes for a specific branch
    cache.record_branch_file("src/main.rs", "main", "abc123def456", Some("commit123"))?;
    cache.record_branch_file("src/lib.rs", "main", "789ghi012jkl", Some("commit123"))?;
    println!("   ✅ Saved 2 hashes for branch 'main'");

    let loaded = cache.load_hashes_for_branch("main")?;
    assert_eq!(loaded.len(), 2);
    assert_eq!(loaded.get("src/main.rs"), Some(&"abc123def456".to_string()));
    println!("   ✅ Loaded {} hashes successfully\n", loaded.len());

    // Test 3: Statistics
    println!("3️⃣  Testing cache statistics...");
    let stats = cache.stats()?;
    println!("   📊 Cache Statistics:");
    println!("      - Total files: {}", stats.total_files);
    println!("      - Cache size: {} bytes ({:.2} KB)",
             stats.index_size_bytes,
             stats.index_size_bytes as f64 / 1024.0);
    println!("      - Last updated: {}", stats.last_updated);
    println!("   ✅ Statistics retrieved\n");

    // Test 4: Cache existence check
    println!("4️⃣  Testing cache existence...");
    assert!(cache.exists(), "Cache should exist");
    println!("   ✅ Cache exists\n");

    // Test 5: Cache clearing
    println!("5️⃣  Testing cache clearing...");
    cache.clear()?;
    assert!(!cache.exists(), "Cache should not exist after clearing");
    println!("   ✅ Cache cleared successfully\n");

    println!("✅ All cache tests passed!\n");
    println!("🎉 Reflex cache system is working correctly");

    Ok(())
}
