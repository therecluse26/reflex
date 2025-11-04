#!/usr/bin/env -S cargo +nightly -Zscript
//! Discover Zig AST node types
//!
//! Run with: cargo run --example discover_zig_ast

use tree_sitter::{Parser};

fn main() -> anyhow::Result<()> {
    let mut parser = Parser::new();
    let language = tree_sitter_zig::LANGUAGE;
    parser.set_language(&language.into())?;

    let source = r#"
pub fn add(a: i32, b: i32) i32 {
    return a + b;
}

const Point = struct {
    x: f32,
    y: f32,
};

const Status = enum {
    active,
    inactive,
};

test "basic test" {
    const x = 1;
}
"#;

    let tree = parser.parse(source, None).unwrap();
    let root = tree.root_node();

    println!("Zig AST Structure:");
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
