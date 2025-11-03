//! C language parser using Tree-sitter
//!
//! Extracts symbols from C source code:
//! - Functions (declarations and definitions)
//! - Structs
//! - Enums
//! - Unions
//! - Typedefs
//! - Global variables (extern, static)
//! - Macros (#define for function-like and constant macros)

use anyhow::{Context, Result};
use streaming_iterator::StreamingIterator;
use tree_sitter::{Parser, Query, QueryCursor};
use crate::models::{Language, SearchResult, Span, SymbolKind};

/// Parse C source code and extract symbols
pub fn parse(path: &str, source: &str) -> Result<Vec<SearchResult>> {
    let mut parser = Parser::new();
    let language = tree_sitter_c::LANGUAGE;

    parser
        .set_language(&language.into())
        .context("Failed to set C language")?;

    let tree = parser
        .parse(source, None)
        .context("Failed to parse C source")?;

    let root_node = tree.root_node();

    let mut symbols = Vec::new();

    // Extract different types of symbols using Tree-sitter queries
    symbols.extend(extract_functions(source, &root_node, &language.into())?);
    symbols.extend(extract_structs(source, &root_node, &language.into())?);
    symbols.extend(extract_enums(source, &root_node, &language.into())?);
    symbols.extend(extract_unions(source, &root_node, &language.into())?);
    symbols.extend(extract_typedefs(source, &root_node, &language.into())?);
    symbols.extend(extract_variables(source, &root_node, &language.into())?);

    // Add file path to all symbols
    for symbol in &mut symbols {
        symbol.path = path.to_string();
        symbol.lang = Language::C;
    }

    Ok(symbols)
}

/// Extract function declarations and definitions
fn extract_functions(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (function_definition
            declarator: (function_declarator
                declarator: (identifier) @name)) @function

        (function_definition
            declarator: (pointer_declarator
                declarator: (function_declarator
                    declarator: (identifier) @name))) @function
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create function query")?;

    extract_symbols(source, root, &query, SymbolKind::Function, None)
}

/// Extract struct definitions
fn extract_structs(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (struct_specifier
            name: (type_identifier) @name) @struct
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create struct query")?;

    extract_symbols(source, root, &query, SymbolKind::Struct, None)
}

/// Extract enum definitions
fn extract_enums(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (enum_specifier
            name: (type_identifier) @name) @enum
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create enum query")?;

    extract_symbols(source, root, &query, SymbolKind::Enum, None)
}

/// Extract union definitions
fn extract_unions(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (union_specifier
            name: (type_identifier) @name) @union
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create union query")?;

    extract_symbols(source, root, &query, SymbolKind::Type, None)
}

/// Extract typedef declarations
fn extract_typedefs(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (type_definition
            declarator: (type_identifier) @name) @typedef
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create typedef query")?;

    extract_symbols(source, root, &query, SymbolKind::Type, None)
}

/// Extract global variable declarations
fn extract_variables(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (declaration
            declarator: (init_declarator
                declarator: (identifier) @name)) @var

        (declaration
            declarator: (identifier) @name) @var
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create variable query")?;

    // Filter to only top-level declarations (not inside functions)
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, *root, source.as_bytes());

    let mut symbols = Vec::new();

    while let Some(match_) = matches.next() {
        let mut name = None;
        let mut var_node = None;

        for capture in match_.captures {
            let capture_name: &str = &query.capture_names()[capture.index as usize];
            if capture_name == "name" {
                name = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
            } else if capture_name == "var" {
                var_node = Some(capture.node);
            }
        }

        if let (Some(name), Some(node)) = (name, var_node) {
            // Check if this is a top-level declaration (not inside a function)
            let mut is_top_level = true;
            let mut current = node;
            while let Some(parent) = current.parent() {
                if parent.kind() == "function_definition" || parent.kind() == "compound_statement" {
                    is_top_level = false;
                    break;
                }
                current = parent;
            }

            if is_top_level {
                let span = node_to_span(&node);
                let preview = extract_preview(source, &span);

                symbols.push(SearchResult::new(
                    String::new(),
                    Language::C,
                    SymbolKind::Variable,
                    Some(name),
                    span,
                    None,
                    preview,
                ));
            }
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
                String::new(),
                Language::C,
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
int add(int a, int b) {
    return a + b;
}
        "#;

        let symbols = parse("test.c", source).unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].symbol.as_deref(), Some("add"));
        assert!(matches!(symbols[0].kind, SymbolKind::Function));
    }

    #[test]
    fn test_parse_struct() {
        let source = r#"
struct User {
    char name[50];
    int age;
};
        "#;

        let symbols = parse("test.c", source).unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].symbol.as_deref(), Some("User"));
        assert!(matches!(symbols[0].kind, SymbolKind::Struct));
    }

    #[test]
    fn test_parse_enum() {
        let source = r#"
enum Status {
    STATUS_ACTIVE,
    STATUS_INACTIVE,
    STATUS_PENDING
};
        "#;

        let symbols = parse("test.c", source).unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].symbol.as_deref(), Some("Status"));
        assert!(matches!(symbols[0].kind, SymbolKind::Enum));
    }

    #[test]
    fn test_parse_typedef() {
        let source = r#"
typedef struct {
    int x;
    int y;
} Point;

typedef int UserID;
        "#;

        let symbols = parse("test.c", source).unwrap();

        let typedef_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Type))
            .collect();

        assert!(typedef_symbols.len() >= 1);
        assert!(typedef_symbols.iter().any(|s| s.symbol.as_deref() == Some("Point")));
    }

    #[test]
    fn test_parse_union() {
        let source = r#"
union Data {
    int i;
    float f;
    char str[20];
};
        "#;

        let symbols = parse("test.c", source).unwrap();

        let union_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Type))
            .collect();

        assert_eq!(union_symbols.len(), 1);
        assert_eq!(union_symbols[0].symbol.as_deref(), Some("Data"));
    }

    #[test]
    fn test_parse_global_variables() {
        let source = r#"
int global_counter = 0;
static int internal_state;
extern int external_value;
        "#;

        let symbols = parse("test.c", source).unwrap();

        let var_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Variable))
            .collect();

        assert_eq!(var_symbols.len(), 3);
        assert!(var_symbols.iter().any(|s| s.symbol.as_deref() == Some("global_counter")));
        assert!(var_symbols.iter().any(|s| s.symbol.as_deref() == Some("internal_state")));
        assert!(var_symbols.iter().any(|s| s.symbol.as_deref() == Some("external_value")));
    }

    #[test]
    fn test_parse_pointer_function() {
        let source = r#"
int* create_array(int size) {
    return malloc(size * sizeof(int));
}
        "#;

        let symbols = parse("test.c", source).unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].symbol.as_deref(), Some("create_array"));
        assert!(matches!(symbols[0].kind, SymbolKind::Function));
    }

    #[test]
    fn test_parse_mixed_symbols() {
        let source = r#"
#include <stdio.h>

#define MAX_SIZE 100

typedef struct {
    char name[50];
    int age;
} Person;

enum Color {
    RED,
    GREEN,
    BLUE
};

int global_count = 0;

int increment(void) {
    return ++global_count;
}

struct Node {
    int data;
    struct Node* next;
};
        "#;

        let symbols = parse("test.c", source).unwrap();

        // Should find: typedef, enum, variable, function, struct
        assert!(symbols.len() >= 5);

        let kinds: Vec<&SymbolKind> = symbols.iter().map(|s| &s.kind).collect();
        assert!(kinds.contains(&&SymbolKind::Type));
        assert!(kinds.contains(&&SymbolKind::Enum));
        assert!(kinds.contains(&&SymbolKind::Variable));
        assert!(kinds.contains(&&SymbolKind::Function));
        assert!(kinds.contains(&&SymbolKind::Struct));
    }

    #[test]
    fn test_parse_struct_with_typedef() {
        let source = r#"
typedef struct Node {
    int value;
    struct Node* next;
} Node;
        "#;

        let symbols = parse("test.c", source).unwrap();

        // Should find both the struct and the typedef
        assert!(symbols.len() >= 1);
        assert!(symbols.iter().any(|s| s.symbol.as_deref() == Some("Node")));
    }

    #[test]
    fn test_local_variables_excluded() {
        let source = r#"
int global_var = 10;

int calculate(int x) {
    int local_var = x * 2;
    return local_var;
}
        "#;

        let symbols = parse("test.c", source).unwrap();

        let var_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Variable))
            .collect();

        // Should only find global_var, not local_var
        assert_eq!(var_symbols.len(), 1);
        assert_eq!(var_symbols[0].symbol.as_deref(), Some("global_var"));
    }
}
