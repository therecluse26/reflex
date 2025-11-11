//! Syntax highlighting utilities for interactive mode
//!
//! This module provides syntax highlighting for code previews in the TUI,
//! converting syntect highlighting to ratatui Spans.

use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};
use std::sync::OnceLock;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Color as SyntectColor, Style as SyntectStyle, Theme, ThemeSet};
use syntect::parsing::{SyntaxReference, SyntaxSet};

use crate::models::Language;

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
    fn get_syntax(&self, lang: &Language) -> Option<&SyntaxReference> {
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
static SYNTAX_HIGHLIGHTER: OnceLock<SyntaxHighlighter> = OnceLock::new();

fn get_syntax_highlighter() -> &'static SyntaxHighlighter {
    SYNTAX_HIGHLIGHTER.get_or_init(SyntaxHighlighter::new)
}

/// Convert syntect color to ratatui color
fn syntect_color_to_ratatui(color: SyntectColor) -> Color {
    Color::Rgb(color.r, color.g, color.b)
}

/// Highlight multiple lines of code and return a vector of Lines with colored Spans
///
/// This function maintains syntect state across lines to properly handle multi-line
/// constructs like strings, comments, and code blocks.
///
/// If the language is not supported or highlighting fails, returns plain text lines.
pub fn highlight_code_lines<'a>(
    lines: &[String],
    lang: Language,
    theme: &Theme,
) -> Vec<Line<'a>> {
    // Early return for unknown languages
    if !lang.is_supported() {
        return lines.iter().map(|line| Line::from(line.to_string())).collect();
    }

    let highlighter = get_syntax_highlighter();

    // Get syntax for the language
    let syntax = match highlighter.get_syntax(&lang) {
        Some(s) => s,
        None => return lines.iter().map(|line| Line::from(line.to_string())).collect(),
    };

    // Create highlighter instance that maintains state across lines
    let mut h = HighlightLines::new(syntax, theme);
    let mut result = Vec::with_capacity(lines.len());

    for line in lines {
        // Highlight the line (state is maintained in h)
        let ranges: Vec<(SyntectStyle, &str)> = match h.highlight_line(line, &highlighter.syntax_set) {
            Ok(ranges) => ranges,
            Err(_) => {
                // On error, fall back to plain text for this line
                result.push(Line::from(line.to_string()));
                continue;
            }
        };

        // Convert syntect ranges to ratatui Spans
        let spans: Vec<Span<'a>> = ranges
            .into_iter()
            .map(|(style, text)| {
                let fg = syntect_color_to_ratatui(style.foreground);
                let ratatui_style = Style::default().fg(fg);
                Span::styled(text.to_string(), ratatui_style)
            })
            .collect();

        result.push(Line::from(spans));
    }

    result
}


/// Get the default theme based on terminal background
///
/// Returns Monokai Extended for dark terminals, InspiredGitHub for light terminals.
pub fn get_default_theme(is_dark: bool) -> Theme {
    let highlighter = get_syntax_highlighter();

    let theme_name = if is_dark {
        "Monokai Extended"
    } else {
        "InspiredGitHub"
    };

    highlighter
        .theme_set
        .themes
        .get(theme_name)
        .or_else(|| {
            // Fallback to base16 themes if preferred themes aren't available
            let fallback_name = if is_dark {
                "base16-ocean.dark"
            } else {
                "base16-ocean.light"
            };
            highlighter.theme_set.themes.get(fallback_name)
        })
        .or_else(|| highlighter.theme_set.themes.values().next())
        .cloned()
        .expect("No themes available in syntect")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlighter_initialization() {
        let highlighter = get_syntax_highlighter();
        assert!(highlighter.get_syntax(&Language::Rust).is_some());
        assert!(highlighter.get_syntax(&Language::Python).is_some());
        assert!(highlighter.get_syntax(&Language::Unknown).is_none());
    }

    #[test]
    fn test_highlight_lines_rust() {
        let lines = vec![
            "fn main() {".to_string(),
            "    println!(\"Hello\");".to_string(),
            "}".to_string(),
        ];
        let theme = get_default_theme(true);
        let highlighted = highlight_code_lines(&lines, Language::Rust, &theme);

        // Should return same number of lines
        assert_eq!(highlighted.len(), 3);

        // Each line should have spans (syntax highlighting)
        for line in &highlighted {
            assert!(!line.spans.is_empty());
        }
    }

    #[test]
    fn test_highlight_lines_unknown_language() {
        let lines = vec![
            "some code".to_string(),
            "more code".to_string(),
        ];
        let theme = get_default_theme(true);
        let highlighted = highlight_code_lines(&lines, Language::Unknown, &theme);

        // Should return plain text for unknown languages
        assert_eq!(highlighted.len(), 2);
        for line in &highlighted {
            assert_eq!(line.spans.len(), 1);
        }
    }

    #[test]
    fn test_highlight_lines_maintains_state() {
        // Test that multi-line constructs like strings work properly
        let lines = vec![
            "<?php".to_string(),
            "use Some\\Namespace;".to_string(),
            "use Another\\Namespace;".to_string(),
        ];
        let theme = get_default_theme(true);
        let highlighted = highlight_code_lines(&lines, Language::PHP, &theme);

        // Should return same number of lines
        assert_eq!(highlighted.len(), 3);

        // All lines should have syntax highlighting
        for line in &highlighted {
            assert!(!line.spans.is_empty());
        }
    }

    #[test]
    fn test_color_conversion() {
        let syntect_color = SyntectColor { r: 255, g: 128, b: 64, a: 255 };
        let ratatui_color = syntect_color_to_ratatui(syntect_color);

        match ratatui_color {
            Color::Rgb(r, g, b) => {
                assert_eq!(r, 255);
                assert_eq!(g, 128);
                assert_eq!(b, 64);
            }
            _ => panic!("Expected RGB color"),
        }
    }

    #[test]
    fn test_get_default_theme() {
        let dark_theme = get_default_theme(true);
        let light_theme = get_default_theme(false);

        // Themes should be different
        assert_ne!(dark_theme.name.as_ref(), light_theme.name.as_ref());
    }
}
