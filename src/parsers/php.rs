//! PHP language parser using Tree-sitter
//!
//! Extracts symbols from PHP source code:
//! - Functions
//! - Classes (regular, abstract, final)
//! - Interfaces
//! - Traits
//! - Methods (with class/trait scope)
//! - Properties (public, protected, private)
//! - Constants (class and global)
//! - Namespaces
//! - Enums (PHP 8.1+)

use anyhow::{Context, Result};
use streaming_iterator::StreamingIterator;
use tree_sitter::{Parser, Query, QueryCursor};
use crate::models::{Language, SearchResult, Span, SymbolKind};

/// Parse PHP source code and extract symbols
pub fn parse(path: &str, source: &str) -> Result<Vec<SearchResult>> {
    let mut parser = Parser::new();
    let language = tree_sitter_php::LANGUAGE_PHP;

    parser
        .set_language(&language.into())
        .context("Failed to set PHP language")?;

    let tree = parser
        .parse(source, None)
        .context("Failed to parse PHP source")?;

    let root_node = tree.root_node();

    let mut symbols = Vec::new();

    // Extract different types of symbols using Tree-sitter queries
    symbols.extend(extract_functions(source, &root_node, &language.into())?);
    symbols.extend(extract_classes(source, &root_node, &language.into())?);
    symbols.extend(extract_interfaces(source, &root_node, &language.into())?);
    symbols.extend(extract_traits(source, &root_node, &language.into())?);
    symbols.extend(extract_methods(source, &root_node, &language.into())?);
    symbols.extend(extract_properties(source, &root_node, &language.into())?);
    symbols.extend(extract_constants(source, &root_node, &language.into())?);
    symbols.extend(extract_namespaces(source, &root_node, &language.into())?);
    symbols.extend(extract_enums(source, &root_node, &language.into())?);

    // Add file path to all symbols
    for symbol in &mut symbols {
        symbol.path = path.to_string();
        symbol.lang = Language::PHP;
    }

    Ok(symbols)
}

/// Extract function definitions
fn extract_functions(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (function_definition
            name: (name) @name) @function
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create function query")?;

    extract_symbols(source, root, &query, SymbolKind::Function, None)
}

/// Extract class declarations (including abstract and final classes)
fn extract_classes(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (class_declaration
            name: (name) @name) @class
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
            name: (name) @name) @interface
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create interface query")?;

    extract_symbols(source, root, &query, SymbolKind::Interface, None)
}

/// Extract trait declarations
fn extract_traits(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (trait_declaration
            name: (name) @name) @trait
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create trait query")?;

    extract_symbols(source, root, &query, SymbolKind::Trait, None)
}

/// Extract method definitions from classes, traits, and interfaces
fn extract_methods(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (class_declaration
            name: (name) @class_name
            body: (declaration_list
                (method_declaration
                    name: (name) @method_name))) @class

        (trait_declaration
            name: (name) @trait_name
            body: (declaration_list
                (method_declaration
                    name: (name) @method_name))) @trait

        (interface_declaration
            name: (name) @interface_name
            body: (declaration_list
                (method_declaration
                    name: (name) @method_name))) @interface
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
                "trait_name" => {
                    scope_name = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                    scope_type = Some("trait");
                }
                "interface_name" => {
                    scope_name = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                    scope_type = Some("interface");
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
                Language::PHP,
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

/// Extract property declarations from classes and traits
fn extract_properties(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (class_declaration
            name: (name) @class_name
            body: (declaration_list
                (property_declaration
                    (property_element
                        (variable_name
                            (name) @prop_name))))) @class

        (trait_declaration
            name: (name) @trait_name
            body: (declaration_list
                (property_declaration
                    (property_element
                        (variable_name
                            (name) @prop_name))))) @trait
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create property query")?;

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, *root, source.as_bytes());

    let mut symbols = Vec::new();

    while let Some(match_) = matches.next() {
        let mut scope_name = None;
        let mut scope_type = None;
        let mut prop_name = None;
        let mut prop_node = None;

        for capture in match_.captures {
            let capture_name: &str = &query.capture_names()[capture.index as usize];
            match capture_name {
                "class_name" => {
                    scope_name = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                    scope_type = Some("class");
                }
                "trait_name" => {
                    scope_name = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                    scope_type = Some("trait");
                }
                "prop_name" => {
                    prop_name = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                    // Find the parent property_declaration node
                    let mut current = capture.node;
                    while let Some(parent) = current.parent() {
                        if parent.kind() == "property_declaration" {
                            prop_node = Some(parent);
                            break;
                        }
                        current = parent;
                    }
                }
                _ => {}
            }
        }

        if let (Some(scope_name), Some(scope_type), Some(prop_name), Some(node)) =
            (scope_name, scope_type, prop_name, prop_node) {
            let scope = format!("{} {}", scope_type, scope_name);
            let span = node_to_span(&node);
            let preview = extract_preview(source, &span);

            symbols.push(SearchResult::new(
                String::new(),
                Language::PHP,
                SymbolKind::Variable,
                Some(prop_name),
                span,
                Some(scope),
                preview,
            ));
        }
    }

    Ok(symbols)
}

/// Extract constant declarations (class constants and global constants)
fn extract_constants(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (const_declaration
            (const_element
                (name) @name)) @const
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create constant query")?;

    extract_symbols(source, root, &query, SymbolKind::Constant, None)
}

/// Extract namespace definitions
fn extract_namespaces(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (namespace_definition
            name: (namespace_name) @name) @namespace
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create namespace query")?;

    extract_symbols(source, root, &query, SymbolKind::Namespace, None)
}

/// Extract enum declarations (PHP 8.1+)
fn extract_enums(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (enum_declaration
            name: (name) @name) @enum
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create enum query")?;

    extract_symbols(source, root, &query, SymbolKind::Enum, None)
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
                Language::PHP,
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
            <?php
            function greet($name) {
                return "Hello, $name!";
            }
        "#;

        let symbols = parse("test.php", source).unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].symbol.as_deref(), Some("greet"));
        assert!(matches!(symbols[0].kind, SymbolKind::Function));
    }

    #[test]
    fn test_parse_class() {
        let source = r#"
            <?php
            class User {
                private $name;
                private $email;

                public function __construct($name, $email) {
                    $this->name = $name;
                    $this->email = $email;
                }
            }
        "#;

        let symbols = parse("test.php", source).unwrap();

        // Should find class
        let class_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Class))
            .collect();

        assert_eq!(class_symbols.len(), 1);
        assert_eq!(class_symbols[0].symbol.as_deref(), Some("User"));
    }

    #[test]
    fn test_parse_class_with_methods() {
        let source = r#"
            <?php
            class Calculator {
                public function add($a, $b) {
                    return $a + $b;
                }

                public function subtract($a, $b) {
                    return $a - $b;
                }
            }
        "#;

        let symbols = parse("test.php", source).unwrap();

        // Should find class + 2 methods
        assert!(symbols.len() >= 3);

        let method_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Method))
            .collect();

        assert_eq!(method_symbols.len(), 2);
        assert!(method_symbols.iter().any(|s| s.symbol.as_deref() == Some("add")));
        assert!(method_symbols.iter().any(|s| s.symbol.as_deref() == Some("subtract")));

        // Check scope
        for method in method_symbols {
            assert_eq!(method.scope.as_ref().unwrap(), "class Calculator");
        }
    }

    #[test]
    fn test_parse_interface() {
        let source = r#"
            <?php
            interface Drawable {
                public function draw();
            }
        "#;

        let symbols = parse("test.php", source).unwrap();

        let interface_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Interface))
            .collect();

        assert_eq!(interface_symbols.len(), 1);
        assert_eq!(interface_symbols[0].symbol.as_deref(), Some("Drawable"));
    }

    #[test]
    fn test_parse_trait() {
        let source = r#"
            <?php
            trait Loggable {
                public function log($message) {
                    echo $message;
                }
            }
        "#;

        let symbols = parse("test.php", source).unwrap();

        let trait_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Trait))
            .collect();

        assert_eq!(trait_symbols.len(), 1);
        assert_eq!(trait_symbols[0].symbol.as_deref(), Some("Loggable"));
    }

    #[test]
    fn test_parse_namespace() {
        let source = r#"
            <?php
            namespace App\Controllers;

            class HomeController {
                public function index() {
                    return 'Home';
                }
            }
        "#;

        let symbols = parse("test.php", source).unwrap();

        let namespace_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Namespace))
            .collect();

        assert_eq!(namespace_symbols.len(), 1);
        assert_eq!(namespace_symbols[0].symbol.as_deref(), Some("App\\Controllers"));
    }

    #[test]
    fn test_parse_constants() {
        let source = r#"
            <?php
            const MAX_SIZE = 100;
            const DEFAULT_NAME = 'Anonymous';
        "#;

        let symbols = parse("test.php", source).unwrap();

        let const_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Constant))
            .collect();

        assert_eq!(const_symbols.len(), 2);
        assert!(const_symbols.iter().any(|s| s.symbol.as_deref() == Some("MAX_SIZE")));
        assert!(const_symbols.iter().any(|s| s.symbol.as_deref() == Some("DEFAULT_NAME")));
    }

    #[test]
    fn test_parse_properties() {
        let source = r#"
            <?php
            class Config {
                private $debug = false;
                public $timeout = 30;
                protected $secret;
            }
        "#;

        let symbols = parse("test.php", source).unwrap();

        let prop_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Variable))
            .collect();

        assert_eq!(prop_symbols.len(), 3);
        assert!(prop_symbols.iter().any(|s| s.symbol.as_deref() == Some("debug")));
        assert!(prop_symbols.iter().any(|s| s.symbol.as_deref() == Some("timeout")));
        assert!(prop_symbols.iter().any(|s| s.symbol.as_deref() == Some("secret")));
    }

    #[test]
    fn test_parse_enum() {
        let source = r#"
            <?php
            enum Status {
                case Active;
                case Inactive;
                case Pending;
            }
        "#;

        let symbols = parse("test.php", source).unwrap();

        let enum_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Enum))
            .collect();

        assert_eq!(enum_symbols.len(), 1);
        assert_eq!(enum_symbols[0].symbol.as_deref(), Some("Status"));
    }

    #[test]
    fn test_parse_mixed_symbols() {
        let source = r#"
            <?php
            namespace App\Models;

            interface UserInterface {
                public function getName();
            }

            trait Timestampable {
                private $createdAt;

                public function getCreatedAt() {
                    return $this->createdAt;
                }
            }

            class User implements UserInterface {
                use Timestampable;

                private $name;
                const DEFAULT_ROLE = 'user';

                public function __construct($name) {
                    $this->name = $name;
                }

                public function getName() {
                    return $this->name;
                }
            }

            function createUser($name) {
                return new User($name);
            }
        "#;

        let symbols = parse("test.php", source).unwrap();

        // Should find: namespace, interface, trait, class, methods, properties, const, function
        assert!(symbols.len() >= 8);

        let kinds: Vec<&SymbolKind> = symbols.iter().map(|s| &s.kind).collect();
        assert!(kinds.contains(&&SymbolKind::Namespace));
        assert!(kinds.contains(&&SymbolKind::Interface));
        assert!(kinds.contains(&&SymbolKind::Trait));
        assert!(kinds.contains(&&SymbolKind::Class));
        assert!(kinds.contains(&&SymbolKind::Method));
        assert!(kinds.contains(&&SymbolKind::Variable));
        assert!(kinds.contains(&&SymbolKind::Constant));
        assert!(kinds.contains(&&SymbolKind::Function));
    }
}
