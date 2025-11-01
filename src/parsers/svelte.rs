//! Svelte component parser
//!
//! Extracts symbols from Svelte components:
//! - Component script exports
//! - Functions and methods
//! - Reactive declarations ($:)
//! - Variables and constants
//! - Module context exports
//!
//! Svelte components contain HTML-like templates mixed with JavaScript/TypeScript.
//! This parser extracts symbols from script sections.
//!
//! Note: This parser uses regex-based extraction for script blocks since
//! tree-sitter-svelte is not compatible with tree-sitter 0.24+.

use anyhow::{Context, Result};
use crate::models::{Language, SearchResult, Span, SymbolKind};
use tree_sitter::{Parser, Query, QueryCursor};
use streaming_iterator::StreamingIterator;

/// Parse Svelte component and extract symbols
pub fn parse(path: &str, source: &str) -> Result<Vec<SearchResult>> {
    let mut symbols = Vec::new();

    // Extract script blocks using line-based parsing
    let script_blocks = extract_script_blocks(source)?;

    // Parse each script block with the TypeScript parser
    for (script_source, script_offset, is_module) in script_blocks {
        let mut script_symbols = parse_script_block(path, &script_source, script_offset)?;

        // Mark module context symbols with scope
        if is_module {
            for symbol in &mut script_symbols {
                symbol.scope = Some("module context".to_string());
            }
        }

        symbols.extend(script_symbols);
    }

    Ok(symbols)
}

/// Extract script blocks from Svelte component using line-based parsing
/// Returns (source_code, line_offset, is_module_context) for each script block
fn extract_script_blocks(source: &str) -> Result<Vec<(String, usize, bool)>> {
    let mut script_blocks = Vec::new();

    // Find all <script> blocks
    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        // Check if this line starts a script tag
        if line.trim_start().starts_with("<script") {
            // Check if this is a module context script
            let is_module = line.contains("context=\"module\"");

            // Find the end of the opening tag
            let mut tag_line = i;
            let mut tag_end_found = false;

            while tag_line < lines.len() {
                if lines[tag_line].contains('>') {
                    tag_end_found = true;
                    break;
                }
                tag_line += 1;
            }

            if !tag_end_found {
                i += 1;
                continue;
            }

            // Find the closing </script> tag
            let mut close_line = tag_line + 1;
            let mut close_found = false;

            while close_line < lines.len() {
                if lines[close_line].trim_start().starts_with("</script>") {
                    close_found = true;
                    break;
                }
                close_line += 1;
            }

            if close_found {
                // Extract the script content (lines between opening and closing tags)
                let script_start = tag_line + 1;
                let script_end = close_line;

                if script_start < script_end {
                    let script_content = lines[script_start..script_end].join("\n");
                    script_blocks.push((script_content, script_start, is_module));
                }

                i = close_line + 1;
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }

    Ok(script_blocks)
}

/// Parse a script block using TypeScript parser
fn parse_script_block(
    path: &str,
    script_source: &str,
    line_offset: usize,
) -> Result<Vec<SearchResult>> {
    let mut parser = Parser::new();

    // Use TSX parser to handle both TypeScript and JavaScript
    let ts_language: tree_sitter::Language = tree_sitter_typescript::LANGUAGE_TSX.into();

    parser
        .set_language(&ts_language)
        .context("Failed to set TypeScript language for script block")?;

    let tree = parser
        .parse(script_source, None)
        .context("Failed to parse script block")?;

    let root_node = tree.root_node();

    let mut symbols = Vec::new();

    // Extract symbols from the script block
    symbols.extend(extract_functions(script_source, &root_node, &ts_language, line_offset)?);
    symbols.extend(extract_arrow_functions(script_source, &root_node, &ts_language, line_offset)?);
    symbols.extend(extract_variables(script_source, &root_node, &ts_language, line_offset)?);
    symbols.extend(extract_reactive_declarations(script_source, &root_node, &ts_language, line_offset)?);

    // Add file path and language to all symbols
    for symbol in &mut symbols {
        symbol.path = path.to_string();
        symbol.lang = Language::Svelte;
    }

    Ok(symbols)
}


/// Extract regular function declarations
fn extract_functions(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
    line_offset: usize,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (function_declaration
            name: (identifier) @name) @function
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create function query")?;

    extract_symbols(source, root, &query, SymbolKind::Function, None, line_offset)
}

/// Extract arrow functions
fn extract_arrow_functions(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
    line_offset: usize,
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

    extract_symbols(source, root, &query, SymbolKind::Function, None, line_offset)
}

/// Extract variable and constant declarations
fn extract_variables(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
    line_offset: usize,
) -> Result<Vec<SearchResult>> {
    let query_str = r#"
        (lexical_declaration
            (variable_declarator
                name: (identifier) @name)) @const

        (variable_declaration
            (variable_declarator
                name: (identifier) @name)) @var
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create variable query")?;

    // Filter to only declarations that are not arrow functions
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
                if let Some(parent) = capture.node.parent() {
                    if parent.kind() == "variable_declarator" {
                        declarator_node = Some(parent);
                    }
                }
            }
        }

        if let (Some(name), Some(declarator)) = (name, declarator_node) {
            if let Some(parent_decl) = declarator.parent() {
                // Skip arrow functions (they're handled separately)
                let mut is_arrow_function = false;
                for i in 0..declarator.child_count() {
                    if let Some(child) = declarator.child(i) {
                        if child.kind() == "arrow_function" {
                            is_arrow_function = true;
                            break;
                        }
                    }
                }

                if !is_arrow_function {
                    let kind = if parent_decl.kind() == "lexical_declaration" {
                        SymbolKind::Constant
                    } else {
                        SymbolKind::Variable
                    };

                    let span = node_to_span(&parent_decl, line_offset);
                    let preview = extract_preview(source, &span, line_offset);

                    symbols.push(SearchResult::new(
                        String::new(),
                        Language::Svelte,
                        kind,
                        name,
                        span,
                        None,
                        preview,
                    ));
                }
            }
        }
    }

    Ok(symbols)
}

/// Extract Svelte reactive declarations ($: syntax)
fn extract_reactive_declarations(
    source: &str,
    root: &tree_sitter::Node,
    language: &tree_sitter::Language,
    line_offset: usize,
) -> Result<Vec<SearchResult>> {
    // Reactive declarations in Svelte use the label statement syntax with $: label
    let query_str = r#"
        (labeled_statement
            label: (statement_identifier) @label
            (expression_statement
                (assignment_expression
                    left: (identifier) @name))) @reactive
    "#;

    let query = Query::new(language, query_str)
        .context("Failed to create reactive declaration query")?;

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, *root, source.as_bytes());

    let mut symbols = Vec::new();

    while let Some(match_) = matches.next() {
        let mut label = None;
        let mut name = None;
        let mut full_node = None;

        for capture in match_.captures {
            let capture_name: &str = &query.capture_names()[capture.index as usize];
            match capture_name {
                "label" => {
                    label = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                }
                "name" => {
                    name = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
                }
                "reactive" => {
                    full_node = Some(capture.node);
                }
                _ => {}
            }
        }

        // Only extract if the label is $ (Svelte reactive declaration)
        if let (Some(label_text), Some(name), Some(node)) = (label, name, full_node) {
            if label_text == "$" {
                let span = node_to_span(&node, line_offset);
                let preview = extract_preview(source, &span, line_offset);

                symbols.push(SearchResult::new(
                    String::new(),
                    Language::Svelte,
                    SymbolKind::Variable,
                    name,
                    span,
                    Some("reactive".to_string()),
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
    line_offset: usize,
) -> Result<Vec<SearchResult>> {
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(query, *root, source.as_bytes());

    let mut symbols = Vec::new();

    while let Some(match_) = matches.next() {
        let mut name = None;
        let mut full_node = None;

        for capture in match_.captures {
            let capture_name: &str = &query.capture_names()[capture.index as usize];
            if capture_name == "name" {
                name = Some(capture.node.utf8_text(source.as_bytes()).unwrap_or("").to_string());
            } else {
                full_node = Some(capture.node);
            }
        }

        if let (Some(name), Some(node)) = (name, full_node) {
            let span = node_to_span(&node, line_offset);
            let preview = extract_preview(source, &span, line_offset);

            symbols.push(SearchResult::new(
                String::new(),
                Language::Svelte,
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

/// Convert a Tree-sitter node to a Span with line offset
fn node_to_span(node: &tree_sitter::Node, line_offset: usize) -> Span {
    let start = node.start_position();
    let end = node.end_position();

    Span::new(
        start.row + 1 + line_offset,
        start.column,
        end.row + 1 + line_offset,
        end.column,
    )
}

/// Extract a preview (7 lines) around the symbol
fn extract_preview(source: &str, span: &Span, line_offset: usize) -> String {
    let lines: Vec<&str> = source.lines().collect();

    // Adjust for the line offset - we're working with the script block content
    let start_idx = (span.start_line - 1 - line_offset) as usize;
    let end_idx = (start_idx + 7).min(lines.len());

    lines[start_idx..end_idx].join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_svelte_component() {
        let source = r#"
<script>
  let count = 0;

  function increment() {
    count += 1;
  }
</script>

<button on:click={increment}>
  Count: {count}
</button>
"#;

        let symbols = parse("test.svelte", source).unwrap();
        assert!(symbols.iter().any(|s| s.symbol == "count"));
        assert!(symbols.iter().any(|s| s.symbol == "increment"));
    }

    #[test]
    fn test_parse_svelte_reactive_declaration() {
        let source = r#"
<script>
  let count = 0;
  $: doubled = count * 2;

  function increment() {
    count += 1;
  }
</script>

<div>Doubled: {doubled}</div>
"#;

        let symbols = parse("test.svelte", source).unwrap();
        assert!(symbols.iter().any(|s| s.symbol == "count"));
        assert!(symbols.iter().any(|s| s.symbol == "doubled" && s.scope == Some("reactive".to_string())));
        assert!(symbols.iter().any(|s| s.symbol == "increment"));
    }

    #[test]
    fn test_parse_svelte_module_context() {
        let source = r#"
<script context="module">
  export const API_URL = "https://api.example.com";
</script>

<script>
  let data = null;

  async function fetchData() {
    const response = await fetch(API_URL);
    data = await response.json();
  }
</script>

<div>{data}</div>
"#;

        let symbols = parse("test.svelte", source).unwrap();

        // Should have module context symbol
        let module_symbols: Vec<_> = symbols.iter()
            .filter(|s| s.scope == Some("module context".to_string()))
            .collect();
        assert!(module_symbols.len() > 0);

        // Should have component symbols
        assert!(symbols.iter().any(|s| s.symbol == "data"));
        assert!(symbols.iter().any(|s| s.symbol == "fetchData"));
    }

    #[test]
    fn test_parse_svelte_typescript() {
        let source = r#"
<script lang="ts">
  interface User {
    name: string;
    age: number;
  }

  let user: User = {
    name: 'Alice',
    age: 30
  };
</script>

<div>{user.name}</div>
"#;

        let symbols = parse("test.svelte", source).unwrap();
        assert!(symbols.iter().any(|s| s.symbol == "user"));
    }
}
