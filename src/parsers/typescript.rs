//! TypeScript/JavaScript language parser using Tree-sitter
//!
//! Extracts symbols from TypeScript and JavaScript source code:
//! - Functions (regular, arrow, async, generator)
//! - Classes (regular, abstract)
//! - Interfaces
//! - Type aliases
//! - Enums
//! - Variables and constants
//! - Methods (with class scope)
//! - Modules/Namespaces
//!
//! This parser handles both TypeScript (.ts, .tsx) and JavaScript (.js, .jsx)
//! files using the tree-sitter-typescript grammar.

use anyhow::{Context, Result};
use streaming_iterator::StreamingIterator;
use tree_sitter::{Parser, Query, QueryCursor};
use crate::models::{Language, SearchResult, Span, SymbolKind};

/// Parse TypeScript/JavaScript source code and extract symbols
pub fn parse(path: &str, source: &str, language: Language) -> Result<Vec<SearchResult>> {
    let mut parser = Parser::new();

    // tree-sitter-typescript provides both TypeScript and TSX grammars
    // For JavaScript, we use the TypeScript grammar (it's a superset)
    let ts_language_fn = match language {
        Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT,
        Language::JavaScript => tree_sitter_typescript::LANGUAGE_TSX, // TSX handles both JS and JSX
        _ => return Err(anyhow::anyhow!("Unsupported language: {:?}", language)),
    };

    // Convert LanguageFn to Language
    let ts_language: tree_sitter::Language = ts_language_fn.into();

    parser
        .set_language(&ts_language)
        .context("Failed to set TypeScript/JavaScript language")?;

    let tree = parser
        .parse(source, None)
        .context("Failed to parse TypeScript/JavaScript source")?;

    let root_node = tree.root_node();

    let mut symbols = Vec::new();

    // Extract different types of symbols using Tree-sitter queries
    symbols.extend(extract_functions(source, &root_node, &ts_language)?);
    symbols.extend(extract_arrow_functions(source, &root_node, &ts_language)?);
    symbols.extend(extract_classes(source, &root_node, &ts_language)?);
    symbols.extend(extract_interfaces(source, &root_node, &ts_language)?);
    symbols.extend(extract_type_aliases(source, &root_node, &ts_language)?);
    symbols.extend(extract_enums(source, &root_node, &ts_language)?);
    symbols.extend(extract_variables(source, &root_node, &ts_language)?);
    symbols.extend(extract_methods(source, &root_node, &ts_language)?);

    // Add file path and language to all symbols
    for symbol in &mut symbols {
        symbol.path = path.to_string();
        symbol.lang = language.clone();
    }

    Ok(symbols)
}

/// Extract regular function declarations (including async and generator)
fn extract_functions(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (function_declaration
            name: (identifier) @name) @function

        (generator_function_declaration
            name: (identifier) @name) @function
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create function query")?;

    extract_symbols(source, root, &query, SymbolKind::Function, None)
}

/// Extract arrow functions assigned to variables/constants
fn extract_arrow_functions(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (lexical_declaration
            (variable_declarator
                name: (identifier) @name
                value: (arrow_function))) @arrow_fn

        (variable_declaration
            (variable_declarator
                name: (identifier) @name
                value: (arrow_function))) @arrow_fn
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create arrow function query")?;

    extract_symbols(source, root, &query, SymbolKind::Function, None)
}

/// Extract class declarations (including abstract classes)
fn extract_classes(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (class_declaration
            name: (type_identifier) @name) @class

        (abstract_class_declaration
            name: (type_identifier) @name) @class
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
            name: (type_identifier) @name) @interface
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create interface query")?;

    extract_symbols(source, root, &query, SymbolKind::Interface, None)
}

/// Extract type alias declarations
fn extract_type_aliases(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (type_alias_declaration
            name: (type_identifier) @name) @type
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create type alias query")?;

    extract_symbols(source, root, &query, SymbolKind::Type, None)
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

/// Extract variable and constant declarations
fn extract_variables(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    // Only extract const declarations that are NOT arrow functions (skip let/var to reduce noise)
    // Arrow functions are already handled by extract_arrow_functions
    let query_str = r#"
        (lexical_declaration
            (variable_declarator
                name: (identifier) @name)) @const
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create variable query")?;

    // Filter to only const declarations that are not arrow functions
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, *root, source.as_bytes());

    let mut symbols = Vec::new();

    while let Some(match_) = matches.next() {
        let mut name = None;
        let mut declarator_node = None;

        for capture in match_.captures {
            let capture_name: &str = &query.capture_names()[capture.index as usize];
            if capture_name == "name" {
                name = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                // Get the variable_declarator node
                if let Some(parent) = capture.node.parent() {
                    if parent.kind() == "variable_declarator" {
                        declarator_node = Some(parent);
                    }
                }
            }
        }

        if let (Some(name), Some(declarator)) = (name, declarator_node) {
            // Check if this is a const declaration AND not an arrow function
            if let Some(lexical_decl) = declarator.parent() {
                let parent_text = lexical_decl.utf8_text(source.as_bytes()).unwrap_or("");

                // Check if it's a const declaration
                if parent_text.trim_start().starts_with("const") {
                    // Check if the value is an arrow function
                    let mut is_arrow_function = false;
                    for i in 0..declarator.child_count() {
                        if let Some(child) = declarator.child(i) {
                            if child.kind() == "arrow_function" {
                                is_arrow_function = true;
                                break;
                            }
                        }
                    }

                    // Only add if it's NOT an arrow function
                    if !is_arrow_function {
                        let span = node_to_span(&lexical_decl);
                        let preview = extract_preview(source, &span);

                        symbols.push(SearchResult::new(
                            String::new(),
                            Language::TypeScript,
                            SymbolKind::Constant,
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

/// Extract method definitions from classes
fn extract_methods(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (class_declaration
            name: (type_identifier) @class_name
            body: (class_body
                (method_definition
                    name: (_) @method_name))) @class

        (abstract_class_declaration
            name: (type_identifier) @class_name
            body: (class_body
                (method_definition
                    name: (_) @method_name))) @class
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create method query")?;

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, *root, source.as_bytes());

    let mut symbols = Vec::new();

    while let Some(match_) = matches.next() {
        let mut class_name = None;
        let mut method_name = None;
        let mut method_node = None;

        for capture in match_.captures {
            let capture_name: &str = &query.capture_names()[capture.index as usize];
            match capture_name {
                "class_name" => {
                    class_name = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                }
                "method_name" => {
                    method_name = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                    // Find the parent method_definition node
                    let mut current = capture.node;
                    while let Some(parent) = current.parent() {
                        if parent.kind() == "method_definition" {
                            method_node = Some(parent);
                            break;
                        }
                        current = parent;
                    }
                }
                _ => {}
            }
        }

        if let (Some(class_name), Some(method_name), Some(node)) = (class_name, method_name, method_node) {
            let scope = format!("class {}", class_name);
            let span = node_to_span(&node);
            let preview = extract_preview(source, &span);

            symbols.push(SearchResult::new(
                String::new(),
                Language::TypeScript,
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
                Language::TypeScript,
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
            function greet(name: string): string {
                return `Hello, ${name}!`;
            }
        "#;

        let symbols = parse("test.ts", source, Language::TypeScript).unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].symbol, "greet");
        assert!(matches!(symbols[0].kind, SymbolKind::Function));
    }

    #[test]
    fn test_parse_arrow_function() {
        let source = r#"
            const add = (a: number, b: number): number => {
                return a + b;
            };
        "#;

        let symbols = parse("test.ts", source, Language::TypeScript).unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].symbol, "add");
        assert!(matches!(symbols[0].kind, SymbolKind::Function));
    }

    #[test]
    fn test_parse_async_function() {
        let source = r#"
            async function fetchData(url: string): Promise<Response> {
                return await fetch(url);
            }
        "#;

        let symbols = parse("test.ts", source, Language::TypeScript).unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].symbol, "fetchData");
        assert!(matches!(symbols[0].kind, SymbolKind::Function));
    }

    #[test]
    fn test_parse_class() {
        let source = r#"
            class User {
                name: string;
                age: number;

                constructor(name: string, age: number) {
                    this.name = name;
                    this.age = age;
                }
            }
        "#;

        let symbols = parse("test.ts", source, Language::TypeScript).unwrap();

        // Should find class
        let class_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Class))
            .collect();

        assert_eq!(class_symbols.len(), 1);
        assert_eq!(class_symbols[0].symbol, "User");
    }

    #[test]
    fn test_parse_class_with_methods() {
        let source = r#"
            class Calculator {
                add(a: number, b: number): number {
                    return a + b;
                }

                subtract(a: number, b: number): number {
                    return a - b;
                }
            }
        "#;

        let symbols = parse("test.ts", source, Language::TypeScript).unwrap();

        // Should find class + 2 methods
        assert!(symbols.len() >= 3);

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
    fn test_parse_interface() {
        let source = r#"
            interface User {
                name: string;
                age: number;
                email?: string;
            }
        "#;

        let symbols = parse("test.ts", source, Language::TypeScript).unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].symbol, "User");
        assert!(matches!(symbols[0].kind, SymbolKind::Interface));
    }

    #[test]
    fn test_parse_type_alias() {
        let source = r#"
            type UserId = string | number;
            type UserRole = 'admin' | 'user' | 'guest';
        "#;

        let symbols = parse("test.ts", source, Language::TypeScript).unwrap();
        assert_eq!(symbols.len(), 2);

        let type_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Type))
            .collect();

        assert_eq!(type_symbols.len(), 2);
        assert!(type_symbols.iter().any(|s| s.symbol == "UserId"));
        assert!(type_symbols.iter().any(|s| s.symbol == "UserRole"));
    }

    #[test]
    fn test_parse_enum() {
        let source = r#"
            enum Status {
                Active,
                Inactive,
                Pending
            }
        "#;

        let symbols = parse("test.ts", source, Language::TypeScript).unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].symbol, "Status");
        assert!(matches!(symbols[0].kind, SymbolKind::Enum));
    }

    #[test]
    fn test_parse_const() {
        let source = r#"
            const MAX_SIZE = 100;
            const DEFAULT_USER = {
                name: "Anonymous",
                age: 0
            };
        "#;

        let symbols = parse("test.ts", source, Language::TypeScript).unwrap();
        assert_eq!(symbols.len(), 2);

        let const_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Constant))
            .collect();

        assert_eq!(const_symbols.len(), 2);
        assert!(const_symbols.iter().any(|s| s.symbol == "MAX_SIZE"));
        assert!(const_symbols.iter().any(|s| s.symbol == "DEFAULT_USER"));
    }

    #[test]
    fn test_parse_react_component() {
        let source = r#"
            import React, { useState } from 'react';

            interface ButtonProps {
                label: string;
                onClick: () => void;
            }

            const Button: React.FC<ButtonProps> = ({ label, onClick }) => {
                return (
                    <button onClick={onClick}>
                        {label}
                    </button>
                );
            };

            function useCounter(initial: number) {
                const [count, setCount] = React.useState(initial);
                return { count, setCount };
            }

            export default Button;
        "#;

        let symbols = parse("Button.tsx", source, Language::TypeScript).unwrap();

        // Should find interface, Button component (arrow fn), useCounter hook (function)
        assert!(symbols.iter().any(|s| s.symbol == "ButtonProps" && matches!(s.kind, SymbolKind::Interface)));
        assert!(symbols.iter().any(|s| s.symbol == "Button" && matches!(s.kind, SymbolKind::Function)));
        assert!(symbols.iter().any(|s| s.symbol == "useCounter" && matches!(s.kind, SymbolKind::Function)));
    }

    #[test]
    fn test_parse_mixed_symbols() {
        let source = r#"
            interface Config {
                debug: boolean;
            }

            type ConfigKey = keyof Config;

            const DEFAULT_CONFIG: Config = {
                debug: false
            };

            class ConfigManager {
                private config: Config;

                constructor(config: Config) {
                    this.config = config;
                }

                getConfig(): Config {
                    return this.config;
                }
            }

            function loadConfig(): Config {
                return DEFAULT_CONFIG;
            }
        "#;

        let symbols = parse("test.ts", source, Language::TypeScript).unwrap();

        // Should find: interface, type, const, class, method, function
        assert!(symbols.len() >= 6);

        let kinds: Vec<&SymbolKind> = symbols.iter().map(|s| &s.kind).collect();
        assert!(kinds.contains(&&SymbolKind::Interface));
        assert!(kinds.contains(&&SymbolKind::Type));
        assert!(kinds.contains(&&SymbolKind::Constant));
        assert!(kinds.contains(&&SymbolKind::Class));
        assert!(kinds.contains(&&SymbolKind::Method));
        assert!(kinds.contains(&&SymbolKind::Function));
    }
}
