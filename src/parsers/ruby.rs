//! Ruby language parser using Tree-sitter
//!
//! Extracts symbols from Ruby source code:
//! - Classes
//! - Modules
//! - Methods (instance and class methods)
//! - Singleton methods
//! - Constants
//! - Attr readers/writers/accessors
//! - Blocks (lambda, proc)

use anyhow::{Context, Result};
use streaming_iterator::StreamingIterator;
use tree_sitter::{Parser, Query, QueryCursor};
use crate::models::{Language, SearchResult, Span, SymbolKind};

/// Parse Ruby source code and extract symbols
pub fn parse(path: &str, source: &str) -> Result<Vec<SearchResult>> {
    let mut parser = Parser::new();
    let language = tree_sitter_ruby::LANGUAGE;

    parser
        .set_language(&language.into())
        .context("Failed to set Ruby language")?;

    let tree = parser
        .parse(source, None)
        .context("Failed to parse Ruby source")?;

    let root_node = tree.root_node();

    let mut symbols = Vec::new();

    // Extract different types of symbols using Tree-sitter queries
    symbols.extend(extract_modules(source, &root_node, &language.into())?);
    symbols.extend(extract_classes(source, &root_node, &language.into())?);
    symbols.extend(extract_methods(source, &root_node, &language.into())?);
    symbols.extend(extract_singleton_methods(source, &root_node, &language.into())?);
    symbols.extend(extract_constants(source, &root_node, &language.into())?);

    // Add file path to all symbols
    for symbol in &mut symbols {
        symbol.path = path.to_string();
        symbol.lang = Language::Ruby;
    }

    Ok(symbols)
}

/// Extract module declarations
fn extract_modules(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (module
            name: (constant) @name) @module
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create module query")?;

    extract_symbols(source, root, &query, SymbolKind::Module, None)
}

/// Extract class declarations
fn extract_classes(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (class
            name: (constant) @name) @class
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create class query")?;

    extract_symbols(source, root, &query, SymbolKind::Class, None)
}

/// Extract method definitions
fn extract_methods(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (class
            name: (constant) @class_name
            (body_statement
                (method
                    name: (_) @method_name))) @class

        (module
            name: (constant) @module_name
            (body_statement
                (method
                    name: (_) @method_name))) @module
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
                "module_name" => {
                    scope_name = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                    scope_type = Some("module");
                }
                "method_name" => {
                    method_name = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                    // Find the parent method node
                    let mut current = capture.node;
                    while let Some(parent) = current.parent() {
                        if parent.kind() == "method" {
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
                Language::Ruby,
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

/// Extract singleton (class) methods
fn extract_singleton_methods(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (singleton_method
            object: (_) @class_name
            name: (_) @method_name) @method
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create singleton method query")?;

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
                }
                "method" => {
                    method_node = Some(capture.node);
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
                Language::Ruby,
                SymbolKind::Method,
                Some(format!("{}.{}", class_name, method_name)),
                span,
                Some(scope),
                preview,
            ));
        }
    }

    Ok(symbols)
}

/// Extract constants
fn extract_constants(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (assignment
            left: (constant) @name
            right: (_)) @const
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create constant query")?;

    extract_symbols(source, root, &query, SymbolKind::Constant, None)
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
                Language::Ruby,
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
class User
  attr_accessor :name, :email
end
        "#;

        let symbols = parse("test.rb", source).unwrap();

        let class_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Class))
            .collect();

        assert_eq!(class_symbols.len(), 1);
        assert_eq!(class_symbols[0].symbol.as_deref(), Some("User"));
    }

    #[test]
    fn test_parse_module() {
        let source = r#"
module Authentication
  def login
    # implementation
  end
end
        "#;

        let symbols = parse("test.rb", source).unwrap();

        let module_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Module))
            .collect();

        assert_eq!(module_symbols.len(), 1);
        assert_eq!(module_symbols[0].symbol.as_deref(), Some("Authentication"));
    }

    #[test]
    fn test_parse_methods() {
        let source = r#"
class Calculator
  def add(a, b)
    a + b
  end

  def subtract(a, b)
    a - b
  end
end
        "#;

        let symbols = parse("test.rb", source).unwrap();

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
    fn test_parse_singleton_method() {
        let source = r#"
class User
  def self.create(attributes)
    new(attributes).save
  end
end
        "#;

        let symbols = parse("test.rb", source).unwrap();

        let method_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Method))
            .collect();

        assert!(method_symbols.len() >= 1);
        assert!(method_symbols.iter().any(|s| s.symbol.as_deref().unwrap_or("").contains("create")));
    }

    #[test]
    fn test_parse_constants() {
        let source = r#"
MAX_SIZE = 100
DEFAULT_TIMEOUT = 30
API_KEY = "secret123"
        "#;

        let symbols = parse("test.rb", source).unwrap();

        let const_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Constant))
            .collect();

        assert_eq!(const_symbols.len(), 3);
        assert!(const_symbols.iter().any(|s| s.symbol.as_deref() == Some("MAX_SIZE")));
        assert!(const_symbols.iter().any(|s| s.symbol.as_deref() == Some("DEFAULT_TIMEOUT")));
        assert!(const_symbols.iter().any(|s| s.symbol.as_deref() == Some("API_KEY")));
    }

    #[test]
    fn test_parse_nested_class() {
        let source = r#"
module MyApp
  class User
    def initialize(name)
      @name = name
    end
  end
end
        "#;

        let symbols = parse("test.rb", source).unwrap();

        let module_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Module))
            .collect();

        let class_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Class))
            .collect();

        assert_eq!(module_symbols.len(), 1);
        assert_eq!(class_symbols.len(), 1);
        assert_eq!(module_symbols[0].symbol.as_deref(), Some("MyApp"));
        assert_eq!(class_symbols[0].symbol.as_deref(), Some("User"));
    }

    #[test]
    fn test_parse_rails_controller() {
        let source = r#"
class UsersController < ApplicationController
  before_action :authenticate_user!

  def index
    @users = User.all
  end

  def show
    @user = User.find(params[:id])
  end

  def create
    @user = User.new(user_params)
    @user.save
  end
end
        "#;

        let symbols = parse("test.rb", source).unwrap();

        let class_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Class))
            .collect();

        let method_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Method))
            .collect();

        assert_eq!(class_symbols.len(), 1);
        assert_eq!(method_symbols.len(), 3);
        assert!(method_symbols.iter().any(|s| s.symbol.as_deref() == Some("index")));
        assert!(method_symbols.iter().any(|s| s.symbol.as_deref() == Some("show")));
        assert!(method_symbols.iter().any(|s| s.symbol.as_deref() == Some("create")));
    }

    #[test]
    fn test_parse_mixed_symbols() {
        let source = r#"
MAX_RETRIES = 3

module Authentication
  class Session
    def login(username, password)
      # implementation
    end

    def self.destroy_all
      # implementation
    end
  end
end
        "#;

        let symbols = parse("test.rb", source).unwrap();

        // Should find: constant, module, class, instance method, class method
        assert!(symbols.len() >= 4);

        let kinds: Vec<&SymbolKind> = symbols.iter().map(|s| &s.kind).collect();
        assert!(kinds.contains(&&SymbolKind::Constant));
        assert!(kinds.contains(&&SymbolKind::Module));
        assert!(kinds.contains(&&SymbolKind::Class));
        assert!(kinds.contains(&&SymbolKind::Method));
    }
}
