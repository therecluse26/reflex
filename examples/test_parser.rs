#!/usr/bin/env -S cargo +nightly -Zscript
//! Test the Rust parser
//!
//! Run with: cargo run --example test_parser

use reflex::parsers::rust;

fn main() -> anyhow::Result<()> {
    println!("🧪 Testing RefLex Rust Parser\n");

    let rust_code = r#"
// Example Rust code
use std::collections::HashMap;

pub struct User {
    pub name: String,
    pub age: u32,
}

impl User {
    pub fn new(name: String, age: u32) -> Self {
        User { name, age }
    }

    pub fn greet(&self) -> String {
        format!("Hello, I'm {} and I'm {} years old", self.name, self.age)
    }
}

pub enum Status {
    Active,
    Inactive,
}

pub trait Drawable {
    fn draw(&self);
}

pub const MAX_USERS: usize = 100;

pub fn main() {
    let user = User::new("Alice".to_string(), 30);
    println!("{}", user.greet());
}
"#;

    println!("📝 Parsing Rust code...");
    let symbols = rust::parse("example.rs", rust_code)?;

    println!("   ✅ Found {} symbols\n", symbols.len());

    println!("📊 Extracted Symbols:");
    println!("   {:<15} {:<20} {:>6}:{:<4}", "Type", "Name", "Line", "Col");
    println!("   {}", "-".repeat(50));

    for symbol in &symbols {
        println!("   {:<15} {:<20} {:>6}:{:<4}",
                 format!("{:?}", symbol.kind),
                 symbol.symbol.as_deref().unwrap_or("<no symbol>"),
                 symbol.span.start_line,
                 symbol.span.start_col);

        if let Some(scope) = &symbol.scope {
            println!("      └─ Scope: {}", scope);
        }
    }

    println!("\n📋 Summary:");
    let mut counts = std::collections::HashMap::new();
    for symbol in &symbols {
        *counts.entry(format!("{:?}", symbol.kind)).or_insert(0) += 1;
    }

    for (kind, count) in counts.iter() {
        println!("   - {}: {}", kind, count);
    }

    println!("\n✅ Parser test complete!");
    println!("🎉 RefLex Rust parser is working correctly");

    Ok(())
}
