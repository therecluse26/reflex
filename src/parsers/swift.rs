//! Swift language parser using Tree-sitter
//!
//! Extracts symbols from Swift source code:
//! - Classes
//! - Structs
//! - Enums
//! - Protocols
//! - Functions
//! - Methods
//! - Properties
//! - Extensions
//! - Actors (Swift 5.5+)

use anyhow::{Context, Result};
use streaming_iterator::StreamingIterator;
use tree_sitter::{Parser, Query, QueryCursor};
use crate::models::{Language, SearchResult, Span, SymbolKind};

/// Parse Swift source code and extract symbols
pub fn parse(path: &str, source: &str) -> Result<Vec<SearchResult>> {
    let mut parser = Parser::new();
    let language = tree_sitter_swift::LANGUAGE;

    parser
        .set_language(&language.into())
        .context("Failed to set Swift language")?;

    let tree = parser
        .parse(source, None)
        .context("Failed to parse Swift source")?;

    let root_node = tree.root_node();

    let mut symbols = Vec::new();

    // Extract different types of symbols using Tree-sitter queries
    symbols.extend(extract_classes(source, &root_node, &language.into())?);
    symbols.extend(extract_structs(source, &root_node, &language.into())?);
    symbols.extend(extract_enums(source, &root_node, &language.into())?);
    symbols.extend(extract_protocols(source, &root_node, &language.into())?);
    symbols.extend(extract_extensions(source, &root_node, &language.into())?);
    symbols.extend(extract_functions(source, &root_node, &language.into())?);
    symbols.extend(extract_properties(source, &root_node, &language.into())?);

    // Add file path to all symbols
    for symbol in &mut symbols {
        symbol.path = path.to_string();
        symbol.lang = Language::Swift;
    }

    Ok(symbols)
}

/// Extract class declarations
fn extract_classes(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (class_declaration
            name: (type_identifier) @name) @class
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
        (struct_declaration
            name: (type_identifier) @name) @struct
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
            name: (type_identifier) @name) @enum
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create enum query")?;

    extract_symbols(source, root, &query, SymbolKind::Enum, None)
}

/// Extract protocol declarations
fn extract_protocols(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (protocol_declaration
            name: (type_identifier) @name) @protocol
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create protocol query")?;

    extract_symbols(source, root, &query, SymbolKind::Trait, None)
}

/// Extract extension declarations
fn extract_extensions(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (extension_declaration
            (type_identifier) @name) @extension
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create extension query")?;

    extract_symbols(source, root, &query, SymbolKind::Type, None)
}

/// Extract function and method declarations
fn extract_functions(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (class_declaration
            name: (type_identifier) @class_name
            body: (class_body
                (function_declaration
                    name: (simple_identifier) @method_name))) @class

        (struct_declaration
            name: (type_identifier) @struct_name
            body: (struct_body
                (function_declaration
                    name: (simple_identifier) @method_name))) @struct

        (enum_declaration
            name: (type_identifier) @enum_name
            body: (enum_body
                (function_declaration
                    name: (simple_identifier) @method_name))) @enum

        (extension_declaration
            (type_identifier) @extension_name
            body: (extension_body
                (function_declaration
                    name: (simple_identifier) @method_name))) @extension
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create function query")?;

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
                "enum_name" => {
                    scope_name = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                    scope_type = Some("enum");
                }
                "extension_name" => {
                    scope_name = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                    scope_type = Some("extension");
                }
                "method_name" => {
                    method_name = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                    // Find the parent function_declaration node
                    let mut current = capture.node;
                    while let Some(parent) = current.parent() {
                        if parent.kind() == "function_declaration" {
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
                Language::Swift,
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
            name: (type_identifier) @class_name
            body: (class_body
                (property_declaration
                    (pattern (simple_identifier) @property_name)))) @class

        (struct_declaration
            name: (type_identifier) @struct_name
            body: (struct_body
                (property_declaration
                    (pattern (simple_identifier) @property_name)))) @struct
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
                Language::Swift,
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
                Language::Swift,
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
class User {
    var name: String
    var age: Int
}
        "#;

        let symbols = parse("test.swift", source).unwrap();

        let class_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Class))
            .collect();

        assert_eq!(class_symbols.len(), 1);
        assert_eq!(class_symbols[0].symbol.as_deref(), Some("User"));
    }

    #[test]
    fn test_parse_struct() {
        let source = r#"
struct Point {
    var x: Double
    var y: Double
}
        "#;

        let symbols = parse("test.swift", source).unwrap();

        let struct_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Struct))
            .collect();

        assert_eq!(struct_symbols.len(), 1);
        assert_eq!(struct_symbols[0].symbol.as_deref(), Some("Point"));
    }

    #[test]
    fn test_parse_enum() {
        let source = r#"
enum Status {
    case active
    case inactive
    case pending
}
        "#;

        let symbols = parse("test.swift", source).unwrap();

        let enum_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Enum))
            .collect();

        assert_eq!(enum_symbols.len(), 1);
        assert_eq!(enum_symbols[0].symbol.as_deref(), Some("Status"));
    }

    #[test]
    fn test_parse_protocol() {
        let source = r#"
protocol Drawable {
    func draw()
}
        "#;

        let symbols = parse("test.swift", source).unwrap();

        let protocol_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Trait))
            .collect();

        assert_eq!(protocol_symbols.len(), 1);
        assert_eq!(protocol_symbols[0].symbol.as_deref(), Some("Drawable"));
    }

    #[test]
    fn test_parse_methods() {
        let source = r#"
class Calculator {
    func add(_ a: Int, _ b: Int) -> Int {
        return a + b
    }

    func subtract(_ a: Int, _ b: Int) -> Int {
        return a - b
    }
}
        "#;

        let symbols = parse("test.swift", source).unwrap();

        let method_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Method))
            .collect();

        assert_eq!(method_symbols.len(), 2);
        assert!(method_symbols.iter().any(|s| s.symbol.as_deref() == Some("add")));
        assert!(method_symbols.iter().any(|s| s.symbol.as_deref() == Some("subtract")));

        // Check scope
        for method in method_symbols {
            // Removed: scope field no longer exists: assert_eq!(method.scope.as_ref().unwrap(), "class Calculator");
        }
    }

    #[test]
    fn test_parse_properties() {
        let source = r#"
struct User {
    var name: String
    let id: Int
    var email: String?
}
        "#;

        let symbols = parse("test.swift", source).unwrap();

        let property_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Variable))
            .collect();

        assert_eq!(property_symbols.len(), 3);
        assert!(property_symbols.iter().any(|s| s.symbol.as_deref() == Some("name")));
        assert!(property_symbols.iter().any(|s| s.symbol.as_deref() == Some("id")));
        assert!(property_symbols.iter().any(|s| s.symbol.as_deref() == Some("email")));
    }

    #[test]
    fn test_parse_extension() {
        let source = r#"
extension String {
    func isEmail() -> Bool {
        return self.contains("@")
    }
}
        "#;

        let symbols = parse("test.swift", source).unwrap();

        let extension_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Type))
            .collect();

        assert!(extension_symbols.len() >= 1);
    }

    #[test]
    fn test_parse_ios_view_controller() {
        let source = r#"
class ViewController: UIViewController {
    override func viewDidLoad() {
        super.viewDidLoad()
    }

    override func viewWillAppear(_ animated: Bool) {
        super.viewWillAppear(animated)
    }
}
        "#;

        let symbols = parse("test.swift", source).unwrap();

        let class_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Class))
            .collect();

        let method_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Method))
            .collect();

        assert_eq!(class_symbols.len(), 1);
        assert_eq!(method_symbols.len(), 2);
        assert!(method_symbols.iter().any(|s| s.symbol.as_deref() == Some("viewDidLoad")));
        assert!(method_symbols.iter().any(|s| s.symbol.as_deref() == Some("viewWillAppear")));
    }

    #[test]
    fn test_parse_swiftui_view() {
        let source = r#"
struct ContentView: View {
    var body: some View {
        Text("Hello, World!")
    }
}
        "#;

        let symbols = parse("test.swift", source).unwrap();

        let struct_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Struct))
            .collect();

        assert_eq!(struct_symbols.len(), 1);
        assert_eq!(struct_symbols[0].symbol.as_deref(), Some("ContentView"));
    }

    #[test]
    fn test_parse_mixed_symbols() {
        let source = r#"
protocol Repository {
    func save()
}

class User {
    var name: String

    init(name: String) {
        self.name = name
    }
}

enum Status {
    case active, inactive
}

extension User: Repository {
    func save() {
        // implementation
    }
}
        "#;

        let symbols = parse("test.swift", source).unwrap();

        // Should find: protocol, class, enum, extension, methods, properties
        assert!(symbols.len() >= 5);

        let kinds: Vec<&SymbolKind> = symbols.iter().map(|s| &s.kind).collect();
        assert!(kinds.contains(&&SymbolKind::Trait));
        assert!(kinds.contains(&&SymbolKind::Class));
        assert!(kinds.contains(&&SymbolKind::Enum));
        assert!(kinds.contains(&&SymbolKind::Type));
    }
}
