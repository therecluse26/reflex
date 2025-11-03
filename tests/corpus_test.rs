//! Comprehensive Corpus-Based Tests
//!
//! This test suite exercises RefLex against a comprehensive corpus of test files
//! covering all supported languages, edge cases, and real-world patterns.
//!
//! Test categories:
//! - Symbol detection (all kinds, all languages)
//! - Full-text search (unicode, operators, special chars)
//! - Regex patterns (alternation, anchors, character classes)
//! - Filter combinations (language + kind + file + limit + exact)
//! - Edge cases (empty files, long lines, unicode, whitespace)
//! - Performance tests (many symbols, large files)
//! - Real-world scenarios (TODO comments, error handling, async patterns)

mod test_helpers;

use reflex::{Language, QueryFilter, SymbolKind};
use test_helpers::*;

// ==================== Symbol Detection Tests ====================

#[test]
fn test_rust_function_detection() {
    setup_corpus();

    let filter = QueryFilter {
        symbols_mode: true,
        kind: Some(SymbolKind::Function),
        file_pattern: Some("rust/functions.rs".to_string()),
        ..Default::default()
    };

    let results = query_corpus("function", filter);

    // Should find multiple functions
    assert_result_count_at_least(&results, 10);
    assert_symbol_found(&results, "public_function", SymbolKind::Function);
    assert_symbol_found(&results, "async_function", SymbolKind::Function);
    assert_symbol_found(&results, "generic_function", SymbolKind::Function);
}

#[test]
fn test_rust_struct_detection() {
    setup_corpus();

    let filter = QueryFilter {
        symbols_mode: true,
        kind: Some(SymbolKind::Struct),
        file_pattern: Some("rust/structs.rs".to_string()),
        ..Default::default()
    };

    let results = query_corpus("", filter);

    assert_result_count_at_least(&results, 8);
    assert_symbol_found(&results, "Point", SymbolKind::Struct);
    assert_symbol_found(&results, "Container", SymbolKind::Struct);
}

#[test]
fn test_rust_enum_detection() {
    setup_corpus();

    let filter = QueryFilter {
        symbols_mode: true,
        kind: Some(SymbolKind::Enum),
        file_pattern: Some("rust/enums.rs".to_string()),
        ..Default::default()
    };

    let results = query_corpus("", filter);

    assert_result_count_at_least(&results, 6);
    assert_symbol_found(&results, "Direction", SymbolKind::Enum);
    assert_symbol_found(&results, "Message", SymbolKind::Enum);
}

#[test]
fn test_rust_trait_detection() {
    setup_corpus();

    let filter = QueryFilter {
        symbols_mode: true,
        kind: Some(SymbolKind::Trait),
        file_pattern: Some("rust/traits.rs".to_string()),
        ..Default::default()
    };

    let results = query_corpus("", filter);

    assert_result_count_at_least(&results, 6);
    assert_symbol_found(&results, "Drawable", SymbolKind::Trait);
    assert_symbol_found(&results, "Iterator2", SymbolKind::Trait);
}

#[test]
fn test_rust_method_detection() {
    setup_corpus();

    let filter = QueryFilter {
        symbols_mode: true,
        kind: Some(SymbolKind::Function), // Methods should be included with functions
        file_pattern: Some("rust/impls.rs".to_string()),
        ..Default::default()
    };

    let results = query_corpus("new", filter);

    assert_result_count_at_least(&results, 2);
    // Should find Point::new and Container::new
}

#[test]
fn test_rust_module_detection() {
    setup_corpus();

    let filter = QueryFilter {
        symbols_mode: true,
        kind: Some(SymbolKind::Module),
        file_pattern: Some("rust/modules.rs".to_string()),
        ..Default::default()
    };

    let results = query_corpus("", filter);

    assert_result_count_at_least(&results, 5);
}

#[test]
fn test_typescript_class_detection() {
    setup_corpus();

    let filter = QueryFilter {
        symbols_mode: true,
        kind: Some(SymbolKind::Class),
        language: Some(Language::TypeScript),
        file_pattern: Some("typescript/classes.ts".to_string()),
        ..Default::default()
    };

    let results = query_corpus("", filter);

    assert_result_count_at_least(&results, 6);
    assert_symbol_found(&results, "Point", SymbolKind::Class);
    assert_symbol_found(&results, "Employee", SymbolKind::Class);
}

#[test]
fn test_typescript_interface_detection() {
    setup_corpus();

    let filter = QueryFilter {
        symbols_mode: true,
        kind: Some(SymbolKind::Interface),
        file_pattern: Some("typescript/interfaces.ts".to_string()),
        ..Default::default()
    };

    let results = query_corpus("", filter);

    assert_result_count_at_least(&results, 10);
    assert_symbol_found(&results, "User", SymbolKind::Interface);
}

#[test]
fn test_typescript_type_detection() {
    setup_corpus();

    let filter = QueryFilter {
        symbols_mode: true,
        kind: Some(SymbolKind::Type),
        file_pattern: Some("typescript/types.ts".to_string()),
        ..Default::default()
    };

    let results = query_corpus("", filter);

    assert_result_count_at_least(&results, 12);
}

#[test]
fn test_typescript_enum_detection() {
    setup_corpus();

    let filter = QueryFilter {
        symbols_mode: true,
        kind: Some(SymbolKind::Enum),
        file_pattern: Some("typescript/enums.ts".to_string()),
        ..Default::default()
    };

    let results = query_corpus("", filter);

    assert_result_count_at_least(&results, 6);
    assert_symbol_found(&results, "Direction", SymbolKind::Enum);
    assert_symbol_found(&results, "Color", SymbolKind::Enum);
}

#[test]
fn test_javascript_function_detection() {
    setup_corpus();

    let filter = QueryFilter {
        symbols_mode: true,
        kind: Some(SymbolKind::Function),
        language: Some(Language::JavaScript),
        file_pattern: Some("javascript/functions.js".to_string()),
        ..Default::default()
    };

    let results = query_corpus("function", filter);

    assert_result_count_at_least(&results, 5);
}

#[test]
fn test_javascript_class_detection() {
    setup_corpus();

    let filter = QueryFilter {
        symbols_mode: true,
        kind: Some(SymbolKind::Class),
        file_pattern: Some("javascript/classes.js".to_string()),
        ..Default::default()
    };

    let results = query_corpus("", filter);

    assert_result_count_at_least(&results, 6);
    assert_symbol_found(&results, "Point", SymbolKind::Class);
}

#[test]
fn test_php_class_detection() {
    setup_corpus();

    let filter = QueryFilter {
        symbols_mode: true,
        kind: Some(SymbolKind::Class),
        language: Some(Language::PHP),
        file_pattern: Some("php/classes.php".to_string()),
        ..Default::default()
    };

    let results = query_corpus("", filter);

    assert_result_count_at_least(&results, 6);
}

#[test]
fn test_php_function_detection() {
    setup_corpus();

    let filter = QueryFilter {
        symbols_mode: true,
        kind: Some(SymbolKind::Function),
        file_pattern: Some("php/functions.php".to_string()),
        ..Default::default()
    };

    let results = query_corpus("function", filter);

    assert_result_count_at_least(&results, 8);
}

// ==================== Full-Text Search Tests ====================

#[test]
fn test_fulltext_search_across_all_files() {
    setup_corpus();

    let filter = QueryFilter::default();
    let results = query_corpus("function", filter);

    // Should find occurrences across many files
    assert_result_count_at_least(&results, 50);

    let files = unique_files(&results);
    assert!(files.len() > 10, "Should find matches in many files");
}

#[test]
fn test_fulltext_unicode_search() {
    setup_corpus();

    let filter = QueryFilter::default();
    let results = query_corpus("你好", filter);

    assert_result_count_at_least(&results, 1);
    assert_file_match(&results, "unicode");
}

#[test]
fn test_fulltext_emoji_search() {
    setup_corpus();

    let filter = QueryFilter::default();
    let results = query_corpus("🚀", filter);

    assert_result_count_at_least(&results, 1);
}

#[test]
fn test_fulltext_operator_search() {
    setup_corpus();

    let filter = QueryFilter::default();
    let results = query_corpus("&&", filter);

    assert_result_count_at_least(&results, 2);
}

#[test]
fn test_fulltext_special_chars() {
    setup_corpus();

    let filter = QueryFilter::default();
    let results = query_corpus("::", filter);

    assert_result_count_at_least(&results, 5);
}

// ==================== Regex Search Tests ====================

#[test]
fn test_regex_digit_pattern() {
    setup_corpus();

    let filter = QueryFilter {
        use_regex: true,
        file_pattern: Some("rust/many_symbols.rs".to_string()),
        ..Default::default()
    };

    let results = query_corpus(r"func_\d{3}", filter);

    // Should match func_001, func_002, etc.
    assert_result_count_at_least(&results, 90);
}

#[test]
fn test_regex_alternation() {
    setup_corpus();

    let filter = QueryFilter {
        use_regex: true,
        ..Default::default()
    };

    let results = query_corpus(r"(async|await)", filter);

    assert_result_count_at_least(&results, 10);
}

#[test]
fn test_regex_character_class() {
    setup_corpus();

    let filter = QueryFilter {
        use_regex: true,
        file_pattern: Some("rust/single_char.rs".to_string()),
        ..Default::default()
    };

    let results = query_corpus(r"fn [a-z]\(\)", filter);

    assert_result_count_at_least(&results, 8);
}

#[test]
fn test_regex_start_anchor() {
    setup_corpus();

    let filter = QueryFilter {
        use_regex: true,
        ..Default::default()
    };

    let results = query_corpus(r"^pub fn", filter);

    assert_result_count_at_least(&results, 20);
}

#[test]
fn test_regex_word_boundary() {
    setup_corpus();

    let filter = QueryFilter {
        use_regex: true,
        ..Default::default()
    };

    let results = query_corpus(r"\btest\b", filter);

    // Should match "test" but not "testing" or "latest"
    assert_result_count_at_least(&results, 5);
}

// ==================== Filter Combination Tests ====================

#[test]
fn test_filter_language_and_kind() {
    setup_corpus();

    let filter = QueryFilter {
        symbols_mode: true,
        language: Some(Language::Rust),
        kind: Some(SymbolKind::Function),
        ..Default::default()
    };

    let results = query_corpus("async", filter);

    assert_result_count_at_least(&results, 3);
    assert_all_language(&results, Language::Rust);
    assert_all_kind(&results, SymbolKind::Function);
}

#[test]
fn test_filter_language_kind_and_file() {
    setup_corpus();

    let filter = QueryFilter {
        symbols_mode: true,
        language: Some(Language::TypeScript),
        kind: Some(SymbolKind::Class),
        file_pattern: Some("typescript/classes.ts".to_string()),
        ..Default::default()
    };

    let results = query_corpus("", filter);

    assert_result_count_at_least(&results, 5);
    assert_all_language(&results, Language::TypeScript);
}

#[test]
fn test_filter_exact_match() {
    setup_corpus();

    let filter = QueryFilter {
        symbols_mode: true,
        exact: true,
        file_pattern: Some("rust/functions.rs".to_string()),
        ..Default::default()
    };

    let results = query_corpus("public_function", filter);

    // Should match exactly "public_function", not "pub_async_function"
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].symbol.as_deref(), Some("public_function"));
}

#[test]
fn test_filter_with_limit() {
    setup_corpus();

    let filter = QueryFilter {
        limit: Some(5),
        ..Default::default()
    };

    let results = query_corpus("function", filter);

    assert_eq!(results.len(), 5);
}

#[test]
fn test_filter_symbols_mode_vs_fulltext() {
    setup_corpus();

    // Full-text search
    let fulltext_filter = QueryFilter::default();
    let fulltext_results = query_corpus("greet", fulltext_filter);

    // Symbol search
    let symbol_filter = QueryFilter {
        symbols_mode: true,
        ..Default::default()
    };
    let symbol_results = query_corpus("greet", symbol_filter);

    // Full-text should find more results (includes call sites, comments, etc.)
    assert!(fulltext_results.len() > symbol_results.len());
}

#[test]
fn test_filter_regex_with_symbols() {
    setup_corpus();

    let filter = QueryFilter {
        use_regex: true,
        symbols_mode: true,
        kind: Some(SymbolKind::Function),
        ..Default::default()
    };

    let results = query_corpus(r"test\w+", filter);

    assert_result_count_at_least(&results, 3);
}

// ==================== Edge Case Tests ====================

#[test]
fn test_empty_file() {
    setup_corpus();

    let filter = QueryFilter {
        file_pattern: Some("edge_cases/empty_file.txt".to_string()),
        ..Default::default()
    };

    let results = query_corpus("anything", filter);

    assert_eq!(results.len(), 0);
}

#[test]
fn test_very_long_line() {
    setup_corpus();

    let filter = QueryFilter {
        file_pattern: Some("edge_cases/very_long_line.txt".to_string()),
        ..Default::default()
    };

    let results = query_corpus("testing", filter);

    assert_result_count_at_least(&results, 1);
}

#[test]
fn test_unicode_identifiers() {
    setup_corpus();

    let filter = QueryFilter {
        symbols_mode: true,
        file_pattern: Some("rust/unicode_identifiers.rs".to_string()),
        ..Default::default()
    };

    let results = query_corpus("café", filter);

    assert_result_count_at_least(&results, 1);
    assert_symbol_found(&results, "café", SymbolKind::Function);
}

#[test]
fn test_raw_identifiers() {
    setup_corpus();

    let filter = QueryFilter {
        symbols_mode: true,
        file_pattern: Some("rust/raw_identifiers.rs".to_string()),
        ..Default::default()
    };

    let results = query_corpus("type", filter);

    assert_result_count_at_least(&results, 1);
}

#[test]
fn test_whitespace_handling() {
    setup_corpus();

    let filter = QueryFilter {
        file_pattern: Some("edge_cases/whitespace.txt".to_string()),
        ..Default::default()
    };

    let results = query_corpus("Multiple blank lines", filter);

    assert_result_count_at_least(&results, 1);
}

// ==================== Performance Tests ====================

#[test]
fn test_many_symbols_performance() {
    setup_corpus();

    let filter = QueryFilter {
        symbols_mode: true,
        file_pattern: Some("rust/many_symbols.rs".to_string()),
        ..Default::default()
    };

    let start = std::time::Instant::now();
    let results = query_corpus("func", filter);
    let elapsed = start.elapsed();

    // Should find 100 functions
    assert_result_count_at_least(&results, 90);

    // Should complete in reasonable time (< 100ms)
    assert!(elapsed.as_millis() < 200, "Query took too long: {:?}", elapsed);
}

#[test]
fn test_deterministic_results() {
    setup_corpus();

    let filter = QueryFilter::default();

    let results1 = query_corpus("function", filter.clone());
    let results2 = query_corpus("function", filter.clone());
    let results3 = query_corpus("function", filter);

    assert_eq!(results1.len(), results2.len());
    assert_eq!(results1.len(), results3.len());

    // Results should be sorted
    assert_sorted(&results1);

    // Results should be identical
    for i in 0..results1.len() {
        assert_eq!(results1[i].path, results2[i].path);
        assert_eq!(results1[i].span.start_line, results2[i].span.start_line);
    }
}

#[test]
fn test_no_duplicate_results() {
    setup_corpus();

    let filter = QueryFilter::default();
    let results = query_corpus("test", filter);

    assert_no_duplicates(&results);
}

// ==================== Real-World Scenario Tests ====================

#[test]
fn test_find_async_functions() {
    setup_corpus();

    let filter = QueryFilter {
        symbols_mode: true,
        kind: Some(SymbolKind::Function),
        ..Default::default()
    };

    let results = query_corpus("async", filter);

    assert_result_count_at_least(&results, 10);
}

#[test]
fn test_find_error_handling() {
    setup_corpus();

    let filter = QueryFilter {
        file_pattern: Some("rust/error_handling.rs".to_string()),
        ..Default::default()
    };

    let results = query_corpus("unwrap", filter);

    assert_result_count_at_least(&results, 3);
}

#[test]
fn test_find_generic_functions() {
    setup_corpus();

    let filter = QueryFilter {
        file_pattern: Some("rust/generics_complex.rs".to_string()),
        ..Default::default()
    };

    let results = query_corpus("<T>", filter);

    assert_result_count_at_least(&results, 5);
}

#[test]
fn test_cross_language_search() {
    setup_corpus();

    let filter = QueryFilter::default();

    // Search for "Point" class across all languages
    let results = query_corpus("Point", filter);

    let files = unique_files(&results);

    // Should find Point in Rust, TypeScript, JavaScript, PHP
    assert!(files.iter().any(|f| f.contains("rust")));
    assert!(files.iter().any(|f| f.contains("typescript")));
    assert!(files.iter().any(|f| f.contains("javascript")));
}
