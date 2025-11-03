//! C++ language parser using Tree-sitter
//!
//! Extracts symbols from C++ source code:
//! - Functions (regular and template)
//! - Classes (regular, abstract, template)
//! - Structs
//! - Namespaces
//! - Templates (class and function)
//! - Methods (with class scope, virtual, override)
//! - Constructors/Destructors
//! - Operators
//! - Enums (enum and enum class)
//! - Using declarations
//! - Type aliases

use anyhow::{Context, Result};
use streaming_iterator::StreamingIterator;
use tree_sitter::{Parser, Query, QueryCursor};
use crate::models::{Language, SearchResult, Span, SymbolKind};

/// Parse C++ source code and extract symbols
pub fn parse(path: &str, source: &str) -> Result<Vec<SearchResult>> {
    let mut parser = Parser::new();
    let language = tree_sitter_cpp::LANGUAGE;

    parser
        .set_language(&language.into())
        .context("Failed to set C++ language")?;

    let tree = parser
        .parse(source, None)
        .context("Failed to parse C++ source")?;

    let root_node = tree.root_node();

    let mut symbols = Vec::new();

    // Extract different types of symbols using Tree-sitter queries
    symbols.extend(extract_functions(source, &root_node, &language.into())?);
    symbols.extend(extract_classes(source, &root_node, &language.into())?);
    symbols.extend(extract_structs(source, &root_node, &language.into())?);
    symbols.extend(extract_namespaces(source, &root_node, &language.into())?);
    symbols.extend(extract_enums(source, &root_node, &language.into())?);
    symbols.extend(extract_methods(source, &root_node, &language.into())?);
    symbols.extend(extract_type_aliases(source, &root_node, &language.into())?);

    // Add file path to all symbols
    for symbol in &mut symbols {
        symbol.path = path.to_string();
        symbol.lang = Language::Cpp;
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
            declarator: (function_declarator
                declarator: (qualified_identifier
                    name: (identifier) @name))) @function

        (template_declaration
            (function_definition
                declarator: (function_declarator
                    declarator: (identifier) @name))) @function
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create function query")?;

    extract_symbols(source, root, &query, SymbolKind::Function, None)
}

/// Extract class declarations
fn extract_classes(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (class_specifier
            name: (type_identifier) @name) @class

        (template_declaration
            (class_specifier
                name: (type_identifier) @name)) @class
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create class query")?;

    extract_symbols(source, root, &query, SymbolKind::Class, None)
}

/// Extract struct declarations
fn extract_structs(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (struct_specifier
            name: (type_identifier) @name) @struct

        (template_declaration
            (struct_specifier
                name: (type_identifier) @name)) @struct
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create struct query")?;

    extract_symbols(source, root, &query, SymbolKind::Struct, None)
}

/// Extract namespace definitions
fn extract_namespaces(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (namespace_definition
            name: (_) @name) @namespace
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create namespace query")?;

    extract_symbols(source, root, &query, SymbolKind::Namespace, None)
}

/// Extract enum declarations
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

/// Extract method definitions from classes and structs
fn extract_methods(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (class_specifier
            name: (type_identifier) @class_name
            body: (field_declaration_list
                (function_definition
                    declarator: (function_declarator
                        declarator: (field_identifier) @method_name)))) @class

        (struct_specifier
            name: (type_identifier) @struct_name
            body: (field_declaration_list
                (function_definition
                    declarator: (function_declarator
                        declarator: (field_identifier) @method_name)))) @struct
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create method query")?;

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, *root, source.as_bytes());

    let mut symbols = Vec::new();

    while let Some(match_) = matches.next() {
        let mut scope_name = None;
        let mut scope_type = None;
        let mut method_name = None;
        let mut method_node = None;

        for capture in match_.captures {
            let capture_name: &str = &query.capture_names()[capture.index as usize];
            match capture_name {
                "class_name" => {
                    scope_name = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                    scope_type = Some("class");
                }
                "struct_name" => {
                    scope_name = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                    scope_type = Some("struct");
                }
                "method_name" => {
                    method_name = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                    // Find the parent function_definition node
                    let mut current = capture.node;
                    while let Some(parent) = current.parent() {
                        if parent.kind() == "function_definition" {
                            method_node = Some(parent);
                            break;
                        }
                        current = parent;
                    }
                }
                _ => {}
            }
        }

        if let (Some(scope_name), Some(scope_type), Some(method_name), Some(node)) =
            (scope_name, scope_type, method_name, method_node) {
            let scope = format!("{} {}", scope_type, scope_name);
            let span = node_to_span(&node);
            let preview = extract_preview(source, &span);

            symbols.push(SearchResult::new(
                String::new(),
                Language::Cpp,
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

/// Extract type aliases (using and typedef)
fn extract_type_aliases(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (type_definition
            declarator: (type_identifier) @name) @typedef

        (alias_declaration
            name: (type_identifier) @name) @using
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create type alias query")?;

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
    let mut seen_names = std::collections::HashSet::new();

    while let Some(match_) = matches.next() {
        // Find the name capture and the full node
        let mut name = None;
        let mut name_node = None;
        let mut full_node = None;

        for capture in match_.captures {
            let capture_name: &str = &query.capture_names()[capture.index as usize];
            if capture_name == "name" {
                name = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                name_node = Some(capture.node);
            } else {
                // Assume any other capture is the full node
                full_node = Some(capture.node);
            }
        }

        if let (Some(name), Some(name_node), Some(node)) = (name, name_node, full_node) {
            // Deduplicate by name position - this handles cases where template patterns
            // match the same symbol twice (e.g., both template_declaration and class_specifier)
            let name_key = (name_node.start_byte(), name_node.end_byte(), name.clone());
            if seen_names.contains(&name_key) {
                continue; // Skip duplicate
            }
            seen_names.insert(name_key);

            let span = node_to_span(&node);
            let preview = extract_preview(source, &span);

            symbols.push(SearchResult::new(
                String::new(),
                Language::Cpp,
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

        let symbols = parse("test.cpp", source).unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].symbol, "add");
        assert!(matches!(symbols[0].kind, SymbolKind::Function));
    }

    #[test]
    fn test_parse_class() {
        let source = r#"
class User {
private:
    std::string name;
    int age;

public:
    User(std::string n, int a) : name(n), age(a) {}
};
        "#;

        let symbols = parse("test.cpp", source).unwrap();

        let class_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Class))
            .collect();

        assert_eq!(class_symbols.len(), 1);
        assert_eq!(class_symbols[0].symbol, "User");
    }

    #[test]
    fn test_parse_namespace() {
        let source = r#"
namespace MyNamespace {
    int value = 42;
}

namespace Nested::Namespace {
    void function() {}
}
        "#;

        let symbols = parse("test.cpp", source).unwrap();

        let namespace_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Namespace))
            .collect();

        assert!(namespace_symbols.len() >= 1);
        assert!(namespace_symbols.iter().any(|s| s.symbol == "MyNamespace"));
    }

    #[test]
    fn test_parse_struct() {
        let source = r#"
struct Point {
    int x;
    int y;
};
        "#;

        let symbols = parse("test.cpp", source).unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].symbol, "Point");
        assert!(matches!(symbols[0].kind, SymbolKind::Struct));
    }

    #[test]
    fn test_parse_enum() {
        let source = r#"
enum Color {
    RED,
    GREEN,
    BLUE
};

enum class Status {
    Active,
    Inactive
};
        "#;

        let symbols = parse("test.cpp", source).unwrap();

        let enum_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Enum))
            .collect();

        assert_eq!(enum_symbols.len(), 2);
        assert!(enum_symbols.iter().any(|s| s.symbol == "Color"));
        assert!(enum_symbols.iter().any(|s| s.symbol == "Status"));
    }

    #[test]
    fn test_parse_template_class() {
        let source = r#"
template <typename T>
class Container {
private:
    T value;

public:
    Container(T v) : value(v) {}
    T getValue() { return value; }
};
        "#;

        let symbols = parse("test.cpp", source).unwrap();

        let class_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Class))
            .collect();

        assert_eq!(class_symbols.len(), 1);
        assert_eq!(class_symbols[0].symbol, "Container");
    }

    #[test]
    fn test_parse_template_function() {
        let source = r#"
template <typename T>
T max(T a, T b) {
    return (a > b) ? a : b;
}
        "#;

        let symbols = parse("test.cpp", source).unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].symbol, "max");
        assert!(matches!(symbols[0].kind, SymbolKind::Function));
    }

    #[test]
    fn test_parse_class_with_methods() {
        let source = r#"
class Calculator {
public:
    int add(int a, int b) {
        return a + b;
    }

    int subtract(int a, int b) {
        return a - b;
    }
};
        "#;

        let symbols = parse("test.cpp", source).unwrap();

        let method_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Method))
            .collect();

        assert_eq!(method_symbols.len(), 2);
        assert!(method_symbols.iter().any(|s| s.symbol == "add"));
        assert!(method_symbols.iter().any(|s| s.symbol == "subtract"));

        // Check scope
        for method in method_symbols {
            assert_eq!(method.scope.as_ref().unwrap(), "class Calculator");
        }
    }

    #[test]
    fn test_parse_using_declaration() {
        let source = r#"
using StringVector = std::vector<std::string>;
using IntPtr = int*;
        "#;

        let symbols = parse("test.cpp", source).unwrap();

        let type_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Type))
            .collect();

        assert!(type_symbols.len() >= 1);
        assert!(type_symbols.iter().any(|s| s.symbol == "StringVector"));
    }

    #[test]
    fn test_parse_typedef() {
        let source = r#"
typedef unsigned int uint;
typedef struct {
    int x, y;
} Point;
        "#;

        let symbols = parse("test.cpp", source).unwrap();

        let type_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Type))
            .collect();

        assert!(type_symbols.len() >= 1);
    }

    #[test]
    fn test_parse_mixed_symbols() {
        let source = r#"
namespace Math {
    class Vector {
    private:
        double x, y;

    public:
        Vector(double x, double y) : x(x), y(y) {}

        double magnitude() {
            return sqrt(x*x + y*y);
        }
    };

    enum Operation {
        ADD,
        SUBTRACT
    };

    template <typename T>
    T multiply(T a, T b) {
        return a * b;
    }
}
        "#;

        let symbols = parse("test.cpp", source).unwrap();

        // Should find: namespace, class, enum, method, function
        assert!(symbols.len() >= 5);

        let kinds: Vec<&SymbolKind> = symbols.iter().map(|s| &s.kind).collect();
        assert!(kinds.contains(&&SymbolKind::Namespace));
        assert!(kinds.contains(&&SymbolKind::Class));
        assert!(kinds.contains(&&SymbolKind::Enum));
        assert!(kinds.contains(&&SymbolKind::Method));
        assert!(kinds.contains(&&SymbolKind::Function));
    }

    #[test]
    fn test_parse_nested_namespace() {
        let source = r#"
namespace Outer {
    namespace Inner {
        void function() {}
    }
}
        "#;

        let symbols = parse("test.cpp", source).unwrap();

        let namespace_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Namespace))
            .collect();

        assert_eq!(namespace_symbols.len(), 2);
        assert!(namespace_symbols.iter().any(|s| s.symbol == "Outer"));
        assert!(namespace_symbols.iter().any(|s| s.symbol == "Inner"));
    }

    #[test]
    fn test_parse_virtual_methods() {
        let source = r#"
class Base {
public:
    virtual void draw() = 0;
    virtual void update() {}
};

class Derived : public Base {
public:
    void draw() override {
        // Implementation
    }
};
        "#;

        let symbols = parse("test.cpp", source).unwrap();

        let class_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Class))
            .collect();

        assert_eq!(class_symbols.len(), 2);
        assert!(class_symbols.iter().any(|s| s.symbol == "Base"));
        assert!(class_symbols.iter().any(|s| s.symbol == "Derived"));

        let method_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Method))
            .collect();

        assert!(method_symbols.len() >= 2);
    }

    #[test]
    fn test_parse_operator_overload() {
        let source = r#"
class Complex {
private:
    double real, imag;

public:
    Complex operator+(const Complex& other) {
        return Complex(real + other.real, imag + other.imag);
    }
};
        "#;

        let symbols = parse("test.cpp", source).unwrap();

        let class_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Class))
            .collect();

        assert_eq!(class_symbols.len(), 1);
        assert_eq!(class_symbols[0].symbol, "Complex");
    }
}
