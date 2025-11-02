#!/usr/bin/env -S cargo +nightly -Zscript
//! Inspect Java AST structure
//!
//! Run with: cargo run --example test_java_tree

use tree_sitter::{Parser, Query, QueryCursor};
use streaming_iterator::StreamingIterator;

fn main() -> anyhow::Result<()> {
    println!("🔍 Inspecting Java AST Structure\n");

    let java_code = r#"
public class User {
    private String name;

    public void setName(String n) {
        this.name = n;
    }
}

public enum Day {
    MONDAY, TUESDAY;

    public void printDay() {
        System.out.println(this);
    }
}

public interface Repository {
    void save();
    String find(Long id);
}
"#;

    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_java::LANGUAGE.into())?;

    let tree = parser.parse(java_code, None).ok_or_else(|| anyhow::anyhow!("Failed to parse"))?;
    let root = tree.root_node();

    println!("📊 Complete AST Structure:");
    println!("{}", "-".repeat(80));

    fn print_node(node: &tree_sitter::Node, source: &str, depth: usize) {
        let kind = node.kind();
        let text = node.utf8_text(source.as_bytes()).unwrap_or("");
        let preview = if text.len() > 40 {
            format!("{}...", &text[..40].replace('\n', " "))
        } else {
            text.replace('\n', " ")
        };

        let indent = "  ".repeat(depth);
        println!("{}{} [{}:{}..{}:{}] \"{}\"",
                 indent,
                 kind,
                 node.start_position().row,
                 node.start_position().column,
                 node.end_position().row,
                 node.end_position().column,
                 preview);

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                print_node(&child, source, depth + 1);
            }
        }
    }

    print_node(&root, java_code, 0);

    println!("\n{}", "-".repeat(80));
    println!("🔍 Testing Query Patterns\n");

    // Test simple class query
    println!("1. Testing class_declaration query:");
    let class_query = Query::new(&tree_sitter_java::LANGUAGE.into(), r#"
        (class_declaration
            name: (identifier) @name)
    "#)?;

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&class_query, root, java_code.as_bytes());

    while let Some(m) = matches.next() {
        for capture in m.captures {
            let text = capture.node.utf8_text(java_code.as_bytes())?;
            println!("   ✅ Found class: {}", text);
        }
    }

    // Test method query with named field (WRONG - will fail)
    println!("\n2a. Testing method query with 'name:' field (expected to fail):");
    let method_result = Query::new(&tree_sitter_java::LANGUAGE.into(), r#"
        (class_declaration
            body: (class_body
                (method_declaration
                    name: (identifier) @method_name)))
    "#);

    match method_result {
        Ok(query) => {
            let mut cursor = QueryCursor::new();
            let mut matches = cursor.matches(&query, root, java_code.as_bytes());

            while let Some(m) = matches.next() {
                for capture in m.captures {
                    let text = capture.node.utf8_text(java_code.as_bytes())?;
                    println!("   ✅ Found method: {}", text);
                }
            }
        }
        Err(e) => {
            println!("   ❌ Query failed: {}", e);
        }
    }

    // Test method query without named field (CORRECT)
    println!("\n2b. Testing method query without named field (should work):");
    let method_result2 = Query::new(&tree_sitter_java::LANGUAGE.into(), r#"
        (class_declaration
            body: (class_body
                (method_declaration
                    (identifier) @method_name)))
    "#);

    match method_result2 {
        Ok(query) => {
            let mut cursor = QueryCursor::new();
            let mut matches = cursor.matches(&query, root, java_code.as_bytes());

            while let Some(m) = matches.next() {
                for capture in m.captures {
                    let text = capture.node.utf8_text(java_code.as_bytes())?;
                    println!("   ✅ Found method: {}", text);
                }
            }
        }
        Err(e) => {
            println!("   ❌ Query failed: {}", e);
        }
    }

    // Test field query
    println!("\n3. Testing field query pattern:");
    let field_result = Query::new(&tree_sitter_java::LANGUAGE.into(), r#"
        (class_declaration
            body: (class_body
                (field_declaration)))
    "#);

    match field_result {
        Ok(query) => {
            let mut cursor = QueryCursor::new();
            let mut matches = cursor.matches(&query, root, java_code.as_bytes());

            let mut count = 0;
            while matches.next().is_some() {
                count += 1;
            }
            println!("   ✅ Found {} field declarations", count);
        }
        Err(e) => {
            println!("   ❌ Query failed: {}", e);
        }
    }

    // Test enum method query (broken pattern from java.rs)
    println!("\n4. Testing BROKEN enum method pattern from java.rs:");
    let broken_enum_query = Query::new(&tree_sitter_java::LANGUAGE.into(), r#"
        (enum_declaration
            body: (enum_body
                (method_declaration)))
    "#);

    match broken_enum_query {
        Ok(_) => {
            println!("   ❌ Query should have failed but didn't!");
        }
        Err(e) => {
            println!("   ✅ Query correctly failed: {}", e);
        }
    }

    // Test CORRECT enum method query
    println!("\n5. Testing CORRECT enum method pattern:");
    let correct_enum_query = Query::new(&tree_sitter_java::LANGUAGE.into(), r#"
        (enum_declaration
            body: (enum_body
                (enum_body_declarations
                    (method_declaration
                        (identifier) @method_name))))
    "#);

    match correct_enum_query {
        Ok(query) => {
            let mut cursor = QueryCursor::new();
            let mut matches = cursor.matches(&query, root, java_code.as_bytes());

            let mut count = 0;
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    let text = capture.node.utf8_text(java_code.as_bytes())?;
                    println!("   ✅ Found enum method: {}", text);
                    count += 1;
                }
            }
            println!("   Total: {} enum methods", count);
        }
        Err(e) => {
            println!("   ❌ Query failed: {}", e);
        }
    }

    println!("\n✅ Inspection complete!");

    Ok(())
}
