#!/usr/bin/env -S cargo +nightly -Zscript
//! Test the query engine
//!
//! Run with: cargo run --example test_query

use reflex::{CacheManager, Indexer, QueryEngine, QueryFilter};
use reflex::models::IndexConfig;
use std::fs;
use tempfile::TempDir;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    println!("🧪 Testing RefLex Query Engine\n");

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

    pub fn greet(&self) -> String {
        format!("Hello, {}!", self.name)
    }
}

pub fn main() {
    let user = User::new("Alice".to_string(), 30);
    println!("{}", user.greet());
}
"#)?;

    fs::write(temp.path().join("lib.rs"), r#"
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

    // Index the files
    let cache = CacheManager::new(temp.path());
    let config = IndexConfig::default();
    let indexer = Indexer::new(cache, config);

    println!("2️⃣  Indexing files...");
    let stats = indexer.index(temp.path(), false)?;
    println!("   ✅ Indexed {} files\n", stats.total_files);

    // Create query engine
    let cache = CacheManager::new(temp.path());
    let engine = QueryEngine::new(cache);

    // Test 1: Exact symbol search
    println!("3️⃣  Testing exact symbol search...");
    let results = engine.search("symbol:User", QueryFilter::default())?;
    println!("   Query: symbol:User");
    println!("   Results: {}", results.len());
    for result in &results {
        println!("     - {:?} '{}' at {}:{}", result.kind, result.symbol.as_deref().unwrap_or("<no symbol>"), result.path, result.span.start_line);
    }
    assert!(results.len() > 0, "Expected at least one result for 'User'");
    println!("   ✅ Exact search works\n");

    // Test 2: Substring search
    println!("4️⃣  Testing substring search...");
    let results = engine.search("greet", QueryFilter::default())?;
    println!("   Query: greet");
    println!("   Results: {}", results.len());
    for result in &results {
        println!("     - {:?} '{}' at {}:{}", result.kind, result.symbol.as_deref().unwrap_or("<no symbol>"), result.path, result.span.start_line);
    }
    assert!(results.len() > 0, "Expected at least one result for 'greet'");
    println!("   ✅ Substring search works\n");

    // Test 3: Prefix search
    println!("5️⃣  Testing prefix search...");
    let results = engine.search("symbol:n*", QueryFilter::default())?;
    println!("   Query: symbol:n*");
    println!("   Results: {}", results.len());
    for result in &results {
        println!("     - {:?} '{}' at {}:{}", result.kind, result.symbol.as_deref().unwrap_or("<no symbol>"), result.path, result.span.start_line);
    }
    assert!(results.len() > 0, "Expected at least one result for prefix 'n*'");
    println!("   ✅ Prefix search works\n");

    // Test 4: List all symbols
    println!("6️⃣  Testing list all symbols...");
    let results = engine.search("symbol:*", QueryFilter::default())?;
    println!("   Query: symbol:*");
    println!("   Results: {}", results.len());
    assert!(results.len() > 0, "Should find symbols");
    println!("   ✅ List all works\n");

    println!("✅ All query engine tests passed!");
    println!("🎉 RefLex query engine is working correctly");

    Ok(())
}
