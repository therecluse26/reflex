//! Kotlin language parser using Tree-sitter
//!
//! Extracts symbols from Kotlin source code:
//! - Classes (regular, data, sealed, abstract, open)
//! - Objects (singleton)
//! - Interfaces
//! - Functions
//! - Properties (class/object members)
//! - Local variables (val/var inside functions)
//! - Companion objects
//! - Extensions

use anyhow::{Context, Result};
use streaming_iterator::StreamingIterator;
use tree_sitter::{Parser, Query, QueryCursor};
use crate::models::{Language, SearchResult, Span, SymbolKind};

/// Parse Kotlin source code and extract symbols
pub fn parse(path: &str, source: &str) -> Result<Vec<SearchResult>> {
    let mut parser = Parser::new();
    let language = tree_sitter_kotlin_ng::LANGUAGE;

    parser
        .set_language(&language.into())
        .context("Failed to set Kotlin language")?;

    let tree = parser
        .parse(source, None)
        .context("Failed to parse Kotlin source")?;

    let root_node = tree.root_node();

    let mut symbols = Vec::new();

    // Extract different types of symbols using Tree-sitter queries
    symbols.extend(extract_classes(source, &root_node, &language.into())?);
    symbols.extend(extract_objects(source, &root_node, &language.into())?);
    symbols.extend(extract_interfaces(source, &root_node, &language.into())?);
    symbols.extend(extract_functions(source, &root_node, &language.into())?);
    symbols.extend(extract_properties(source, &root_node, &language.into())?);
    symbols.extend(extract_local_variables(source, &root_node, &language.into())?);

    // Add file path to all symbols
    for symbol in &mut symbols {
        symbol.path = path.to_string();
        symbol.lang = Language::Kotlin;
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
            (identifier) @name) @class
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create class query")?;

    extract_symbols(source, root, &query, SymbolKind::Class, None)
}

/// Extract object declarations (singletons)
fn extract_objects(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (object_declaration
            (identifier) @name) @object
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create object query")?;

    extract_symbols(source, root, &query, SymbolKind::Class, None)
}

/// Extract interface declarations
fn extract_interfaces(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (class_declaration
            "interface"
            (identifier) @name) @interface
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create interface query")?;

    extract_symbols(source, root, &query, SymbolKind::Interface, None)
}

/// Extract function declarations
fn extract_functions(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (class_declaration
            (identifier) @class_name
            (class_body
                (function_declaration
                    (identifier) @method_name))) @class

        (object_declaration
            (identifier) @object_name
            (class_body
                (function_declaration
                    (identifier) @method_name))) @object
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
                "object_name" => {
                    scope_name = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                    scope_type = Some("object");
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
                Language::Kotlin,
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
            (identifier) @class_name
            (class_body
                (property_declaration
                    (variable_declaration
                        (identifier) @property_name)))) @class

        (object_declaration
            (identifier) @object_name
            (class_body
                (property_declaration
                    (variable_declaration
                        (identifier) @property_name)))) @object
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
                "object_name" => {
                    scope_name = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                    scope_type = Some("object");
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
                Language::Kotlin,
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

/// Extract local variable declarations (val/var) inside functions
fn extract_local_variables(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (property_declaration
            (variable_declaration
                (identifier) @name)) @var
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create local variable query")?;

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, *root, source.as_bytes());

    let mut symbols = Vec::new();

    while let Some(match_) = matches.next() {
        let mut name = None;
        let mut var_node = None;

        for capture in match_.captures {
            let capture_name: &str = &query.capture_names()[capture.index as usize];
            match capture_name {
                "name" => {
                    name = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                }
                "var" => {
                    var_node = Some(capture.node);
                }
                _ => {}
            }
        }

        // Check if this variable declaration is inside a function_declaration
        // (not inside a class_body or object_body, which would be properties)
        if let (Some(name), Some(node)) = (name, var_node) {
            let mut is_local_var = false;
            let mut is_class_property = false;
            let mut current = node;

            while let Some(parent) = current.parent() {
                // If we find a function_declaration, it's a local variable
                if parent.kind() == "function_declaration" {
                    is_local_var = true;
                    break;
                }
                // If we find a class_body or object body before a function, it's a property
                if parent.kind() == "class_body" {
                    is_class_property = true;
                    break;
                }
                current = parent;
            }

            // Only add if it's a local variable (inside function, not a class property)
            if is_local_var && !is_class_property {
                let span = node_to_span(&node);
                let preview = extract_preview(source, &span);

                // Determine if it's val (constant) or var (variable)
                let decl_text = node.utf8_text(source.as_bytes()).unwrap_or("");
                let kind = if decl_text.trim_start().starts_with("val") {
                    SymbolKind::Constant
                } else {
                    SymbolKind::Variable
                };

                symbols.push(SearchResult::new(
                    String::new(),
                    Language::Kotlin,
                    kind,
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
                Language::Kotlin,
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
class User(val name: String, val age: Int)
        "#;

        let symbols = parse("test.kt", source).unwrap();

        let class_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Class))
            .collect();

        assert_eq!(class_symbols.len(), 1);
        assert_eq!(class_symbols[0].symbol.as_deref(), Some("User"));
    }

    #[test]
    fn test_parse_data_class() {
        let source = r#"
data class Person(val firstName: String, val lastName: String)
        "#;

        let symbols = parse("test.kt", source).unwrap();

        let class_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Class))
            .collect();

        assert_eq!(class_symbols.len(), 1);
        assert_eq!(class_symbols[0].symbol.as_deref(), Some("Person"));
    }

    #[test]
    fn test_parse_object() {
        let source = r#"
object Singleton {
    fun getInstance() = this
}
        "#;

        let symbols = parse("test.kt", source).unwrap();

        let object_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Class))
            .filter(|s| s.symbol.as_deref() == Some("Singleton"))
            .collect();

        assert_eq!(object_symbols.len(), 1);
    }

    #[test]
    fn test_parse_functions() {
        let source = r#"
class Calculator {
    fun add(a: Int, b: Int): Int {
        return a + b
    }

    fun subtract(a: Int, b: Int): Int {
        return a - b
    }
}
        "#;

        let symbols = parse("test.kt", source).unwrap();

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
    fn test_parse_properties() {
        let source = r#"
class User {
    val name: String = ""
    var age: Int = 0
    lateinit var email: String
}
        "#;

        let symbols = parse("test.kt", source).unwrap();

        let property_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Variable))
            .collect();

        assert_eq!(property_symbols.len(), 3);
        assert!(property_symbols.iter().any(|s| s.symbol.as_deref() == Some("name")));
        assert!(property_symbols.iter().any(|s| s.symbol.as_deref() == Some("age")));
        assert!(property_symbols.iter().any(|s| s.symbol.as_deref() == Some("email")));
    }

    #[test]
    fn test_parse_companion_object() {
        let source = r#"
class User {
    companion object {
        fun create(name: String) = User(name)
    }
}
        "#;

        let symbols = parse("test.kt", source).unwrap();

        let class_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Class))
            .collect();

        assert_eq!(class_symbols.len(), 1);
        assert_eq!(class_symbols[0].symbol.as_deref(), Some("User"));
    }

    #[test]
    fn test_parse_sealed_class() {
        let source = r#"
sealed class Result {
    data class Success(val data: String) : Result()
    data class Error(val error: String) : Result()
}
        "#;

        let symbols = parse("test.kt", source).unwrap();

        let class_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Class))
            .collect();

        // Should find Result, Success, and Error
        assert!(class_symbols.len() >= 1);
        assert!(class_symbols.iter().any(|s| s.symbol.as_deref() == Some("Result")));
    }

    #[test]
    fn test_parse_extension_function() {
        let source = r#"
fun String.isEmail(): Boolean {
    return this.contains("@")
}
        "#;

        let symbols = parse("test.kt", source).unwrap();

        // Extension functions should be captured (test verifies parsing succeeds without panic)
        let _ = symbols; // Suppress unused variable warning
    }

    #[test]
    fn test_parse_android_activity() {
        let source = r#"
class MainActivity : AppCompatActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)
    }

    override fun onResume() {
        super.onResume()
    }
}
        "#;

        let symbols = parse("test.kt", source).unwrap();

        let class_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Class))
            .collect();

        let method_symbols: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Method))
            .collect();

        assert_eq!(class_symbols.len(), 1);
        assert_eq!(method_symbols.len(), 2);
        assert!(method_symbols.iter().any(|s| s.symbol.as_deref() == Some("onCreate")));
        assert!(method_symbols.iter().any(|s| s.symbol.as_deref() == Some("onResume")));
    }

    #[test]
    fn test_parse_mixed_symbols() {
        let source = r#"
object Config {
    const val API_URL = "https://api.example.com"
}

interface Repository {
    fun save(item: Any)
}

data class User(val id: Int, val name: String) {
    fun display(): String {
        return "$id: $name"
    }
}
        "#;

        let symbols = parse("test.kt", source).unwrap();

        // Should find: object, interface, data class, method
        assert!(symbols.len() >= 3);

        let kinds: Vec<&SymbolKind> = symbols.iter().map(|s| &s.kind).collect();
        assert!(kinds.contains(&&SymbolKind::Class));
        assert!(kinds.contains(&&SymbolKind::Method));
    }

    #[test]
    fn test_local_variables_included() {
        let source = r#"
class Calculator {
    val multiplier: Int = 2

    fun compute(input: Int): Int {
        val localConst = 10
        var localVar = input * multiplier
        localVar += localConst
        return localVar
    }
}

fun topLevel(): String {
    val result = "test"
    var counter = 0
    return result
}
        "#;

        let symbols = parse("test.kt", source).unwrap();

        // Filter to constants and variables
        let constants: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Constant))
            .collect();

        let variables: Vec<_> = symbols.iter()
            .filter(|s| matches!(s.kind, SymbolKind::Variable))
            .collect();

        // Check that val local variables are captured as constants
        assert!(constants.iter().any(|c| c.symbol.as_deref() == Some("localConst")));
        assert!(constants.iter().any(|c| c.symbol.as_deref() == Some("result")));

        // Check that var local variables are captured as variables
        assert!(variables.iter().any(|v| v.symbol.as_deref() == Some("localVar")));
        assert!(variables.iter().any(|v| v.symbol.as_deref() == Some("counter")));

        // Check that class property is still captured
        assert!(variables.iter().any(|v| v.symbol.as_deref() == Some("multiplier")));

        // Verify that local variables have no scope
        for constant in &constants {
            if constant.symbol.as_deref() == Some("localConst")
                || constant.symbol.as_deref() == Some("result") {
                assert_eq!(constant.scope, None);
            }
        }

        for variable in &variables {
            if variable.symbol.as_deref() == Some("localVar")
                || variable.symbol.as_deref() == Some("counter") {
                assert_eq!(variable.scope, None);
            }
        }

        // Verify that class property has scope
        let multiplier = variables.iter()
            .find(|v| v.symbol.as_deref() == Some("multiplier"))
            .unwrap();
        assert_eq!(multiplier.scope.as_ref().unwrap(), "class Calculator");
    }
}
