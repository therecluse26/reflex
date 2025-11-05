use std::collections::HashMap;

pub struct Parser {
    tokens: Vec<String>,
}

impl Parser {
    pub fn new() -> Self {
        // TODO: Initialize with config
        Parser { tokens: vec![] }
    }

    pub fn extract_pattern(&self, pattern: &str) -> Option<String> {
        self.tokens.iter().find(|t| t.contains(pattern)).cloned()
    }
}
