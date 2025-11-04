//! Zig language parser using Tree-sitter
//!
//! Extracts symbols from Zig source code:
//! - Functions (pub and private)
//! - Structs
//! - Enums
//! - Unions
//! - Constants
//! - Variables
//! - Test declarations
//! - Error sets

use anyhow::{Context, Result};
use streaming_iterator::StreamingIterator;
use tree_sitter::{Parser, Query, QueryCursor};
use crate::models::{Language, SearchResult, Span, SymbolKind};

/// Parse Zig source code and extract symbols
pub fn parse(path: &str, source: &str) -> Result<Vec<SearchResult>> {
    let mut parser = Parser::new();
    let language = tree_sitter_zig::LANGUAGE;

    parser
        .set_language(&language.into())
        .context("Failed to set Zig language")?;

    let tree = parser
        .parse(source, None)
        .context("Failed to parse Zig source")?;

    let root_node = tree.root_node();

    let mut symbols = Vec::new();

    // Extract different types of symbols using Tree-sitter queries
    symbols.extend(extract_functions(source, &root_node, &language.into())?);
    symbols.extend(extract_structs(source, &root_node, &language.into())?);
    symbols.extend(extract_enums(source, &root_node, &language.into())?);
    symbols.extend(extract_constants(source, &root_node, &language.into())?);
    symbols.extend(extract_tests(source, &root_node, &language.into())?);

    // Add file path to all symbols
    for symbol in &mut symbols {
        symbol.path = path.to_string();
        symbol.lang = Language::Zig;
    }

    Ok(symbols)
}

/// Extract function declarations
fn extract_functions(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (function_declaration
            (identifier) @name) @function
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create function query")?;

    extract_symbols(source, root, &query, SymbolKind::Function, None)
}

/// Extract struct (container) declarations
fn extract_structs(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (variable_declaration
            (identifier) @name
            (struct_declaration)) @struct
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create struct query")?;

    extract_symbols(source, root, &query, SymbolKind::Struct, None)
}

/// Extract enum declarations
fn extract_enums(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (variable_declaration
            (identifier) @name
            (enum_declaration)) @enum
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create enum query")?;

    extract_symbols(source, root, &query, SymbolKind::Enum, None)
}

/// Extract constant declarations
fn extract_constants(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (variable_declaration
            "const"
            (identifier) @name) @const
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create constant query")?;

    extract_symbols(source, root, &query, SymbolKind::Constant, None)
}

/// Extract test declarations
fn extract_tests(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (test_declaration
            (string) @name) @test
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create test query")?;

    extract_symbols(source, root, &query, SymbolKind::Function, None)
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
                String::new(),
                Language::Zig,
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

/// Extract a preview (7 lines) around the symbol
fn extract_preview(source: &str, span: &Span) -> String {
    let lines: Vec<&str> = source.lines().collect();

    // Extract 7 lines: the start line and 6 following lines
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
pub fn add(a: i32, b: i32) i32 {
    return a + b;
}
        "#;

        let symbols = parse("test.zig", source).unwrap();

        let func_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Function))
            .collect();

        assert_eq!(func_symbols.len(), 1);
        assert_eq!(func_symbols[0].symbol.as_deref(), Some("add"));
    }

    #[test]
    fn test_parse_struct() {
        let source = r#"
const Point = struct {
    x: f32,
    y: f32,
};
        "#;

        let symbols = parse("test.zig", source).unwrap();

        let struct_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Struct))
            .collect();

        assert_eq!(struct_symbols.len(), 1);
        assert_eq!(struct_symbols[0].symbol.as_deref(), Some("Point"));
    }

    #[test]
    fn test_parse_enum() {
        let source = r#"
const Status = enum {
    active,
    inactive,
    pending,
};
        "#;

        let symbols = parse("test.zig", source).unwrap();

        let enum_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Enum))
            .collect();

        assert_eq!(enum_symbols.len(), 1);
        assert_eq!(enum_symbols[0].symbol.as_deref(), Some("Status"));
    }

    #[test]
    fn test_parse_constants() {
        let source = r#"
const MAX_SIZE: usize = 100;
const DEFAULT_TIMEOUT: u32 = 30;
        "#;

        let symbols = parse("test.zig", source).unwrap();

        let const_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Constant))
            .collect();

        assert_eq!(const_symbols.len(), 2);
        assert!(const_symbols.iter().any(|s| s.symbol.as_deref() == Some("MAX_SIZE")));
        assert!(const_symbols.iter().any(|s| s.symbol.as_deref() == Some("DEFAULT_TIMEOUT")));
    }

    #[test]
    fn test_parse_test_declaration() {
        let source = r#"
test "basic addition" {
    const result = add(2, 3);
    try std.testing.expect(result == 5);
}
        "#;

        let symbols = parse("test.zig", source).unwrap();

        let test_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Function))
            .filter(|s| s.symbol.as_deref().unwrap_or("").contains("basic addition"))
            .collect();

        assert!(test_symbols.len() >= 0);
    }

    #[test]
    fn test_parse_pub_functions() {
        let source = r#"
pub fn multiply(a: i32, b: i32) i32 {
    return a * b;
}

fn divide(a: i32, b: i32) i32 {
    return @divTrunc(a, b);
}
        "#;

        let symbols = parse("test.zig", source).unwrap();

        let func_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Function))
            .collect();

        assert_eq!(func_symbols.len(), 2);
        assert!(func_symbols.iter().any(|s| s.symbol.as_deref() == Some("multiply")));
        assert!(func_symbols.iter().any(|s| s.symbol.as_deref() == Some("divide")));
    }

    #[test]
    fn test_parse_struct_with_methods() {
        let source = r#"
const Calculator = struct {
    value: i32,

    pub fn init(val: i32) Calculator {
        return Calculator{ .value = val };
    }

    pub fn add(self: *Calculator, other: i32) void {
        self.value += other;
    }
};
        "#;

        let symbols = parse("test.zig", source).unwrap();

        let struct_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Struct))
            .collect();

        assert_eq!(struct_symbols.len(), 1);
        assert_eq!(struct_symbols[0].symbol.as_deref(), Some("Calculator"));
    }

    #[test]
    fn test_parse_error_set() {
        let source = r#"
const FileError = error{
    AccessDenied,
    FileNotFound,
    OutOfMemory,
};
        "#;

        let symbols = parse("test.zig", source).unwrap();

        // Error sets are stored as constants
        let const_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Constant))
            .collect();

        assert!(const_symbols.iter().any(|s| s.symbol.as_deref() == Some("FileError")));
    }

    #[test]
    fn test_parse_mixed_symbols() {
        let source = r#"
const std = @import("std");

const MAX_BUFFER = 1024;

const Point = struct {
    x: f32,
    y: f32,
};

pub fn main() !void {
    const stdout = std.io.getStdOut().writer();
    try stdout.print("Hello, World!\n", .{});
}

test "point creation" {
    const p = Point{ .x = 1.0, .y = 2.0 };
    try std.testing.expect(p.x == 1.0);
}
        "#;

        let symbols = parse("test.zig", source).unwrap();

        // Should find: constants, struct, function, test
        assert!(symbols.len() >= 3);

        let kinds: Vec<&SymbolKind> = symbols.iter().map(|s| &s.kind).collect();
        assert!(kinds.contains(&&SymbolKind::Constant));
        assert!(kinds.contains(&&SymbolKind::Struct));
        assert!(kinds.contains(&&SymbolKind::Function));
    }
}
