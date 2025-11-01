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
//! - Modules
//! - Type aliases
//! - Macros

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
    symbols.extend(extract_modules(source, &root_node)?);
    symbols.extend(extract_type_aliases(source, &root_node)?);

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
                method_name,
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
                name,
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
        assert_eq!(symbols[0].symbol, "hello_world");
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
        assert_eq!(symbols[0].symbol, "User");
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
        assert!(method_symbols.iter().any(|s| s.symbol == "new"));
        assert!(method_symbols.iter().any(|s| s.symbol == "get_name"));

        // Check scope
        for method in method_symbols {
            assert_eq!(method.scope.as_ref().unwrap(), "impl User");
        }
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
        assert_eq!(symbols[0].symbol, "Status");
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
        assert_eq!(symbols[0].symbol, "Drawable");
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
}
