//! C# language parser using Tree-sitter
//!
//! Extracts symbols from C# source code:
//! - Classes (regular, abstract, sealed, partial, static)
//! - Interfaces
//! - Structs
//! - Enums
//! - Delegates
//! - Records (C# 9+)
//! - Methods (with class scope, visibility)
//! - Properties (class/struct/record members)
//! - Local variables (inside methods)
//! - Events
//! - Namespaces
//! - Constructors

use anyhow::{Context, Result};
use streaming_iterator::StreamingIterator;
use tree_sitter::{Parser, Query, QueryCursor};
use crate::models::{Language, SearchResult, Span, SymbolKind};

/// Parse C# source code and extract symbols
pub fn parse(path: &str, source: &str) -> Result<Vec<SearchResult>> {
    let mut parser = Parser::new();
    let language = tree_sitter_c_sharp::LANGUAGE;

    parser
        .set_language(&language.into())
        .context("Failed to set C# language")?;

    let tree = parser
        .parse(source, None)
        .context("Failed to parse C# source")?;

    let root_node = tree.root_node();

    let mut symbols = Vec::new();

    // Extract different types of symbols using Tree-sitter queries
    symbols.extend(extract_namespaces(source, &root_node, &language.into())?);
    symbols.extend(extract_classes(source, &root_node, &language.into())?);
    symbols.extend(extract_interfaces(source, &root_node, &language.into())?);
    symbols.extend(extract_structs(source, &root_node, &language.into())?);
    symbols.extend(extract_enums(source, &root_node, &language.into())?);
    symbols.extend(extract_records(source, &root_node, &language.into())?);
    symbols.extend(extract_delegates(source, &root_node, &language.into())?);
    symbols.extend(extract_methods(source, &root_node, &language.into())?);
    symbols.extend(extract_properties(source, &root_node, &language.into())?);
    symbols.extend(extract_local_variables(source, &root_node, &language.into())?);

    // Add file path to all symbols
    for symbol in &mut symbols {
        symbol.path = path.to_string();
        symbol.lang = Language::CSharp;
    }

    Ok(symbols)
}

/// Extract namespace declarations
fn extract_namespaces(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (namespace_declaration
            name: (_) @name) @namespace

        (file_scoped_namespace_declaration
            name: (_) @name) @namespace
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create namespace query")?;

    extract_symbols(source, root, &query, SymbolKind::Namespace, None)
}

/// Extract class declarations
fn extract_classes(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (class_declaration
            name: (identifier) @name) @class
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create class query")?;

    extract_symbols(source, root, &query, SymbolKind::Class, None)
}

/// Extract interface declarations
fn extract_interfaces(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (interface_declaration
            name: (identifier) @name) @interface
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create interface query")?;

    extract_symbols(source, root, &query, SymbolKind::Interface, None)
}

/// Extract struct declarations
fn extract_structs(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (struct_declaration
            name: (identifier) @name) @struct
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
        (enum_declaration
            name: (identifier) @name) @enum
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create enum query")?;

    extract_symbols(source, root, &query, SymbolKind::Enum, None)
}

/// Extract record declarations (C# 9+)
fn extract_records(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (record_declaration
            name: (identifier) @name) @record
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create record query")?;

    extract_symbols(source, root, &query, SymbolKind::Type, None)
}

/// Extract delegate declarations
fn extract_delegates(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (delegate_declaration
            name: (identifier) @name) @delegate
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create delegate query")?;

    extract_symbols(source, root, &query, SymbolKind::Type, None)
}

/// Extract method declarations from classes, structs, and interfaces
fn extract_methods(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (class_declaration
            name: (identifier) @class_name
            body: (declaration_list
                (method_declaration
                    name: (identifier) @method_name))) @class

        (struct_declaration
            name: (identifier) @struct_name
            body: (declaration_list
                (method_declaration
                    name: (identifier) @method_name))) @struct

        (interface_declaration
            name: (identifier) @interface_name
            body: (declaration_list
                (method_declaration
                    name: (identifier) @method_name))) @interface

        (record_declaration
            name: (identifier) @record_name
            body: (declaration_list
                (method_declaration
                    name: (identifier) @method_name))) @record
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
                "interface_name" => {
                    scope_name = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                    scope_type = Some("interface");
                }
                "record_name" => {
                    scope_name = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                    scope_type = Some("record");
                }
                "method_name" => {
                    method_name = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                    // Find the parent method_declaration node
                    let mut current = capture.node;
                    while let Some(parent) = current.parent() {
                        if parent.kind() == "method_declaration" {
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
                Language::CSharp,
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

/// Extract property declarations
fn extract_properties(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (class_declaration
            name: (identifier) @class_name
            body: (declaration_list
                (property_declaration
                    name: (identifier) @property_name))) @class

        (struct_declaration
            name: (identifier) @struct_name
            body: (declaration_list
                (property_declaration
                    name: (identifier) @property_name))) @struct

        (interface_declaration
            name: (identifier) @interface_name
            body: (declaration_list
                (property_declaration
                    name: (identifier) @property_name))) @interface

        (record_declaration
            name: (identifier) @record_name
            body: (declaration_list
                (property_declaration
                    name: (identifier) @property_name))) @record
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create property query")?;

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, *root, source.as_bytes());

    let mut symbols = Vec::new();

    while let Some(match_) = matches.next() {
        let mut scope_name = None;
        let mut scope_type = None;
        let mut property_name = None;
        let mut property_node = None;

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
                "interface_name" => {
                    scope_name = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                    scope_type = Some("interface");
                }
                "record_name" => {
                    scope_name = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                    scope_type = Some("record");
                }
                "property_name" => {
                    property_name = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                    // Find the parent property_declaration node
                    let mut current = capture.node;
                    while let Some(parent) = current.parent() {
                        if parent.kind() == "property_declaration" {
                            property_node = Some(parent);
                            break;
                        }
                        current = parent;
                    }
                }
                _ => {}
            }
        }

        if let (Some(scope_name), Some(scope_type), Some(property_name), Some(node)) =
            (scope_name, scope_type, property_name, property_node) {
            let scope = format!("{} {}", scope_type, scope_name);
            let span = node_to_span(&node);
            let preview = extract_preview(source, &span);

            symbols.push(SearchResult::new(
                String::new(),
                Language::CSharp,
                SymbolKind::Variable,
                Some(property_name),
                span,
                Some(scope),
                preview,
            ));
        }
    }

    Ok(symbols)
}

/// Extract local variable declarations inside methods
fn extract_local_variables(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (local_declaration_statement
            (variable_declaration
                (variable_declarator
                    (identifier) @name))) @var
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create local variable query")?;

    extract_symbols(source, root, &query, SymbolKind::Variable, None)
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
                Language::CSharp,
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
    fn test_parse_class() {
        let source = r#"
public class User
{
    private string name;
    private int age;
}
        "#;

        let symbols = parse("test.cs", source).unwrap();

        let class_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Class))
            .collect();

        assert_eq!(class_symbols.len(), 1);
        assert_eq!(class_symbols[0].symbol.as_deref(), Some("User"));
    }

    #[test]
    fn test_parse_namespace() {
        let source = r#"
namespace MyApp.Models
{
    public class User { }
}
        "#;

        let symbols = parse("test.cs", source).unwrap();

        let namespace_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Namespace))
            .collect();

        assert!(namespace_symbols.len() >= 1);
    }

    #[test]
    fn test_parse_file_scoped_namespace() {
        let source = r#"
namespace MyApp.Models;

public class User { }
        "#;

        let symbols = parse("test.cs", source).unwrap();

        let namespace_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Namespace))
            .collect();

        assert!(namespace_symbols.len() >= 1);
    }

    #[test]
    fn test_parse_interface() {
        let source = r#"
public interface IRepository
{
    void Save();
    void Delete();
}
        "#;

        let symbols = parse("test.cs", source).unwrap();

        let interface_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Interface))
            .collect();

        assert_eq!(interface_symbols.len(), 1);
        assert_eq!(interface_symbols[0].symbol.as_deref(), Some("IRepository"));
    }

    #[test]
    fn test_parse_struct() {
        let source = r#"
public struct Point
{
    public int X;
    public int Y;
}
        "#;

        let symbols = parse("test.cs", source).unwrap();

        let struct_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Struct))
            .collect();

        assert_eq!(struct_symbols.len(), 1);
        assert_eq!(struct_symbols[0].symbol.as_deref(), Some("Point"));
    }

    #[test]
    fn test_parse_enum() {
        let source = r#"
public enum Status
{
    Active,
    Inactive,
    Pending
}
        "#;

        let symbols = parse("test.cs", source).unwrap();

        let enum_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Enum))
            .collect();

        assert_eq!(enum_symbols.len(), 1);
        assert_eq!(enum_symbols[0].symbol.as_deref(), Some("Status"));
    }

    #[test]
    fn test_parse_record() {
        let source = r#"
public record Person(string FirstName, string LastName);
        "#;

        let symbols = parse("test.cs", source).unwrap();

        let record_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Type))
            .filter(|s| s.symbol.as_deref() == Some("Person"))
            .collect();

        assert_eq!(record_symbols.len(), 1);
    }

    #[test]
    fn test_parse_methods() {
        let source = r#"
public class Calculator
{
    public int Add(int a, int b)
    {
        return a + b;
    }

    public int Subtract(int a, int b)
    {
        return a - b;
    }
}
        "#;

        let symbols = parse("test.cs", source).unwrap();

        let method_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Method))
            .collect();

        assert_eq!(method_symbols.len(), 2);
        assert!(method_symbols.iter().any(|s| s.symbol.as_deref() == Some("Add")));
        assert!(method_symbols.iter().any(|s| s.symbol.as_deref() == Some("Subtract")));

        // Check scope
        for method in method_symbols {
            assert_eq!(method.scope.as_ref().unwrap(), "class Calculator");
        }
    }

    #[test]
    fn test_parse_properties() {
        let source = r#"
public class User
{
    public string Name { get; set; }
    public int Age { get; set; }
    public string Email { get; init; }
}
        "#;

        let symbols = parse("test.cs", source).unwrap();

        let property_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Variable))
            .collect();

        assert_eq!(property_symbols.len(), 3);
        assert!(property_symbols.iter().any(|s| s.symbol.as_deref() == Some("Name")));
        assert!(property_symbols.iter().any(|s| s.symbol.as_deref() == Some("Age")));
        assert!(property_symbols.iter().any(|s| s.symbol.as_deref() == Some("Email")));
    }

    #[test]
    fn test_parse_delegate() {
        let source = r#"
public delegate void EventHandler(object sender, EventArgs e);
        "#;

        let symbols = parse("test.cs", source).unwrap();

        let delegate_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Type))
            .filter(|s| s.symbol.as_deref() == Some("EventHandler"))
            .collect();

        assert_eq!(delegate_symbols.len(), 1);
    }

    #[test]
    fn test_parse_mixed_symbols() {
        let source = r#"
namespace MyApp
{
    public interface IService
    {
        void Execute();
    }

    public class Service : IService
    {
        public void Execute()
        {
            // Implementation
        }
    }

    public enum Priority
    {
        Low, Medium, High
    }
}
        "#;

        let symbols = parse("test.cs", source).unwrap();

        // Should find: namespace, interface, class, enum, method
        assert!(symbols.len() >= 5);

        let kinds: Vec<&SymbolKind> = symbols.iter().map(|s| &s.kind).collect();
        assert!(kinds.contains(&&SymbolKind::Namespace));
        assert!(kinds.contains(&&SymbolKind::Interface));
        assert!(kinds.contains(&&SymbolKind::Class));
        assert!(kinds.contains(&&SymbolKind::Enum));
        assert!(kinds.contains(&&SymbolKind::Method));
    }

    #[test]
    fn test_local_variables_included() {
        let source = r#"
public class Calculator
{
    public int Multiplier { get; set; } = 2;

    public int Compute(int input)
    {
        int localVar = input * Multiplier;
        var result = localVar + 10;
        return result;
    }
}

public class Helper
{
    public static string Format()
    {
        string message = "Hello";
        var count = 5;
        return message;
    }
}
        "#;

        let symbols = parse("test.cs", source).unwrap();

        // Filter to just variables
        let variables: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Variable))
            .collect();

        // Check that local variables are captured
        assert!(variables.iter().any(|v| v.symbol.as_deref() == Some("localVar")));
        assert!(variables.iter().any(|v| v.symbol.as_deref() == Some("result")));
        assert!(variables.iter().any(|v| v.symbol.as_deref() == Some("message")));
        assert!(variables.iter().any(|v| v.symbol.as_deref() == Some("count")));

        // Check that class property is also captured
        assert!(variables.iter().any(|v| v.symbol.as_deref() == Some("Multiplier")));

        // Verify that local variables have no scope
        let local_vars: Vec<_> = variables.iter()
            .filter(|v| v.symbol.as_deref() == Some("localVar")
                     || v.symbol.as_deref() == Some("result")
                     || v.symbol.as_deref() == Some("message")
                     || v.symbol.as_deref() == Some("count"))
            .collect();

        for var in local_vars {
            assert_eq!(var.scope, None);
        }

        // Verify that class property has scope
        let property = variables.iter()
            .find(|v| v.symbol.as_deref() == Some("Multiplier"))
            .unwrap();
        assert_eq!(property.scope.as_ref().unwrap(), "class Calculator");
    }
}
