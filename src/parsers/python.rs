//! Python language parser using Tree-sitter
//!
//! Extracts symbols from Python source code:
//! - Functions (def, async def)
//! - Classes (regular, abstract)
//! - Methods (regular, async, static, class methods, properties via @property)
//! - Decorators (tracked in scope)
//! - Lambda expressions assigned to variables
//! - Local variables (inside functions)
//! - Global variables (module-level non-uppercase variables)
//! - Constants (module-level uppercase variables)
//! - Imports/Exports

use anyhow::{Context, Result};
use streaming_iterator::StreamingIterator;
use tree_sitter::{Parser, Query, QueryCursor};
use crate::models::{Language, SearchResult, Span, SymbolKind};

/// Parse Python source code and extract symbols
pub fn parse(path: &str, source: &str) -> Result<Vec<SearchResult>> {
    let mut parser = Parser::new();
    let language = tree_sitter_python::LANGUAGE;

    parser
        .set_language(&language.into())
        .context("Failed to set Python language")?;

    let tree = parser
        .parse(source, None)
        .context("Failed to parse Python source")?;

    let root_node = tree.root_node();

    let mut symbols = Vec::new();

    // Extract different types of symbols using Tree-sitter queries
    symbols.extend(extract_functions(source, &root_node, &language.into())?);
    symbols.extend(extract_classes(source, &root_node, &language.into())?);
    symbols.extend(extract_methods(source, &root_node, &language.into())?);
    symbols.extend(extract_constants(source, &root_node, &language.into())?);
    symbols.extend(extract_global_variables(source, &root_node, &language.into())?);
    symbols.extend(extract_local_variables(source, &root_node, &language.into())?);
    symbols.extend(extract_lambdas(source, &root_node, &language.into())?);

    // Add file path to all symbols
    for symbol in &mut symbols {
        symbol.path = path.to_string();
        symbol.lang = Language::Python;
    }

    Ok(symbols)
}

/// Extract function definitions (including async functions)
fn extract_functions(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (function_definition
            name: (identifier) @name) @function
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create function query")?;

    extract_symbols(source, root, &query, SymbolKind::Function, None)
}

/// Extract class definitions
fn extract_classes(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (class_definition
            name: (identifier) @name) @class
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create class query")?;

    extract_symbols(source, root, &query, SymbolKind::Class, None)
}

/// Extract method definitions from classes
fn extract_methods(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (class_definition
            name: (identifier) @class_name
            body: (block
                (function_definition
                    name: (identifier) @method_name))) @class

        (class_definition
            name: (identifier) @class_name
            body: (block
                (decorated_definition
                    (function_definition
                        name: (identifier) @method_name)))) @class
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

        if let (Some(class_name), Some(method_name), Some(node)) = (class_name, method_name, method_node) {
            let scope = format!("class {}", class_name);
            let span = node_to_span(&node);
            let preview = extract_preview(source, &span);

            symbols.push(SearchResult::new(
                String::new(),
                Language::Python,
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

/// Extract module-level constants (uppercase variable assignments)
fn extract_constants(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (module
            (expression_statement
                (assignment
                    left: (identifier) @name))) @const
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create constant query")?;

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, *root, source.as_bytes());

    let mut symbols = Vec::new();

    while let Some(match_) = matches.next() {
        let mut name = None;
        let mut const_node = None;

        for capture in match_.captures {
            let capture_name: &str = &query.capture_names()[capture.index as usize];
            if capture_name == "name" {
                let name_text = capture.node.utf8_text(source.as_bytes()).unwrap_or("");
                // Only include if it's all uppercase (Python constant convention)
                if name_text.chars().all(|c| c.is_uppercase() || c == '_' || c.is_numeric()) {
                    name = Some(name_text.to_string());
                    // Get the assignment node
                    let mut current = capture.node;
                    while let Some(parent) = current.parent() {
                        if parent.kind() == "assignment" {
                            const_node = Some(parent);
                            break;
                        }
                        current = parent;
                    }
                }
            }
        }

        if let (Some(name), Some(node)) = (name, const_node) {
            let span = node_to_span(&node);
            let preview = extract_preview(source, &span);

            symbols.push(SearchResult::new(
                String::new(),
                Language::Python,
                SymbolKind::Constant,
                Some(name),
                span,
                None,
                preview,
            ));
        }
    }

    Ok(symbols)
}

/// Extract module-level global variables (non-uppercase variable assignments)
fn extract_global_variables(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (module
            (expression_statement
                (assignment
                    left: (identifier) @name))) @var
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create global variable query")?;

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, *root, source.as_bytes());

    let mut symbols = Vec::new();

    while let Some(match_) = matches.next() {
        let mut name = None;
        let mut var_node = None;

        for capture in match_.captures {
            let capture_name: &str = &query.capture_names()[capture.index as usize];
            if capture_name == "name" {
                let name_text = capture.node.utf8_text(source.as_bytes()).unwrap_or("");
                // Only include if it's NOT all uppercase (constants are handled separately)
                if !name_text.chars().all(|c| c.is_uppercase() || c == '_' || c.is_numeric()) {
                    name = Some(name_text.to_string());
                    // Get the assignment node
                    let mut current = capture.node;
                    while let Some(parent) = current.parent() {
                        if parent.kind() == "assignment" {
                            var_node = Some(parent);
                            break;
                        }
                        current = parent;
                    }
                }
            }
        }

        if let (Some(name), Some(node)) = (name, var_node) {
            let span = node_to_span(&node);
            let preview = extract_preview(source, &span);

            symbols.push(SearchResult::new(
                String::new(),
                Language::Python,
                SymbolKind::Variable,
                Some(name),
                span,
                None,
                preview,
            ));
        }
    }

    Ok(symbols)
}

/// Extract local variable assignments inside functions
fn extract_local_variables(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (assignment
            left: (identifier) @name) @assignment
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create local variable query")?;

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, *root, source.as_bytes());

    let mut symbols = Vec::new();

    while let Some(match_) = matches.next() {
        let mut name = None;
        let mut assignment_node = None;

        for capture in match_.captures {
            let capture_name: &str = &query.capture_names()[capture.index as usize];
            match capture_name {
                "name" => {
                    let name_text = capture.node.utf8_text(source.as_bytes()).unwrap_or("");
                    // Skip uppercase constants (handled by extract_constants)
                    if !name_text.chars().all(|c| c.is_uppercase() || c == '_' || c.is_numeric()) {
                        name = Some(name_text.to_string());
                    }
                }
                "assignment" => {
                    assignment_node = Some(capture.node);
                }
                _ => {}
            }
        }

        // Check if this assignment is inside a function definition
        if let (Some(name), Some(node)) = (name, assignment_node) {
            let mut is_in_function = false;
            let mut current = node;

            while let Some(parent) = current.parent() {
                if parent.kind() == "function_definition" {
                    is_in_function = true;
                    break;
                }
                // Stop if we hit module level
                if parent.kind() == "module" {
                    break;
                }
                current = parent;
            }

            if is_in_function {
                let span = node_to_span(&node);
                let preview = extract_preview(source, &span);

                symbols.push(SearchResult::new(
                    String::new(),
                    Language::Python,
                    SymbolKind::Variable,
                    Some(name),
                    span,
                    None,  // No scope for local variables
                    preview,
                ));
            }
        }
    }

    Ok(symbols)
}

/// Extract lambda expressions assigned to variables
fn extract_lambdas(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (assignment
            left: (identifier) @name
            right: (lambda)) @lambda
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create lambda query")?;

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
                Language::Python,
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
def hello_world():
    print("Hello, world!")
    return True
        "#;

        let symbols = parse("test.py", source).unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].symbol.as_deref(), Some("hello_world"));
        assert!(matches!(symbols[0].kind, SymbolKind::Function));
    }

    #[test]
    fn test_parse_async_function() {
        let source = r#"
async def fetch_data(url):
    async with aiohttp.ClientSession() as session:
        async with session.get(url) as response:
            return await response.text()
        "#;

        let symbols = parse("test.py", source).unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].symbol.as_deref(), Some("fetch_data"));
        assert!(matches!(symbols[0].kind, SymbolKind::Function));
    }

    #[test]
    fn test_parse_class() {
        let source = r#"
class User:
    def __init__(self, name, age):
        self.name = name
        self.age = age
        "#;

        let symbols = parse("test.py", source).unwrap();

        let class_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Class))
            .collect();

        assert_eq!(class_symbols.len(), 1);
        assert_eq!(class_symbols[0].symbol.as_deref(), Some("User"));
    }

    #[test]
    fn test_parse_class_with_methods() {
        let source = r#"
class Calculator:
    def add(self, a, b):
        return a + b

    def subtract(self, a, b):
        return a - b

    @staticmethod
    def multiply(a, b):
        return a * b
        "#;

        let symbols = parse("test.py", source).unwrap();

        let method_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Method))
            .collect();

        assert_eq!(method_symbols.len(), 3);
        assert!(method_symbols.iter().any(|s| s.symbol.as_deref() == Some("add")));
        assert!(method_symbols.iter().any(|s| s.symbol.as_deref() == Some("subtract")));
        assert!(method_symbols.iter().any(|s| s.symbol.as_deref() == Some("multiply")));

        // Check scope
        for method in method_symbols {
            // Removed: scope field no longer exists: assert_eq!(method.scope.as_ref().unwrap(), "class Calculator");
        }
    }

    #[test]
    fn test_parse_async_method() {
        let source = r#"
class DataFetcher:
    async def get_user(self, user_id):
        return await fetch(f"/users/{user_id}")

    async def get_all_users(self):
        return await fetch("/users")
        "#;

        let symbols = parse("test.py", source).unwrap();

        let method_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Method))
            .collect();

        assert_eq!(method_symbols.len(), 2);
        assert!(method_symbols.iter().any(|s| s.symbol.as_deref() == Some("get_user")));
        assert!(method_symbols.iter().any(|s| s.symbol.as_deref() == Some("get_all_users")));
    }

    #[test]
    fn test_parse_constants() {
        let source = r#"
MAX_SIZE = 100
DEFAULT_TIMEOUT = 30
API_URL = "https://api.example.com"
        "#;

        let symbols = parse("test.py", source).unwrap();

        let const_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Constant))
            .collect();

        assert_eq!(const_symbols.len(), 3);
        assert!(const_symbols.iter().any(|s| s.symbol.as_deref() == Some("MAX_SIZE")));
        assert!(const_symbols.iter().any(|s| s.symbol.as_deref() == Some("DEFAULT_TIMEOUT")));
        assert!(const_symbols.iter().any(|s| s.symbol.as_deref() == Some("API_URL")));
    }

    #[test]
    fn test_parse_lambda() {
        let source = r#"
square = lambda x: x * x
add = lambda a, b: a + b
        "#;

        let symbols = parse("test.py", source).unwrap();

        let lambda_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Function))
            .collect();

        assert!(lambda_symbols.len() >= 2);
        assert!(lambda_symbols.iter().any(|s| s.symbol.as_deref() == Some("square")));
        assert!(lambda_symbols.iter().any(|s| s.symbol.as_deref() == Some("add")));
    }

    #[test]
    fn test_parse_decorated_method() {
        let source = r#"
class WebService:
    @property
    def url(self):
        return self._url

    @classmethod
    def from_config(cls, config):
        return cls(config['url'])

    @staticmethod
    def validate_url(url):
        return url.startswith('http')
        "#;

        let symbols = parse("test.py", source).unwrap();

        let method_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Method))
            .collect();

        assert_eq!(method_symbols.len(), 3);
        assert!(method_symbols.iter().any(|s| s.symbol.as_deref() == Some("url")));
        assert!(method_symbols.iter().any(|s| s.symbol.as_deref() == Some("from_config")));
        assert!(method_symbols.iter().any(|s| s.symbol.as_deref() == Some("validate_url")));
    }

    #[test]
    fn test_parse_mixed_symbols() {
        let source = r#"
API_KEY = "secret123"
MAX_RETRIES = 3

class APIClient:
    def __init__(self, api_key):
        self.api_key = api_key

    async def request(self, endpoint):
        return await self._fetch(endpoint)

    @staticmethod
    def build_url(endpoint):
        return f"https://api.example.com/{endpoint}"

def create_client():
    return APIClient(API_KEY)

process = lambda data: data.strip().lower()
        "#;

        let symbols = parse("test.py", source).unwrap();

        // Should find: 2 constants, 1 class, 3 methods, 1 function, 1 lambda
        assert!(symbols.len() >= 8);

        let kinds: Vec<&SymbolKind> = symbols.iter().map(|s| &s.kind).collect();
        assert!(kinds.contains(&&SymbolKind::Constant));
        assert!(kinds.contains(&&SymbolKind::Class));
        assert!(kinds.contains(&&SymbolKind::Method));
        assert!(kinds.contains(&&SymbolKind::Function));
    }

    #[test]
    fn test_parse_nested_class() {
        let source = r#"
class Outer:
    class Inner:
        def inner_method(self):
            pass

    def outer_method(self):
        pass
        "#;

        let symbols = parse("test.py", source).unwrap();

        let class_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Class))
            .collect();

        // Should find both Outer and Inner classes
        assert_eq!(class_symbols.len(), 2);
        assert!(class_symbols.iter().any(|s| s.symbol.as_deref() == Some("Outer")));
        assert!(class_symbols.iter().any(|s| s.symbol.as_deref() == Some("Inner")));
    }

    #[test]
    fn test_local_variables_included() {
        let source = r#"
def calculate(input):
    local_var = input * 2
    result = local_var + 10
    return result

class Calculator:
    def compute(self, value):
        temp = value * 3
        final = temp + 5
        return final
        "#;

        let symbols = parse("test.py", source).unwrap();

        // Filter to just variables
        let variables: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Variable))
            .collect();

        // Check that local variables are captured
        assert!(variables.iter().any(|v| v.symbol.as_deref() == Some("local_var")));
        assert!(variables.iter().any(|v| v.symbol.as_deref() == Some("result")));
        assert!(variables.iter().any(|v| v.symbol.as_deref() == Some("temp")));
        assert!(variables.iter().any(|v| v.symbol.as_deref() == Some("final")));

        // Verify that local variables have no scope
        for var in variables {
            // Removed: scope field no longer exists: assert_eq!(var.scope, None);
        }
    }

    #[test]
    fn test_global_variables() {
        let source = r#"
# Global constants (uppercase)
MAX_SIZE = 100
DEFAULT_TIMEOUT = 30

# Global variables (non-uppercase)
database_url = "postgresql://localhost/mydb"
config = {"debug": True}
current_user = None

def get_config():
    return config
        "#;

        let symbols = parse("test.py", source).unwrap();

        // Filter to constants and variables
        let constants: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Constant))
            .collect();

        let variables: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Variable))
            .collect();

        // Check that constants are captured (uppercase)
        assert!(constants.iter().any(|c| c.symbol.as_deref() == Some("MAX_SIZE")));
        assert!(constants.iter().any(|c| c.symbol.as_deref() == Some("DEFAULT_TIMEOUT")));

        // Check that global variables are captured (non-uppercase)
        assert!(variables.iter().any(|v| v.symbol.as_deref() == Some("database_url")));
        assert!(variables.iter().any(|v| v.symbol.as_deref() == Some("config")));
        assert!(variables.iter().any(|v| v.symbol.as_deref() == Some("current_user")));

        // Verify no scope for both
        for constant in constants {
            // Removed: scope field no longer exists: assert_eq!(constant.scope, None);
        }
        for var in variables {
            // Removed: scope field no longer exists: assert_eq!(var.scope, None);
        }
    }
}
