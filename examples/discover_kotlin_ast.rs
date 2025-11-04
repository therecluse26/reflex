#!/usr/bin/env -S cargo +nightly -Zscript
//! Discover Kotlin AST node types
//!
//! Run with: cargo run --example discover_kotlin_ast

use tree_sitter::{Parser};

fn main() -> anyhow::Result<()> {
    let mut parser = Parser::new();
    let language = tree_sitter_kotlin_ng::LANGUAGE;
    parser.set_language(&language.into())?;

    let source = r#"
class User(val name: String, val age: Int)

data class Person(val firstName: String, val lastName: String)

object Singleton {
    fun getInstance() = this
}

class Calculator {
    val property: String = ""

    fun add(a: Int, b: Int): Int {
        return a + b
    }
}

interface Repository {
    fun save(item: Any)
}
"#;

    let tree = parser.parse(source, None).unwrap();
    let root = tree.root_node();

    println!("Kotlin AST Structure:");
    print_tree(&root, source, 0);

    Ok(())
}

fn print_tree(node: &tree_sitter::Node, source: &str, depth: usize) {
    let indent = "  ".repeat(depth);
    let text = node.utf8_text(source.as_bytes()).unwrap_or("");
    let preview = if text.len() > 60 {
        format!("{}...", &text[..60].replace('\n', "\\n"))
    } else {
        text.replace('\n', "\\n")
    };

    if node.child_count() > 0 || !node.kind().starts_with(char::is_uppercase) {
        println!("{}{} {}", indent, node.kind(), if preview.is_empty() { "".to_string() } else { format!("\"{}\"", preview) });
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            print_tree(&child, source, depth + 1);
        }
    }
}
