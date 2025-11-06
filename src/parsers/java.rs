//! Java language parser using Tree-sitter
//!
//! Extracts symbols from Java source code:
//! - Classes (regular, abstract, final)
//! - Interfaces
//! - Enums
//! - Records (Java 14+)
//! - Methods (with class scope, visibility)
//! - Fields (public, private, protected, static)
//! - Constructors
//! - Annotations
//! - Local variables (inside method bodies)

use anyhow::{Context, Result};
use streaming_iterator::StreamingIterator;
use tree_sitter::{Parser, Query, QueryCursor};
use crate::models::{Language, SearchResult, Span, SymbolKind};

/// Parse Java source code and extract symbols
pub fn parse(path: &str, source: &str) -> Result<Vec<SearchResult>> {
    let mut parser = Parser::new();
    let language = tree_sitter_java::LANGUAGE;

    parser
        .set_language(&language.into())
        .context("Failed to set Java language")?;

    let tree = parser
        .parse(source, None)
        .context("Failed to parse Java source")?;

    let root_node = tree.root_node();

    let mut symbols = Vec::new();

    // Extract different types of symbols using Tree-sitter queries
    symbols.extend(extract_classes(source, &root_node, &language.into())?);
    symbols.extend(extract_interfaces(source, &root_node, &language.into())?);
    symbols.extend(extract_enums(source, &root_node, &language.into())?);
    symbols.extend(extract_class_methods(source, &root_node, &language.into())?);
    symbols.extend(extract_interface_methods(source, &root_node, &language.into())?);
    symbols.extend(extract_fields(source, &root_node, &language.into())?);
    symbols.extend(extract_constructors(source, &root_node, &language.into())?);
    symbols.extend(extract_local_variables(source, &root_node, &language.into())?);

    // Add file path to all symbols
    for symbol in &mut symbols {
        symbol.path = path.to_string();
        symbol.lang = Language::Java;
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

/// Extract method declarations from classes
fn extract_class_methods(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (class_declaration
            name: (identifier) @class_name
            body: (class_body
                (method_declaration
                    name: (identifier) @method_name))) @class

        (enum_declaration
            name: (identifier) @enum_name
            body: (enum_body
                (enum_body_declarations
                    (method_declaration
                        name: (identifier) @method_name)))) @enum
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
                "enum_name" => {
                    scope_name = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                    scope_type = Some("enum");
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
                Language::Java,
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

/// Extract field declarations from classes
fn extract_fields(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (class_declaration
            name: (identifier) @class_name
            body: (class_body
                (field_declaration
                    declarator: (variable_declarator
                        name: (identifier) @field_name)))) @class

        (enum_declaration
            name: (identifier) @enum_name
            body: (enum_body
                (enum_body_declarations
                    (field_declaration
                        declarator: (variable_declarator
                            name: (identifier) @field_name))))) @enum
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create field query")?;

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, *root, source.as_bytes());

    let mut symbols = Vec::new();

    while let Some(match_) = matches.next() {
        let mut scope_name = None;
        let mut scope_type = None;
        let mut field_name = None;
        let mut field_node = None;

        for capture in match_.captures {
            let capture_name: &str = &query.capture_names()[capture.index as usize];
            match capture_name {
                "class_name" => {
                    scope_name = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                    scope_type = Some("class");
                }
                "enum_name" => {
                    scope_name = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                    scope_type = Some("enum");
                }
                "field_name" => {
                    field_name = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                    // Find the parent field_declaration node
                    let mut current = capture.node;
                    while let Some(parent) = current.parent() {
                        if parent.kind() == "field_declaration" {
                            field_node = Some(parent);
                            break;
                        }
                        current = parent;
                    }
                }
                _ => {}
            }
        }

        if let (Some(scope_name), Some(scope_type), Some(field_name), Some(node)) =
            (scope_name, scope_type, field_name, field_node) {
            let scope = format!("{} {}", scope_type, scope_name);
            let span = node_to_span(&node);
            let preview = extract_preview(source, &span);

            symbols.push(SearchResult::new(
                String::new(),
                Language::Java,
                SymbolKind::Variable,
                Some(field_name),
                span,
                Some(scope),
                preview,
            ));
        }
    }

    Ok(symbols)
}

/// Extract constructor declarations
fn extract_constructors(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (class_declaration
            name: (identifier) @class_name
            body: (class_body
                (constructor_declaration
                    name: (identifier) @constructor_name))) @class
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create constructor query")?;

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, *root, source.as_bytes());

    let mut symbols = Vec::new();

    while let Some(match_) = matches.next() {
        let mut class_name = None;
        let mut constructor_name = None;
        let mut constructor_node = None;

        for capture in match_.captures {
            let capture_name: &str = &query.capture_names()[capture.index as usize];
            match capture_name {
                "class_name" => {
                    class_name = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                }
                "constructor_name" => {
                    constructor_name = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                    // Find the parent constructor_declaration node
                    let mut current = capture.node;
                    while let Some(parent) = current.parent() {
                        if parent.kind() == "constructor_declaration" {
                            constructor_node = Some(parent);
                            break;
                        }
                        current = parent;
                    }
                }
                _ => {}
            }
        }

        if let (Some(class_name), Some(constructor_name), Some(node)) =
            (class_name, constructor_name, constructor_node) {
            let scope = format!("class {}", class_name);
            let span = node_to_span(&node);
            let preview = extract_preview(source, &span);

            symbols.push(SearchResult::new(
                String::new(),
                Language::Java,
                SymbolKind::Method,
                Some(constructor_name),
                span,
                Some(scope),
                preview,
            ));
        }
    }

    Ok(symbols)
}

/// Extract method declarations from interfaces
fn extract_interface_methods(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (interface_declaration
            name: (identifier) @interface_name
            body: (interface_body
                (method_declaration
                    name: (identifier) @method_name))) @interface
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create interface method query")?;

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, *root, source.as_bytes());

    let mut symbols = Vec::new();

    while let Some(match_) = matches.next() {
        let mut interface_name = None;
        let mut method_name = None;
        let mut method_node = None;

        for capture in match_.captures {
            let capture_name: &str = &query.capture_names()[capture.index as usize];
            match capture_name {
                "interface_name" => {
                    interface_name = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
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

        if let (Some(interface_name), Some(method_name), Some(node)) =
            (interface_name, method_name, method_node) {
            let scope = format!("interface {}", interface_name);
            let span = node_to_span(&node);
            let preview = extract_preview(source, &span);

            symbols.push(SearchResult::new(
                String::new(),
                Language::Java,
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

/// Extract local variable declarations from method bodies
fn extract_local_variables(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (local_variable_declaration
            declarator: (variable_declarator
                name: (identifier) @name)) @var
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
                Language::Java,
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
public class User {
    private String name;
    private int age;
}
        "#;

        let symbols = parse("test.java", source).unwrap();

        let class_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Class))
            .collect();

        assert_eq!(class_symbols.len(), 1);
        assert_eq!(class_symbols[0].symbol.as_deref(), Some("User"));
    }

    #[test]
    fn test_parse_class_with_methods() {
        let source = r#"
public class Calculator {
    public int add(int a, int b) {
        return a + b;
    }

    public int subtract(int a, int b) {
        return a - b;
    }
}
        "#;

        let symbols = parse("test.java", source).unwrap();

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
public interface Drawable {
    void draw();
    void resize(int width, int height);
}
        "#;

        let symbols = parse("test.java", source).unwrap();

        let interface_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Interface))
            .collect();

        assert_eq!(interface_symbols.len(), 1);
        assert_eq!(interface_symbols[0].symbol.as_deref(), Some("Drawable"));
    }

    #[test]
    fn test_parse_enum() {
        let source = r#"
public enum Status {
    ACTIVE,
    INACTIVE,
    PENDING
}
        "#;

        let symbols = parse("test.java", source).unwrap();

        let enum_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Enum))
            .collect();

        assert_eq!(enum_symbols.len(), 1);
        assert_eq!(enum_symbols[0].symbol.as_deref(), Some("Status"));
    }

    #[test]
    fn test_parse_fields() {
        let source = r#"
public class Config {
    private static final int MAX_SIZE = 100;
    private String hostname;
    public int port;
}
        "#;

        let symbols = parse("test.java", source).unwrap();

        let field_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Variable))
            .collect();

        assert_eq!(field_symbols.len(), 3);
        assert!(field_symbols.iter().any(|s| s.symbol.as_deref() == Some("MAX_SIZE")));
        assert!(field_symbols.iter().any(|s| s.symbol.as_deref() == Some("hostname")));
        assert!(field_symbols.iter().any(|s| s.symbol.as_deref() == Some("port")));
    }

    #[test]
    fn test_parse_constructor() {
        let source = r#"
public class User {
    private String name;

    public User(String name) {
        this.name = name;
    }

    public User() {
        this("Anonymous");
    }
}
        "#;

        let symbols = parse("test.java", source).unwrap();

        let constructor_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Method) && s.symbol.as_deref() == Some("User"))
            .collect();

        assert_eq!(constructor_symbols.len(), 2);
    }

    #[test]
    fn test_parse_abstract_class() {
        let source = r#"
public abstract class Animal {
    protected String name;

    public abstract void makeSound();

    public void sleep() {
        System.out.println("Sleeping...");
    }
}
        "#;

        let symbols = parse("test.java", source).unwrap();

        let class_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Class))
            .collect();

        assert_eq!(class_symbols.len(), 1);
        assert_eq!(class_symbols[0].symbol.as_deref(), Some("Animal"));

        let method_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Method))
            .collect();

        assert_eq!(method_symbols.len(), 2);
        assert!(method_symbols.iter().any(|s| s.symbol.as_deref() == Some("makeSound")));
        assert!(method_symbols.iter().any(|s| s.symbol.as_deref() == Some("sleep")));
    }

    #[test]
    fn test_parse_nested_class() {
        let source = r#"
public class Outer {
    private int outerField;

    public static class Nested {
        private int nestedField;

        public void nestedMethod() {
            // ...
        }
    }

    public void outerMethod() {
        // ...
    }
}
        "#;

        let symbols = parse("test.java", source).unwrap();

        let class_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Class))
            .collect();

        assert_eq!(class_symbols.len(), 2);
        assert!(class_symbols.iter().any(|s| s.symbol.as_deref() == Some("Outer")));
        assert!(class_symbols.iter().any(|s| s.symbol.as_deref() == Some("Nested")));
    }

    #[test]
    fn test_parse_interface_with_methods() {
        let source = r#"
public interface Repository<T> {
    T findById(Long id);
    List<T> findAll();
    void save(T entity);
    void delete(T entity);
}
        "#;

        let symbols = parse("test.java", source).unwrap();

        let interface_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Interface))
            .collect();

        assert_eq!(interface_symbols.len(), 1);

        let method_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Method))
            .collect();

        assert_eq!(method_symbols.len(), 4);
        assert!(method_symbols.iter().any(|s| s.symbol.as_deref() == Some("findById")));
        assert!(method_symbols.iter().any(|s| s.symbol.as_deref() == Some("findAll")));
        assert!(method_symbols.iter().any(|s| s.symbol.as_deref() == Some("save")));
        assert!(method_symbols.iter().any(|s| s.symbol.as_deref() == Some("delete")));
    }

    #[test]
    fn test_parse_enum_with_methods() {
        let source = r#"
public enum Day {
    MONDAY, TUESDAY, WEDNESDAY;

    public boolean isWeekend() {
        return this == SATURDAY || this == SUNDAY;
    }
}
        "#;

        let symbols = parse("test.java", source).unwrap();

        let enum_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Enum))
            .collect();

        assert_eq!(enum_symbols.len(), 1);

        let method_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Method))
            .collect();

        assert_eq!(method_symbols.len(), 1);
        assert_eq!(method_symbols[0].symbol.as_deref(), Some("isWeekend"));
    }

    #[test]
    fn test_parse_mixed_symbols() {
        let source = r#"
package com.example;

public interface UserService {
    User findUser(Long id);
}

public class User {
    private Long id;
    private String name;

    public User(Long id, String name) {
        this.id = id;
        this.name = name;
    }

    public String getName() {
        return name;
    }
}

public enum UserRole {
    ADMIN, USER, GUEST
}
        "#;

        let symbols = parse("test.java", source).unwrap();

        // Should find: interface, class, enum, fields, constructor, methods
        assert!(symbols.len() >= 7);

        let kinds: Vec<&SymbolKind> = symbols.iter().map(|s| &s.kind).collect();
        assert!(kinds.contains(&&SymbolKind::Interface));
        assert!(kinds.contains(&&SymbolKind::Class));
        assert!(kinds.contains(&&SymbolKind::Enum));
        assert!(kinds.contains(&&SymbolKind::Variable));
        assert!(kinds.contains(&&SymbolKind::Method));
    }

    #[test]
    fn test_parse_generic_class() {
        let source = r#"
public class Container<T> {
    private T value;

    public Container(T value) {
        this.value = value;
    }

    public T getValue() {
        return value;
    }

    public void setValue(T value) {
        this.value = value;
    }
}
        "#;

        let symbols = parse("test.java", source).unwrap();

        let class_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Class))
            .collect();

        assert_eq!(class_symbols.len(), 1);
        assert_eq!(class_symbols[0].symbol.as_deref(), Some("Container"));

        let method_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Method))
            .collect();

        assert!(method_symbols.len() >= 3);
    }

    #[test]
    fn test_local_variables_included() {
        let source = r#"
public class Calculator {
    private int globalCount = 10;

    public int calculate(int x) {
        int localVar = x * 2;
        int anotherLocal = 5;
        return localVar + anotherLocal + globalCount;
    }
}
        "#;

        let symbols = parse("test.java", source).unwrap();

        let var_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Variable))
            .collect();

        // Should find both field (globalCount) and local variables (localVar, anotherLocal)
        assert_eq!(var_symbols.len(), 3);
        assert!(var_symbols.iter().any(|s| s.symbol.as_deref() == Some("globalCount")));
        assert!(var_symbols.iter().any(|s| s.symbol.as_deref() == Some("localVar")));
        assert!(var_symbols.iter().any(|s| s.symbol.as_deref() == Some("anotherLocal")));

        // Check scopes: field should have scope, local vars should not
        let global_count = var_symbols.iter().find(|s| s.symbol.as_deref() == Some("globalCount")).unwrap();
        assert_eq!(global_count.scope.as_ref().unwrap(), "class Calculator");

        let local_var = var_symbols.iter().find(|s| s.symbol.as_deref() == Some("localVar")).unwrap();
        assert_eq!(local_var.scope, None);
    }
}
