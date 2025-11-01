//! Tree-sitter parsers for extracting symbols from source code
//!
//! This module provides language-specific parsers that extract symbols
//! (functions, classes, structs, etc.) from source code using Tree-sitter.
//!
//! Each language has its own submodule with a `parse` function that takes
//! source code and returns a vector of symbols.

pub mod rust;
pub mod typescript;
pub mod vue;
pub mod svelte;
pub mod php;

use anyhow::Result;
use crate::models::{Language, SearchResult};

/// Parser factory that selects the appropriate parser based on language
pub struct ParserFactory;

impl ParserFactory {
    /// Parse a file and extract symbols based on its language
    pub fn parse(
        path: &str,
        source: &str,
        language: Language,
    ) -> Result<Vec<SearchResult>> {
        match language {
            Language::Rust => rust::parse(path, source),
            Language::TypeScript => typescript::parse(path, source, language),
            Language::JavaScript => typescript::parse(path, source, language),
            Language::Vue => vue::parse(path, source),
            Language::Svelte => svelte::parse(path, source),
            Language::Python => {
                // TODO: Implement Python parser
                Ok(vec![])
            }
            Language::Go => {
                // TODO: Implement Go parser
                Ok(vec![])
            }
            Language::Java => {
                // TODO: Implement Java parser
                Ok(vec![])
            }
            Language::PHP => php::parse(path, source),
            Language::C => {
                // TODO: Implement C parser
                Ok(vec![])
            }
            Language::Cpp => {
                // TODO: Implement C++ parser
                Ok(vec![])
            }
            Language::Unknown => {
                log::warn!("Unknown language for file: {}", path);
                Ok(vec![])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_factory() {
        // Simple test to ensure module compiles
        let _factory = ParserFactory;
    }
}
