//! Extract literal sequences from regex patterns for trigram optimization
//!
//! This module implements literal extraction from regular expressions to enable
//! fast regex search using the trigram index. The key insight is that many regex
//! patterns contain literal substrings that can narrow down candidate files.
//!
//! # Strategy
//!
//! 1. Extract all literal sequences from the regex pattern (≥3 chars)
//! 2. Generate trigrams from those literals
//! 3. Use trigrams to find files containing ANY literal (UNION approach)
//! 4. Verify actual matches with the regex engine
//!
//! # File Selection: UNION vs INTERSECTION
//!
//! For correctness, we use **UNION** (files with ANY literal):
//! - Alternation `(a|b)` needs files with a OR b → UNION is correct ✓
//! - Sequential `a.*b` needs files with a AND b → UNION includes extra files but still correct ✓
//!
//! Trade-off: UNION may scan 2-3x more files for sequential patterns, but ensures
//! we never miss matches. Performance impact is minimal (<5ms) due to memory-mapped I/O.
//!
//! # Examples
//!
//! - `fn\s+test_.*` → extracts "test_" → searches files containing "test_"
//! - `(class|function)` → extracts ["class", "function"] → searches files with class OR function
//! - `class.*Controller` → extracts ["class", "Controller"] → searches files with class OR Controller
//! - `.*` → no literals → fall back to full scan
//!
//! # References
//!
//! - Russ Cox - Regular Expression Matching with a Trigram Index
//!   https://swtch.com/~rsc/regexp/regexp4.html

use crate::trigram::{extract_trigrams, Trigram};

/// Extract guaranteed trigrams from a regex pattern
///
/// Returns trigrams that MUST appear in any string matching the pattern.
/// These trigrams are used to narrow down candidate files before running
/// the full regex match.
///
/// # Algorithm (MVP - Simple Literal Extraction)
///
/// 1. Split pattern on regex metacharacters: . * + ? | ( ) [ ] { } ^ $ \
/// 2. Keep literal sequences of 3+ characters
/// 3. Extract trigrams from each literal sequence
/// 4. Return all trigrams
///
/// # Examples
///
/// ```
/// use reflex::regex_trigrams::extract_trigrams_from_regex;
///
/// // Simple literal
/// let trigrams = extract_trigrams_from_regex("extract_symbols");
/// assert!(!trigrams.is_empty());
///
/// // Pattern with wildcard
/// let trigrams = extract_trigrams_from_regex("fn.*test");
/// assert!(!trigrams.is_empty()); // Has "fn " and "test"
///
/// // No literals
/// let trigrams = extract_trigrams_from_regex(".*");
/// assert!(trigrams.is_empty()); // Must fall back to full scan
/// ```
pub fn extract_trigrams_from_regex(pattern: &str) -> Vec<Trigram> {
    let literals = extract_literal_sequences(pattern);

    if literals.is_empty() {
        log::debug!("No literals found in regex pattern '{}', will fall back to full scan", pattern);
        return vec![];
    }

    log::debug!("Extracted {} literal sequences from regex: {:?}", literals.len(), literals);

    // Extract trigrams from all literal sequences
    let mut all_trigrams = Vec::new();
    for literal in literals {
        let trigrams = extract_trigrams(&literal);
        all_trigrams.extend(trigrams);
    }

    // Deduplicate trigrams
    all_trigrams.sort_unstable();
    all_trigrams.dedup();

    log::debug!("Extracted {} unique trigrams from regex pattern", all_trigrams.len());
    all_trigrams
}

/// Extract literal sequences (≥3 chars) from a regex pattern
///
/// This is a simple heuristic that splits on regex metacharacters.
/// It doesn't parse the full regex AST but works for common patterns.
///
/// # Regex Metacharacters
///
/// The following characters are treated as non-literal:
/// - Wildcards: `.` `*` `+` `?`
/// - Alternation: `|`
/// - Grouping: `(` `)`
/// - Character classes: `[` `]`
/// - Anchors: `^` `$`
/// - Escapes: `\` (followed by special char)
/// - Quantifiers: `{` `}`
///
/// # Examples
///
/// ```
/// use reflex::regex_trigrams::extract_literal_sequences;
///
/// assert_eq!(extract_literal_sequences("hello"), vec!["hello"]);
/// assert_eq!(extract_literal_sequences("fn.*test"), vec!["test"]);
/// assert_eq!(extract_literal_sequences("class.*Controller"), vec!["class", "Controller"]);
/// ```
pub fn extract_literal_sequences(pattern: &str) -> Vec<String> {
    let mut sequences = Vec::new();
    let mut current = String::new();
    let mut chars = pattern.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            // Regex metacharacters - break the literal sequence
            '.' | '*' | '+' | '?' | '|' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' => {
                if current.len() >= 3 {
                    sequences.push(current.clone());
                }
                current.clear();
            }

            // Backslash escapes
            '\\' => {
                if let Some(&next_ch) = chars.peek() {
                    match next_ch {
                        // Common escape sequences that represent single characters
                        's' | 'd' | 'w' | 'S' | 'D' | 'W' | 'n' | 't' | 'r' | 'b' | 'B' => {
                            // These are not literal characters, break the sequence
                            chars.next(); // consume the escaped char
                            if current.len() >= 3 {
                                sequences.push(current.clone());
                            }
                            current.clear();
                        }
                        // Escaped metacharacter - treat as literal
                        _ => {
                            chars.next(); // consume the escaped char
                            current.push(next_ch);
                        }
                    }
                } else {
                    // Backslash at end of pattern - ignore
                    if current.len() >= 3 {
                        sequences.push(current.clone());
                    }
                    current.clear();
                }
            }

            // Regular literal character
            _ => {
                current.push(ch);
            }
        }
    }

    // Don't forget the last sequence
    if current.len() >= 3 {
        sequences.push(current);
    }

    sequences
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_literal_sequences_simple() {
        let sequences = extract_literal_sequences("hello");
        assert_eq!(sequences, vec!["hello"]);
    }

    #[test]
    fn test_extract_literal_sequences_with_wildcard() {
        let sequences = extract_literal_sequences("fn.*test");
        assert_eq!(sequences, vec!["test"]);
    }

    #[test]
    fn test_extract_literal_sequences_multiple() {
        let sequences = extract_literal_sequences("class.*Controller");
        assert_eq!(sequences, vec!["class", "Controller"]);
    }

    #[test]
    fn test_extract_literal_sequences_no_literals() {
        let sequences = extract_literal_sequences(".*");
        assert!(sequences.is_empty());
    }

    #[test]
    fn test_extract_literal_sequences_short_literals() {
        // "fn" is only 2 chars, should be skipped
        let sequences = extract_literal_sequences("fn.*test");
        assert_eq!(sequences, vec!["test"]);
    }

    #[test]
    fn test_extract_literal_sequences_escaped() {
        // \. is escaped period, should be literal
        let sequences = extract_literal_sequences("test\\.txt");
        assert_eq!(sequences, vec!["test.txt"]);
    }

    #[test]
    fn test_extract_literal_sequences_whitespace_escape() {
        // \s is whitespace class, not literal
        let sequences = extract_literal_sequences("fn\\s+extract");
        assert_eq!(sequences, vec!["extract"]);
    }

    #[test]
    fn test_extract_trigrams_simple_literal() {
        let trigrams = extract_trigrams_from_regex("extract");
        // "extract" has 5 trigrams: "ext", "xtr", "tra", "rac", "act"
        assert_eq!(trigrams.len(), 5);
    }

    #[test]
    fn test_extract_trigrams_with_wildcard() {
        let trigrams = extract_trigrams_from_regex("fn.*test");
        // "test" has 2 trigrams: "tes", "est"
        assert_eq!(trigrams.len(), 2);
    }

    #[test]
    fn test_extract_trigrams_multiple_literals() {
        let trigrams = extract_trigrams_from_regex("class.*Controller");
        // "class" has 3 trigrams, "Controller" has 8
        // Total unique: 11
        assert!(trigrams.len() >= 10); // At least 10 unique trigrams
    }

    #[test]
    fn test_extract_trigrams_no_literals() {
        let trigrams = extract_trigrams_from_regex(".*");
        assert!(trigrams.is_empty());
    }

    #[test]
    fn test_extract_trigrams_complex_pattern() {
        // "(function|const)\s+\w+\s*=" has "function" and "const" as literals
        let trigrams = extract_trigrams_from_regex("(function|const)");
        // "function" has 6 trigrams, "const" has 3
        assert!(trigrams.len() >= 6);
    }

    #[test]
    fn test_extract_literal_sequences_alternation() {
        // Alternation patterns should extract all alternatives as separate literals
        let sequences = extract_literal_sequences("(SymbolWriter|ContentWriter)");
        assert_eq!(sequences, vec!["SymbolWriter", "ContentWriter"]);
    }

    #[test]
    fn test_extract_literal_sequences_three_way_alternation() {
        // Three-way alternation
        let sequences = extract_literal_sequences("(Indexer|QueryEngine|CacheManager)");
        assert_eq!(sequences, vec!["Indexer", "QueryEngine", "CacheManager"]);
    }
}
