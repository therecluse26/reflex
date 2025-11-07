//! Rust language parser using Tree-sitter
//!
//! Extracts symbols from Rust source code:
//! - Functions (fn)
//! - Structs
//! - Enums
//! - Traits
//! - Impl blocks
//! - Constants
//! - Static variables
//! - Local variables (let bindings)
//! - Modules
//! - Type aliases
//! - Macros (macro_rules! definitions)

use anyhow::{Context, Result};
use streaming_iterator::StreamingIterator;
use tree_sitter::{Parser, Query, QueryCursor};
use crate::models::{Language, SearchResult, Span, SymbolKind};

/// Parse Rust source code and extract symbols
pub fn parse(path: &str, source: &str) -> Result<Vec<SearchResult>> {
    let mut parser = Parser::new();
    let language = tree_sitter_rust::LANGUAGE;

    parser
        .set_language(&language.into())
        .context("Failed to set Rust language")?;

    let tree = parser
        .parse(source, None)
        .context("Failed to parse Rust source")?;

    let root_node = tree.root_node();

    let mut symbols = Vec::new();

    // Extract different types of symbols using Tree-sitter queries
    symbols.extend(extract_functions(source, &root_node)?);
    symbols.extend(extract_structs(source, &root_node)?);
    symbols.extend(extract_enums(source, &root_node)?);
    symbols.extend(extract_traits(source, &root_node)?);
    symbols.extend(extract_impls(source, &root_node)?);
    symbols.extend(extract_constants(source, &root_node)?);
    symbols.extend(extract_statics(source, &root_node)?);
    symbols.extend(extract_local_variables(source, &root_node)?);
    symbols.extend(extract_modules(source, &root_node)?);
    symbols.extend(extract_type_aliases(source, &root_node)?);
    symbols.extend(extract_macros(source, &root_node)?);
    symbols.extend(extract_attributes(source, &root_node)?);


    // Add file path to all symbols
    for symbol in &mut symbols {
        symbol.path = path.to_string();
        symbol.lang = Language::Rust;
    }

    Ok(symbols)
}

/// Extract function definitions
fn extract_functions(source: &str, root: &tree_sitter::Node) -> Result<Vec<SearchResult>> {
    let language = tree_sitter_rust::LANGUAGE;
    let query_str = r#"
        (function_item
            name: (identifier) @name) @function
    "#;

    let query = Query::new(&language.into(), query_str)
        .context("Failed to create function query")?;

    extract_symbols(source, root, &query, SymbolKind::Function, None)
}

/// Extract struct definitions
fn extract_structs(source: &str, root: &tree_sitter::Node) -> Result<Vec<SearchResult>> {
    let language = tree_sitter_rust::LANGUAGE;
    let query_str = r#"
        (struct_item
            name: (type_identifier) @name) @struct
    "#;

    let query = Query::new(&language.into(), query_str)
        .context("Failed to create struct query")?;

    extract_symbols(source, root, &query, SymbolKind::Struct, None)
}

/// Extract enum definitions
fn extract_enums(source: &str, root: &tree_sitter::Node) -> Result<Vec<SearchResult>> {
    let language = tree_sitter_rust::LANGUAGE;
    let query_str = r#"
        (enum_item
            name: (type_identifier) @name) @enum
    "#;

    let query = Query::new(&language.into(), query_str)
        .context("Failed to create enum query")?;

    extract_symbols(source, root, &query, SymbolKind::Enum, None)
}

/// Extract trait definitions
fn extract_traits(source: &str, root: &tree_sitter::Node) -> Result<Vec<SearchResult>> {
    let language = tree_sitter_rust::LANGUAGE;
    let query_str = r#"
        (trait_item
            name: (type_identifier) @name) @trait
    "#;

    let query = Query::new(&language.into(), query_str)
        .context("Failed to create trait query")?;

    extract_symbols(source, root, &query, SymbolKind::Trait, None)
}

/// Extract impl blocks
fn extract_impls(source: &str, root: &tree_sitter::Node) -> Result<Vec<SearchResult>> {
    let language = tree_sitter_rust::LANGUAGE;

    // Extract methods from impl blocks
    let query_str = r#"
        (impl_item
            type: (type_identifier) @impl_name
            body: (declaration_list
                (function_item
                    name: (identifier) @method_name))) @impl
    "#;

    let query = Query::new(&language.into(), query_str)
        .context("Failed to create impl query")?;

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, *root, source.as_bytes());

    let mut symbols = Vec::new();

    while let Some(match_) = matches.next() {
        let mut impl_name = None;
        let mut method_name = None;
        let mut method_node = None;

        for capture in match_.captures {
            let capture_name: &str = &query.capture_names()[capture.index as usize];
            match capture_name {
                "impl_name" => {
                    impl_name = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                }
                "method_name" => {
                    method_name = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                    // Find the parent function_item node
                    let mut current = capture.node;
                    while let Some(parent) = current.parent() {
                        if parent.kind() == "function_item" {
                            method_node = Some(parent);
                            break;
                        }
                        current = parent;
                    }
                }
                _ => {}
            }
        }

        if let (Some(impl_name), Some(method_name), Some(node)) = (impl_name, method_name, method_node) {
            let scope = format!("impl {}", impl_name);
            let span = node_to_span(&node);
            let preview = extract_preview(source, &span);

            symbols.push(SearchResult::new(
                String::new(), // Path will be filled in later
                Language::Rust,
                SymbolKind::Method,
                Some(method_name),
                span,
                Some(scope),
                preview,
            ));
        }
    }

    Ok(symbols)
}

/// Extract constants
fn extract_constants(source: &str, root: &tree_sitter::Node) -> Result<Vec<SearchResult>> {
    let language = tree_sitter_rust::LANGUAGE;
    let query_str = r#"
        (const_item
            name: (identifier) @name) @const
    "#;

    let query = Query::new(&language.into(), query_str)
        .context("Failed to create const query")?;

    extract_symbols(source, root, &query, SymbolKind::Constant, None)
}

/// Extract static variables
fn extract_statics(source: &str, root: &tree_sitter::Node) -> Result<Vec<SearchResult>> {
    let language = tree_sitter_rust::LANGUAGE;
    let query_str = r#"
        (static_item
            name: (identifier) @name) @static
    "#;

    let query = Query::new(&language.into(), query_str)
        .context("Failed to create static query")?;

    extract_symbols(source, root, &query, SymbolKind::Variable, None)
}

/// Extract local variable bindings (let statements)
fn extract_local_variables(source: &str, root: &tree_sitter::Node) -> Result<Vec<SearchResult>> {
    let language = tree_sitter_rust::LANGUAGE;
    let query_str = r#"
        (let_declaration
            pattern: (identifier) @name) @let
    "#;

    let query = Query::new(&language.into(), query_str)
        .context("Failed to create let declaration query")?;

    extract_symbols(source, root, &query, SymbolKind::Variable, None)
}

/// Extract module declarations
fn extract_modules(source: &str, root: &tree_sitter::Node) -> Result<Vec<SearchResult>> {
    let language = tree_sitter_rust::LANGUAGE;
    let query_str = r#"
        (mod_item
            name: (identifier) @name) @module
    "#;

    let query = Query::new(&language.into(), query_str)
        .context("Failed to create module query")?;

    extract_symbols(source, root, &query, SymbolKind::Module, None)
}

/// Extract type aliases
fn extract_type_aliases(source: &str, root: &tree_sitter::Node) -> Result<Vec<SearchResult>> {
    let language = tree_sitter_rust::LANGUAGE;
    let query_str = r#"
        (type_item
            name: (type_identifier) @name) @type
    "#;

    let query = Query::new(&language.into(), query_str)
        .context("Failed to create type query")?;

    extract_symbols(source, root, &query, SymbolKind::Type, None)
}

/// Extract macro definitions (macro_rules!)
fn extract_macros(source: &str, root: &tree_sitter::Node) -> Result<Vec<SearchResult>> {
    let language = tree_sitter_rust::LANGUAGE;
    let query_str = r#"
        (macro_definition
            name: (identifier) @name) @macro
    "#;

    let query = Query::new(&language.into(), query_str)
        .context("Failed to create macro query")?;

    extract_symbols(source, root, &query, SymbolKind::Macro, None)
}

/// Extract attributes: BOTH definitions and uses
/// Definitions: #[proc_macro_attribute] pub fn route(...)
/// Uses: #[test] fn my_test(), #[derive(Debug)] struct Foo
fn extract_attributes(source: &str, root: &tree_sitter::Node) -> Result<Vec<SearchResult>> {
    let language = tree_sitter_rust::LANGUAGE;
    let mut symbols = Vec::new();

    // Part 1: Extract attribute DEFINITIONS (proc macro attributes)
    let func_query_str = r#"
        (function_item
            name: (identifier) @name) @function
    "#;

    let func_query = Query::new(&language.into(), func_query_str)
        .context("Failed to create function query")?;

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&func_query, *root, source.as_bytes());

    while let Some(match_) = matches.next() {
        let mut name = None;
        let mut func_node = None;

        for capture in match_.captures {
            let capture_name: &str = &func_query.capture_names()[capture.index as usize];
            match capture_name {
                "name" => {
                    name = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                }
                "function" => {
                    func_node = Some(capture.node);
                }
                _ => {}
            }
        }

        // Check if this function has #[proc_macro_attribute] attribute
        if let (Some(name), Some(func_node)) = (name, func_node) {
            let mut has_proc_macro_attr = false;

            if let Some(parent) = func_node.parent() {
                let mut func_index = None;
                for i in 0..parent.child_count() {
                    if let Some(child) = parent.child(i) {
                        if child.id() == func_node.id() {
                            func_index = Some(i);
                            break;
                        }
                    }
                }

                if let Some(func_idx) = func_index {
                    for i in (0..func_idx).rev() {
                        if let Some(child) = parent.child(i) {
                            if child.kind() == "attribute_item" {
                                let attr_text = child.utf8_text(source.as_bytes()).unwrap_or("");
                                if attr_text.contains("proc_macro_attribute") {
                                    has_proc_macro_attr = true;
                                }
                            } else if !child.kind().contains("comment") && child.kind() != "line_comment" {
                                break;
                            }
                        }
                    }
                }
            }

            if has_proc_macro_attr {
                let span = node_to_span(&func_node);
                let preview = extract_preview(source, &span);

                symbols.push(SearchResult::new(
                    String::new(),
                    Language::Rust,
                    SymbolKind::Attribute,
                    Some(name),
                    span,
                    None,
                    preview,
                ));
            }
        }
    }

    // Part 2: Extract attribute USES (#[test], #[derive(...)], etc.)
    let attr_query_str = r#"
        (attribute_item
            (attribute
                (identifier) @attr_name)) @attr
    "#;

    let attr_query = Query::new(&language.into(), attr_query_str)
        .context("Failed to create attribute use query")?;

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&attr_query, *root, source.as_bytes());

    while let Some(match_) = matches.next() {
        let mut attr_name = None;
        let mut attr_node = None;

        for capture in match_.captures {
            let capture_name: &str = &attr_query.capture_names()[capture.index as usize];
            match capture_name {
                "attr_name" => {
                    attr_name = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                }
                "attr" => {
                    attr_node = Some(capture.node);
                }
                _ => {}
            }
        }

        if let (Some(name), Some(node)) = (attr_name, attr_node) {
            let span = node_to_span(&node);
            let preview = extract_preview(source, &span);

            symbols.push(SearchResult::new(
                String::new(),
                Language::Rust,
                SymbolKind::Attribute,
                Some(name),
                span,
                None,
                preview,
            ));
        }
    }

    Ok(symbols)
}

/// Generic symbol extraction helper
fn extract_symbols(
    source: &str,
    root: &tree_sitter::Node,
    query: &Query,
    kind: SymbolKind,
    scope: Option<String>,
) -> Result<Vec<SearchResult>> {
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(query, *root, source.as_bytes());

    let mut symbols = Vec::new();

    while let Some(match_) = matches.next() {
        // Find the name capture and the full node
        let mut name = None;
        let mut full_node = None;

        for capture in match_.captures {
            let capture_name: &str = &query.capture_names()[capture.index as usize];
            if capture_name == "name" {
                name = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
            } else {
                // Assume any other capture is the full node
                full_node = Some(capture.node);
            }
        }

        if let (Some(name), Some(node)) = (name, full_node) {
            let span = node_to_span(&node);
            let preview = extract_preview(source, &span);

            symbols.push(SearchResult::new(
                String::new(), // Path will be filled in later
                Language::Rust,
                kind.clone(),
                Some(name),
                span,
                scope.clone(),
                preview,
            ));
        }
    }

    Ok(symbols)
}

/// Convert a Tree-sitter node to a Span
fn node_to_span(node: &tree_sitter::Node) -> Span {
    let start = node.start_position();
    let end = node.end_position();

    Span::new(
        start.row + 1,  // Convert 0-indexed to 1-indexed
        start.column,
        end.row + 1,
        end.column,
    )
}

/// Extract a preview (5-7 lines) around the symbol
fn extract_preview(source: &str, span: &Span) -> String {
    let lines: Vec<&str> = source.lines().collect();

    // Extract 7 lines: the start line and 6 following lines
    // This provides enough context for AI agents to understand the code
    let start_idx = (span.start_line - 1) as usize; // Convert back to 0-indexed
    let end_idx = (start_idx + 7).min(lines.len());

    lines[start_idx..end_idx].join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_function() {
        let source = r#"
            fn hello_world() {
                println!("Hello, world!");
            }
        "#;

        let symbols = parse("test.rs", source).unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].symbol.as_deref(), Some("hello_world"));
        assert!(matches!(symbols[0].kind, SymbolKind::Function));
    }

    #[test]
    fn test_parse_struct() {
        let source = r#"
            struct User {
                name: String,
                age: u32,
            }
        "#;

        let symbols = parse("test.rs", source).unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].symbol.as_deref(), Some("User"));
        assert!(matches!(symbols[0].kind, SymbolKind::Struct));
    }

    #[test]
    fn test_parse_impl() {
        let source = r#"
            struct User {
                name: String,
            }

            impl User {
                fn new(name: String) -> Self {
                    User { name }
                }

                fn get_name(&self) -> &str {
                    &self.name
                }
            }
        "#;

        let symbols = parse("test.rs", source).unwrap();

        // Should find: struct User, method new, method get_name
        assert!(symbols.len() >= 3);

        let method_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Method))
            .collect();

        assert_eq!(method_symbols.len(), 2);
        assert!(method_symbols.iter().any(|s| s.symbol.as_deref() == Some("new")));
        assert!(method_symbols.iter().any(|s| s.symbol.as_deref() == Some("get_name")));

        // Note: scope field was removed from SearchResult for token optimization
        // Methods are identified by SymbolKind::Method
    }

    #[test]
    fn test_parse_enum() {
        let source = r#"
            enum Status {
                Active,
                Inactive,
            }
        "#;

        let symbols = parse("test.rs", source).unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].symbol.as_deref(), Some("Status"));
        assert!(matches!(symbols[0].kind, SymbolKind::Enum));
    }

    #[test]
    fn test_parse_trait() {
        let source = r#"
            trait Drawable {
                fn draw(&self);
            }
        "#;

        let symbols = parse("test.rs", source).unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].symbol.as_deref(), Some("Drawable"));
        assert!(matches!(symbols[0].kind, SymbolKind::Trait));
    }

    #[test]
    fn test_parse_multiple_symbols() {
        let source = r#"
            const MAX_SIZE: usize = 100;

            struct Config {
                size: usize,
            }

            fn create_config() -> Config {
                Config { size: MAX_SIZE }
            }
        "#;

        let symbols = parse("test.rs", source).unwrap();

        // Should find: const, struct, function
        assert_eq!(symbols.len(), 3);

        let kinds: Vec<&SymbolKind> = symbols.iter().map(|s| &s.kind).collect();
        assert!(kinds.contains(&&SymbolKind::Constant));
        assert!(kinds.contains(&&SymbolKind::Struct));
        assert!(kinds.contains(&&SymbolKind::Function));
    }

    #[test]
    fn test_local_variables_included() {
        let source = r#"
            fn calculate(input: i32) -> i32 {
                let local_var = input * 2;
                let result = local_var + 10;
                result
            }

            struct Calculator;

            impl Calculator {
                fn compute(&self, value: i32) -> i32 {
                    let temp = value * 3;
                    let mut final_value = temp + 5;
                    final_value += 1;
                    final_value
                }
            }
        "#;

        let symbols = parse("test.rs", source).unwrap();

        // Filter to just variables
        let variables: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Variable))
            .collect();

        // Check that local variables are captured
        assert!(variables.iter().any(|v| v.symbol.as_deref() == Some("local_var")));
        assert!(variables.iter().any(|v| v.symbol.as_deref() == Some("result")));
        assert!(variables.iter().any(|v| v.symbol.as_deref() == Some("temp")));
        assert!(variables.iter().any(|v| v.symbol.as_deref() == Some("final_value")));

        // Note: scope field was removed from SearchResult for token optimization
    }

    #[test]
    fn test_static_variables() {
        let source = r#"
            static GLOBAL_COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            static mut MUTABLE_GLOBAL: i32 = 0;

            const MAX_SIZE: usize = 100;

            fn increment() {
                GLOBAL_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
        "#;

        let symbols = parse("test.rs", source).unwrap();

        // Filter to statics and constants
        let statics: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Variable))
            .collect();

        let constants: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Constant))
            .collect();

        // Check that static variables are captured
        assert!(statics.iter().any(|v| v.symbol.as_deref() == Some("GLOBAL_COUNTER")));
        assert!(statics.iter().any(|v| v.symbol.as_deref() == Some("MUTABLE_GLOBAL")));

        // Check that constants are still separate
        assert!(constants.iter().any(|c| c.symbol.as_deref() == Some("MAX_SIZE")));
    }

    #[test]
    fn test_macros() {
        let source = r#"
            macro_rules! say_hello {
                () => {
                    println!("Hello!");
                };
            }

            macro_rules! vec_of_strings {
                ($($x:expr),*) => {
                    vec![$($x.to_string()),*]
                };
            }

            fn main() {
                say_hello!();
            }
        "#;

        let symbols = parse("test.rs", source).unwrap();

        // Filter to macros
        let macros: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Macro))
            .collect();

        // Check that macros are captured
        assert!(macros.iter().any(|m| m.symbol.as_deref() == Some("say_hello")));
        assert!(macros.iter().any(|m| m.symbol.as_deref() == Some("vec_of_strings")));
        assert_eq!(macros.len(), 2);
    }

    #[test]
    fn test_attribute_proc_macros() {
        let source = r#"
            use proc_macro::TokenStream;

            #[proc_macro_attribute]
            pub fn test(attr: TokenStream, item: TokenStream) -> TokenStream {
                item
            }

            #[proc_macro_attribute]
            pub fn route(attr: TokenStream, item: TokenStream) -> TokenStream {
                item
            }

            // Regular function - should NOT be captured
            pub fn helper() {}
        "#;

        let symbols = parse("test.rs", source).unwrap();

        // Filter to attributes
        let attributes: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Attribute))
            .collect();

        // Check that attribute proc macro DEFINITIONS are captured
        assert!(attributes.iter().any(|a| a.symbol.as_deref() == Some("test")));
        assert!(attributes.iter().any(|a| a.symbol.as_deref() == Some("route")));

        // Verify helper function is NOT captured as attribute
        assert!(!attributes.iter().any(|a| a.symbol.as_deref() == Some("helper")));

        // Should find 2 proc macro definitions + 2 attribute uses (#[proc_macro_attribute])
        assert_eq!(attributes.len(), 4);
    }

    #[test]
    fn test_attribute_uses() {
        let source = r#"
            #[test]
            fn test_something() {
                assert_eq!(1, 1);
            }

            #[test]
            #[should_panic]
            fn test_panic() {
                panic!("expected");
            }

            #[derive(Debug, Clone)]
            struct MyStruct {
                field: i32
            }

            #[cfg(test)]
            mod tests {
                #[test]
                fn nested_test() {}
            }
        "#;

        let symbols = parse("test.rs", source).unwrap();

        // Filter to attributes
        let attributes: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Attribute))
            .collect();

        // Check that attribute USES are captured
        assert!(attributes.iter().any(|a| a.symbol.as_deref() == Some("test")));
        assert!(attributes.iter().any(|a| a.symbol.as_deref() == Some("should_panic")));
        assert!(attributes.iter().any(|a| a.symbol.as_deref() == Some("derive")));
        assert!(attributes.iter().any(|a| a.symbol.as_deref() == Some("cfg")));

        // Should find: test (3x), should_panic (1x), derive (1x), cfg (1x) = 6 total
        assert_eq!(attributes.len(), 6);
    }
}
