//! Go language parser using Tree-sitter
//!
//! Extracts symbols from Go source code:
//! - Functions (func)
//! - Types (struct, interface)
//! - Methods (with receiver type)
//! - Constants (const declarations and blocks)
//! - Variables (package-level var)
//! - Packages/Imports

use anyhow::{Context, Result};
use streaming_iterator::StreamingIterator;
use tree_sitter::{Parser, Query, QueryCursor};
use crate::models::{Language, SearchResult, Span, SymbolKind};

/// Parse Go source code and extract symbols
pub fn parse(path: &str, source: &str) -> Result<Vec<SearchResult>> {
    let mut parser = Parser::new();
    let language = tree_sitter_go::LANGUAGE;

    parser
        .set_language(&language.into())
        .context("Failed to set Go language")?;

    let tree = parser
        .parse(source, None)
        .context("Failed to parse Go source")?;

    let root_node = tree.root_node();

    let mut symbols = Vec::new();

    // Extract different types of symbols using Tree-sitter queries
    symbols.extend(extract_functions(source, &root_node, &language.into())?);
    symbols.extend(extract_types(source, &root_node, &language.into())?);
    symbols.extend(extract_interfaces(source, &root_node, &language.into())?);
    symbols.extend(extract_methods(source, &root_node, &language.into())?);
    symbols.extend(extract_constants(source, &root_node, &language.into())?);
    symbols.extend(extract_variables(source, &root_node, &language.into())?);

    // Add file path to all symbols
    for symbol in &mut symbols {
        symbol.path = path.to_string();
        symbol.lang = Language::Go;
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
            name: (identifier) @name) @function
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create function query")?;

    extract_symbols(source, root, &query, SymbolKind::Function, None)
}

/// Extract type declarations (structs)
fn extract_types(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (type_declaration
            (type_spec
                name: (type_identifier) @name
                type: (struct_type))) @struct
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create struct query")?;

    extract_symbols(source, root, &query, SymbolKind::Struct, None)
}

/// Extract interface declarations
fn extract_interfaces(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (type_declaration
            (type_spec
                name: (type_identifier) @name
                type: (interface_type))) @interface
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create interface query")?;

    extract_symbols(source, root, &query, SymbolKind::Interface, None)
}

/// Extract method declarations (functions with receivers)
fn extract_methods(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (method_declaration
            receiver: (parameter_list
                (parameter_declaration
                    type: [(type_identifier) (pointer_type (type_identifier))] @receiver_type))
            name: (field_identifier) @method_name) @method
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create method query")?;

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, *root, source.as_bytes());

    let mut symbols = Vec::new();

    while let Some(match_) = matches.next() {
        let mut receiver_type = None;
        let mut method_name = None;
        let mut method_node = None;

        for capture in match_.captures {
            let capture_name: &str = &query.capture_names()[capture.index as usize];
            match capture_name {
                "receiver_type" => {
                    receiver_type = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                }
                "method_name" => {
                    method_name = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                }
                "method" => {
                    method_node = Some(capture.node);
                }
                _ => {}
            }
        }

        if let (Some(receiver_type), Some(method_name), Some(node)) = (receiver_type, method_name, method_node) {
            // Clean up receiver type (remove * if pointer)
            let clean_receiver = receiver_type.trim_start_matches('*');
            let scope = format!("type {}", clean_receiver);
            let span = node_to_span(&node);
            let preview = extract_preview(source, &span);

            symbols.push(SearchResult::new(
                String::new(),
                Language::Go,
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

/// Extract constant declarations
fn extract_constants(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (const_declaration
            (const_spec
                name: (identifier) @name)) @const
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create const query")?;

    extract_symbols(source, root, &query, SymbolKind::Constant, None)
}

/// Extract package-level variable declarations
fn extract_variables(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (var_declaration
            (var_spec
                name: (identifier) @name)) @var
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create var query")?;

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, *root, source.as_bytes());

    let mut symbols = Vec::new();
    let mut seen_vars = std::collections::HashSet::new();

    while let Some(match_) = matches.next() {
        for capture in match_.captures {
            let capture_name: &str = &query.capture_names()[capture.index as usize];
            if capture_name == "name" {
                let name = capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string();
                // Only add unique variable names (var blocks can have duplicates)
                if !seen_vars.contains(&name) {
                    seen_vars.insert(name.clone());

                    // Find the var_declaration node
                    let mut current = capture.node;
                    let mut var_node = None;
                    while let Some(parent) = current.parent() {
                        if parent.kind() == "var_declaration" {
                            var_node = Some(parent);
                            break;
                        }
                        current = parent;
                    }

                    if let Some(node) = var_node {
                        let span = node_to_span(&node);
                        let preview = extract_preview(source, &span);

                        symbols.push(SearchResult::new(
                            String::new(),
                            Language::Go,
                            SymbolKind::Variable,
                            name,
                            span,
                            None,
                            preview,
                        ));
                    }
                }
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
                Language::Go,
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
package main

func helloWorld() string {
    return "Hello, world!"
}
        "#;

        let symbols = parse("test.go", source).unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].symbol, "helloWorld");
        assert!(matches!(symbols[0].kind, SymbolKind::Function));
    }

    #[test]
    fn test_parse_struct() {
        let source = r#"
package main

type User struct {
    Name string
    Age  int
}
        "#;

        let symbols = parse("test.go", source).unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].symbol, "User");
        assert!(matches!(symbols[0].kind, SymbolKind::Struct));
    }

    #[test]
    fn test_parse_interface() {
        let source = r#"
package main

type Reader interface {
    Read(p []byte) (n int, err error)
}
        "#;

        let symbols = parse("test.go", source).unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].symbol, "Reader");
        assert!(matches!(symbols[0].kind, SymbolKind::Interface));
    }

    #[test]
    fn test_parse_method() {
        let source = r#"
package main

type User struct {
    Name string
}

func (u *User) GetName() string {
    return u.Name
}

func (u User) SetName(name string) {
    u.Name = name
}
        "#;

        let symbols = parse("test.go", source).unwrap();

        let method_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Method))
            .collect();

        assert_eq!(method_symbols.len(), 2);
        assert!(method_symbols.iter().any(|s| s.symbol == "GetName"));
        assert!(method_symbols.iter().any(|s| s.symbol == "SetName"));

        // Check scope
        for method in method_symbols {
            assert_eq!(method.scope.as_ref().unwrap(), "type User");
        }
    }

    #[test]
    fn test_parse_constants() {
        let source = r#"
package main

const MaxSize = 100
const DefaultTimeout = 30

const (
    StatusActive   = 1
    StatusInactive = 2
)
        "#;

        let symbols = parse("test.go", source).unwrap();

        let const_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Constant))
            .collect();

        assert_eq!(const_symbols.len(), 4);
        assert!(const_symbols.iter().any(|s| s.symbol == "MaxSize"));
        assert!(const_symbols.iter().any(|s| s.symbol == "DefaultTimeout"));
        assert!(const_symbols.iter().any(|s| s.symbol == "StatusActive"));
        assert!(const_symbols.iter().any(|s| s.symbol == "StatusInactive"));
    }

    #[test]
    fn test_parse_variables() {
        let source = r#"
package main

var GlobalConfig Config
var (
    Logger  *log.Logger
    Version = "1.0.0"
)
        "#;

        let symbols = parse("test.go", source).unwrap();

        let var_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Variable))
            .collect();

        assert_eq!(var_symbols.len(), 3);
        assert!(var_symbols.iter().any(|s| s.symbol == "GlobalConfig"));
        assert!(var_symbols.iter().any(|s| s.symbol == "Logger"));
        assert!(var_symbols.iter().any(|s| s.symbol == "Version"));
    }

    #[test]
    fn test_parse_mixed_symbols() {
        let source = r#"
package main

const DefaultPort = 8080

type Server struct {
    Port int
}

type Handler interface {
    Handle(req *Request) error
}

func (s *Server) Start() error {
    return nil
}

func NewServer(port int) *Server {
    return &Server{Port: port}
}

var globalServer *Server
        "#;

        let symbols = parse("test.go", source).unwrap();

        // Should find: const, struct, interface, method, function, var
        assert!(symbols.len() >= 6);

        let kinds: Vec<&SymbolKind> = symbols.iter().map(|s| &s.kind).collect();
        assert!(kinds.contains(&&SymbolKind::Constant));
        assert!(kinds.contains(&&SymbolKind::Struct));
        assert!(kinds.contains(&&SymbolKind::Interface));
        assert!(kinds.contains(&&SymbolKind::Method));
        assert!(kinds.contains(&&SymbolKind::Function));
        assert!(kinds.contains(&&SymbolKind::Variable));
    }

    #[test]
    fn test_parse_multiple_methods() {
        let source = r#"
package main

type Calculator struct{}

func (c *Calculator) Add(a, b int) int {
    return a + b
}

func (c *Calculator) Subtract(a, b int) int {
    return a - b
}

func (c *Calculator) Multiply(a, b int) int {
    return a * b
}
        "#;

        let symbols = parse("test.go", source).unwrap();

        let method_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Method))
            .collect();

        assert_eq!(method_symbols.len(), 3);
        assert!(method_symbols.iter().any(|s| s.symbol == "Add"));
        assert!(method_symbols.iter().any(|s| s.symbol == "Subtract"));
        assert!(method_symbols.iter().any(|s| s.symbol == "Multiply"));
    }

    #[test]
    fn test_parse_type_alias() {
        let source = r#"
package main

type UserID string
type Age int

type Config struct {
    Host string
    Port int
}
        "#;

        let symbols = parse("test.go", source).unwrap();

        // Should find the Config struct (type aliases UserID and Age are type_spec but not struct_type)
        let struct_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Struct))
            .collect();

        assert_eq!(struct_symbols.len(), 1);
        assert_eq!(struct_symbols[0].symbol, "Config");
    }

    #[test]
    fn test_parse_embedded_interface() {
        let source = r#"
package main

type Reader interface {
    Read(p []byte) (n int, err error)
}

type Writer interface {
    Write(p []byte) (n int, err error)
}

type ReadWriter interface {
    Reader
    Writer
}
        "#;

        let symbols = parse("test.go", source).unwrap();

        let interface_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Interface))
            .collect();

        assert_eq!(interface_symbols.len(), 3);
        assert!(interface_symbols.iter().any(|s| s.symbol == "Reader"));
        assert!(interface_symbols.iter().any(|s| s.symbol == "Writer"));
        assert!(interface_symbols.iter().any(|s| s.symbol == "ReadWriter"));
    }
}
