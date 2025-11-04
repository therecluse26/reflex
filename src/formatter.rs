//! Terminal output formatting for query results
//!
//! This module provides beautiful, syntax-highlighted terminal output using ratatui.
//! It supports both static output (print and exit) and prepares for future interactive mode.

use anyhow::Result;
use crossterm::tty::IsTty;
use std::collections::HashMap;
use std::io;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style as SyntectStyle, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

use crate::models::{Language, SearchResult, SymbolKind};

/// Lazy-loaded syntax highlighting resources
struct SyntaxHighlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl SyntaxHighlighter {
    fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }

    /// Get syntax reference for a language
    fn get_syntax(&self, lang: &Language) -> Option<&syntect::parsing::SyntaxReference> {
        let name = match lang {
            Language::Rust => "Rust",
            Language::Python => "Python",
            Language::JavaScript => "JavaScript",
            Language::TypeScript => "TypeScript",
            Language::Go => "Go",
            Language::Java => "Java",
            Language::C => "C",
            Language::Cpp => "C++",
            Language::CSharp => "C#",
            Language::PHP => "PHP",
            Language::Ruby => "Ruby",
            Language::Kotlin => "Kotlin",
            Language::Swift => "Swift",
            Language::Zig => "Zig",
            Language::Vue => "Vue Component",
            Language::Svelte => "Svelte",
            Language::Unknown => return None,
        };

        self.syntax_set.find_syntax_by_name(name)
    }
}

// Global syntax highlighter (initialized on first use)
use std::sync::OnceLock;
static SYNTAX_HIGHLIGHTER: OnceLock<SyntaxHighlighter> = OnceLock::new();

fn get_syntax_highlighter() -> &'static SyntaxHighlighter {
    SYNTAX_HIGHLIGHTER.get_or_init(|| SyntaxHighlighter::new())
}

/// Output formatter configuration
pub struct OutputFormatter {
    /// Whether to use colors and formatting
    pub use_colors: bool,
    /// Whether to use syntax highlighting
    pub use_syntax_highlighting: bool,
}

impl OutputFormatter {
    /// Create a new formatter with automatic TTY detection
    pub fn new(plain: bool) -> Self {
        let is_tty = io::stdout().is_tty();
        let no_color = std::env::var("NO_COLOR").is_ok();

        let use_colors = !plain && !no_color && is_tty;

        Self {
            use_colors,
            use_syntax_highlighting: use_colors, // Enable syntax highlighting if colors enabled
        }
    }

    /// Format and print search results to stdout
    pub fn format_results(&self, results: &[SearchResult], pattern: &str) -> Result<()> {
        if results.is_empty() {
            if self.use_colors {
                println!("{}", "No results found.".to_string());
            } else {
                println!("No results found.");
            }
            return Ok(());
        }

        // Group results by file
        let grouped = self.group_by_file(results);

        // Print each file group
        for (idx, (file_path, file_results)) in grouped.iter().enumerate() {
            self.print_file_group(file_path, file_results, pattern, idx == grouped.len() - 1)?;
        }

        Ok(())
    }

    /// Group results by file path
    fn group_by_file<'a>(&self, results: &'a [SearchResult]) -> Vec<(String, Vec<&'a SearchResult>)> {
        let mut grouped: HashMap<String, Vec<&'a SearchResult>> = HashMap::new();

        for result in results {
            grouped
                .entry(result.path.clone())
                .or_insert_with(Vec::new)
                .push(result);
        }

        // Convert to sorted vec (by file path)
        let mut grouped_vec: Vec<_> = grouped.into_iter().collect();
        grouped_vec.sort_by(|a, b| a.0.cmp(&b.0));

        grouped_vec
    }

    /// Print a group of results for a single file
    fn print_file_group(
        &self,
        file_path: &str,
        results: &[&SearchResult],
        pattern: &str,
        is_last: bool,
    ) -> Result<()> {
        // Print file header
        self.print_file_header(file_path, results.len())?;

        // Print each result
        for (idx, result) in results.iter().enumerate() {
            let is_last_result = idx == results.len() - 1;
            self.print_result(result, pattern, is_last_result)?;
        }

        // Add spacing between files (unless it's the last file)
        if !is_last {
            println!();
        }

        Ok(())
    }

    /// Print file header with match count
    fn print_file_header(&self, file_path: &str, count: usize) -> Result<()> {
        if self.use_colors {
            // Colorized header with file icon
            println!(
                "{} {} {}",
                "📁".bright_blue(),
                file_path.bright_cyan().bold(),
                format!("({} {})", count, if count == 1 { "match" } else { "matches" })
                    .dimmed()
            );
        } else {
            // Plain text header
            println!(
                "{} ({} {})",
                file_path,
                count,
                if count == 1 { "match" } else { "matches" }
            );
        }

        Ok(())
    }

    /// Print a single search result
    fn print_result(&self, result: &SearchResult, pattern: &str, is_last: bool) -> Result<()> {
        let connector = if is_last { "└─" } else { "├─" };
        let continuation = if is_last { "  " } else { "│ " };

        // Format line number (right-aligned to 4 digits)
        let line_no = format!("{:>4}", result.span.start_line);

        // Get symbol badge if available
        let symbol_badge = self.format_symbol_badge(&result.kind, result.symbol.as_deref());

        // Print the line with result
        if self.use_colors {
            println!(
                "  {} {} {} {}",
                connector.dimmed(),
                line_no.yellow(),
                "│".dimmed(),
                symbol_badge
            );

            // Print code preview with syntax highlighting
            let highlighted = self.highlight_code(&result.preview, &result.lang, pattern);
            println!("  {}   {} {}", continuation.dimmed(), "│".dimmed(), highlighted);

            // Print scope if available
            if let Some(scope) = &result.scope {
                println!(
                    "  {}   {} {}",
                    continuation.dimmed(),
                    "└─".dimmed(),
                    format!("in {}", scope).dimmed().italic()
                );
            }

            // Add blank line between results for better readability (except for last result)
            if !is_last {
                println!("  {}", continuation.dimmed());
            }
        } else{
            // Plain text output
            println!("  {} {} | {}", connector, line_no, symbol_badge);
            println!("  {}   | {}", continuation, result.preview);

            if let Some(scope) = &result.scope {
                println!("  {}   └─ in {}", continuation, scope);
            }

            // Add blank line between results for better readability (except for last result)
            if !is_last {
                println!("  {}", continuation);
            }
        }

        Ok(())
    }

    /// Format symbol kind badge
    fn format_symbol_badge(&self, kind: &SymbolKind, symbol: Option<&str>) -> String {
        let (kind_str, color_fn): (&str, fn(&str) -> String) = match kind {
            SymbolKind::Function => ("fn", |s| s.green().to_string()),
            SymbolKind::Class => ("class", |s| s.blue().to_string()),
            SymbolKind::Struct => ("struct", |s| s.cyan().to_string()),
            SymbolKind::Enum => ("enum", |s| s.magenta().to_string()),
            SymbolKind::Trait => ("trait", |s| s.yellow().to_string()),
            SymbolKind::Interface => ("interface", |s| s.blue().to_string()),
            SymbolKind::Method => ("method", |s| s.green().to_string()),
            SymbolKind::Constant => ("const", |s| s.red().to_string()),
            SymbolKind::Variable => ("var", |s| s.white().to_string()),
            SymbolKind::Module => ("mod", |s| s.bright_magenta().to_string()),
            SymbolKind::Namespace => ("namespace", |s| s.bright_magenta().to_string()),
            SymbolKind::Type => ("type", |s| s.cyan().to_string()),
            SymbolKind::Import => ("import", |s| s.bright_blue().to_string()),
            SymbolKind::Export => ("export", |s| s.bright_blue().to_string()),
            SymbolKind::Unknown(_) => ("", |s| s.white().to_string()),
        };

        if self.use_colors && !kind_str.is_empty() {
            if let Some(sym) = symbol {
                format!("{} {}", color_fn(&format!("[{}]", kind_str)), sym.bold())
            } else {
                color_fn(&format!("[{}]", kind_str))
            }
        } else if !kind_str.is_empty() {
            if let Some(sym) = symbol {
                format!("[{}] {}", kind_str, sym)
            } else {
                format!("[{}]", kind_str)
            }
        } else {
            symbol.unwrap_or("").to_string()
        }
    }

    /// Highlight code with syntax highlighting
    fn highlight_code(&self, code: &str, lang: &Language, pattern: &str) -> String {
        if !self.use_syntax_highlighting {
            return code.to_string();
        }

        let highlighter = get_syntax_highlighter();

        // Try to get syntax for the language
        let syntax = match highlighter.get_syntax(lang) {
            Some(s) => s,
            None => {
                // Fallback: highlight the pattern match manually
                return self.highlight_pattern(code, pattern);
            }
        };

        // Get theme - try Monokai Extended, fall back to base16-ocean.dark or first available
        let theme = highlighter.theme_set.themes.get("Monokai Extended")
            .or_else(|| highlighter.theme_set.themes.get("base16-ocean.dark"))
            .or_else(|| highlighter.theme_set.themes.values().next())
            .expect("No themes available in syntect");

        let mut output = String::new();
        let mut h = HighlightLines::new(syntax, theme);

        for line in LinesWithEndings::from(code) {
            let ranges: Vec<(SyntectStyle, &str)> = h.highlight_line(line, &highlighter.syntax_set).unwrap_or_default();
            let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
            output.push_str(&escaped);
        }

        output
    }

    /// Fallback: manually highlight pattern matches in code
    fn highlight_pattern(&self, code: &str, pattern: &str) -> String {
        if pattern.is_empty() || !self.use_colors {
            return code.to_string();
        }

        // Simple substring highlighting (case-sensitive)
        if let Some(pos) = code.find(pattern) {
            let before = &code[..pos];
            let matched = &code[pos..pos + pattern.len()];
            let after = &code[pos + pattern.len()..];

            format!(
                "{}{}{}",
                before,
                matched.black().on_yellow().bold(),
                after
            )
        } else {
            code.to_string()
        }
    }
}

// Import color trait extensions
use owo_colors::OwoColorize;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Span;

    #[test]
    fn test_formatter_creation() {
        let formatter = OutputFormatter::new(false);
        // In tests, stdout is not a TTY, so colors should be disabled
        assert!(!formatter.use_colors);
    }

    #[test]
    fn test_plain_mode() {
        let formatter = OutputFormatter::new(true);
        assert!(!formatter.use_colors);
        assert!(!formatter.use_syntax_highlighting);
    }

    #[test]
    fn test_group_by_file() {
        let formatter = OutputFormatter::new(true);

        let results = vec![
            SearchResult {
                path: "a.rs".to_string(),
                lang: Language::Rust,
                kind: SymbolKind::Function,
                symbol: Some("foo".to_string()),
                span: Span {
                    start_line: 1,
                    end_line: 1,
                    start_col: 0,
                    end_col: 0,
                },
                scope: None,
                preview: "fn foo() {}".to_string(),
            },
            SearchResult {
                path: "a.rs".to_string(),
                lang: Language::Rust,
                kind: SymbolKind::Function,
                symbol: Some("bar".to_string()),
                span: Span {
                    start_line: 2,
                    end_line: 2,
                    start_col: 0,
                    end_col: 0,
                },
                scope: None,
                preview: "fn bar() {}".to_string(),
            },
            SearchResult {
                path: "b.rs".to_string(),
                lang: Language::Rust,
                kind: SymbolKind::Function,
                symbol: Some("baz".to_string()),
                span: Span {
                    start_line: 1,
                    end_line: 1,
                    start_col: 0,
                    end_col: 0,
                },
                scope: None,
                preview: "fn baz() {}".to_string(),
            },
        ];

        let grouped = formatter.group_by_file(&results);

        assert_eq!(grouped.len(), 2);
        assert_eq!(grouped[0].0, "a.rs");
        assert_eq!(grouped[0].1.len(), 2);
        assert_eq!(grouped[1].0, "b.rs");
        assert_eq!(grouped[1].1.len(), 1);
    }

    #[test]
    fn test_symbol_badge_formatting() {
        let formatter = OutputFormatter::new(true);

        let badge = formatter.format_symbol_badge(&SymbolKind::Function, Some("test"));
        assert_eq!(badge, "[fn] test");

        let badge = formatter.format_symbol_badge(&SymbolKind::Class, None);
        assert_eq!(badge, "[class]");
    }
}
