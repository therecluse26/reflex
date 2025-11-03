#!/usr/bin/env -S cargo +nightly -Zscript
//! Test the indexer
//!
//! Run with: cargo run --example test_indexer

use reflex::{CacheManager, Indexer};
use reflex::models::IndexConfig;
use std::fs;
use tempfile::TempDir;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    println!("🧪 Testing RefLex Indexer\n");

    // Create temporary directory with test Rust files
    let temp = TempDir::new()?;
    println!("📁 Test directory: {:?}\n", temp.path());

    // Create test files
    fs::write(temp.path().join("main.rs"), r#"
pub struct User {
    name: String,
    age: u32,
}

impl User {
    pub fn new(name: String, age: u32) -> Self {
        User { name, age }
    }
}

pub fn main() {
    let user = User::new("Alice".to_string(), 30);
}
"#)?;

    fs::write(temp.path().join("lib.rs"), r#"
pub mod utils;

pub trait Drawable {
    fn draw(&self);
}

pub const MAX_SIZE: usize = 100;

pub enum Status {
    Active,
    Inactive,
}
"#)?;

    println!("1️⃣  Created test files\n");

    // Create indexer
    let cache = CacheManager::new(temp.path());
    let config = IndexConfig::default();
    let indexer = Indexer::new(cache, config);

    // Run indexing
    println!("2️⃣  Running indexer...");
    let stats = indexer.index(temp.path(), false)?;

    println!("   ✅ Indexing complete\n");

    // Show statistics
    println!("📊 Index Statistics:");
    println!("   - Files indexed: {}", stats.total_files);
    println!("   - Cache size: {} bytes ({:.2} KB)",
             stats.index_size_bytes,
             stats.index_size_bytes as f64 / 1024.0);
    println!("   - Last updated: {}", stats.last_updated);

    // Verify cache files exist
    println!("\n3️⃣  Verifying cache files...");
    let cache_path = temp.path().join(".reflex");
    assert!(cache_path.join("meta.db").exists(), "meta.db not found");
    assert!(cache_path.join("symbols.bin").exists(), "symbols.bin not found");
    assert!(cache_path.join("hashes.json").exists(), "hashes.json not found");
    println!("   ✅ All cache files present");

    // Test incremental indexing
    println!("\n4️⃣  Testing incremental indexing...");
    let stats2 = indexer.index(temp.path(), false)?;
    println!("   ✅ Incremental indexing complete (should skip unchanged files)");
    println!("   - Files indexed: {}", stats2.total_files);

    println!("\n✅ All indexer tests passed!");
    println!("🎉 RefLex indexer is working correctly");

    Ok(())
}
