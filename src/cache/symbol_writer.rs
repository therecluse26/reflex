//! Symbol writer for writing symbols to symbols.bin with rkyv serialization

use anyhow::{Context, Result};
use rkyv::{Archive, Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Seek, Write};
use std::path::Path;

use crate::models::SearchResult;

/// Serializable symbol entry for rkyv
#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
pub struct SymbolEntry {
    pub kind: String,  // SymbolKind as string for simplicity
    pub name: String,
    pub path: String,
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
    pub scope: Option<String>,
    pub preview: String,
}

impl From<&SearchResult> for SymbolEntry {
    fn from(result: &SearchResult) -> Self {
        Self {
            kind: format!("{:?}", result.kind),
            name: result.symbol.clone(),
            path: result.path.clone(),
            start_line: result.span.start_line,
            start_col: result.span.start_col,
            end_line: result.span.end_line,
            end_col: result.span.end_col,
            scope: result.scope.clone(),
            preview: result.preview.clone(),
        }
    }
}

/// Writes symbols to symbols.bin using rkyv serialization
pub struct SymbolWriter {
    symbols: Vec<SymbolEntry>,
    symbol_index: HashMap<String, Vec<usize>>, // symbol name → indices in symbols vec
}

impl SymbolWriter {
    pub fn new() -> Self {
        Self {
            symbols: Vec::new(),
            symbol_index: HashMap::new(),
        }
    }

    /// Add a symbol to the writer
    pub fn add(&mut self, symbol: &SearchResult) {
        let entry = SymbolEntry::from(symbol);
        let idx = self.symbols.len();

        // Update index
        self.symbol_index
            .entry(entry.name.clone())
            .or_insert_with(Vec::new)
            .push(idx);

        self.symbols.push(entry);
    }

    /// Add multiple symbols
    pub fn add_all(&mut self, symbols: &[SearchResult]) {
        for symbol in symbols {
            self.add(symbol);
        }
    }

    /// Write symbols to file
    pub fn write(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();

        // Serialize symbols with rkyv
        let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&self.symbols)
            .context("Failed to serialize symbols with rkyv")?;

        // Open file for writing
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .context("Failed to open symbols.bin for writing")?;

        // Write header
        self.write_header(&mut file, bytes.len())?;

        // Write serialized data
        file.write_all(&bytes)
            .context("Failed to write symbol data")?;

        // Write index
        let index_offset = 32 + bytes.len(); // header (32) + data
        self.write_index(&mut file)?;

        // Update header with index offset
        file.seek(std::io::SeekFrom::Start(16))?;
        file.write_all(&(index_offset as u64).to_le_bytes())?;

        file.flush()?;

        log::debug!("Wrote {} symbols to {:?}", self.symbols.len(), path);

        Ok(())
    }

    /// Write file header
    fn write_header(&self, file: &mut File, _data_size: usize) -> Result<()> {
        // Magic bytes: "RFLX"
        file.write_all(b"RFLX")?;

        // Version: u32
        file.write_all(&1u32.to_le_bytes())?;

        // Symbol count: u64
        file.write_all(&(self.symbols.len() as u64).to_le_bytes())?;

        // Index offset: u64 (will be updated later)
        file.write_all(&0u64.to_le_bytes())?;

        // Reserved: 8 bytes
        file.write_all(&[0u8; 8])?;

        Ok(())
    }

    /// Write symbol index (simple JSON for now)
    fn write_index(&self, file: &mut File) -> Result<()> {
        let index_json = serde_json::to_vec(&self.symbol_index)
            .context("Failed to serialize symbol index")?;

        file.write_all(&index_json)?;

        Ok(())
    }

    /// Get number of symbols
    pub fn len(&self) -> usize {
        self.symbols.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Language, Span, SymbolKind};
    use tempfile::TempDir;

    #[test]
    fn test_symbol_writer() {
        let temp = TempDir::new().unwrap();
        let symbols_path = temp.path().join("symbols.bin");

        let mut writer = SymbolWriter::new();

        // Add test symbol
        let symbol = SearchResult {
            path: "test.rs".to_string(),
            lang: Language::Rust,
            kind: SymbolKind::Function,
            symbol: "test_func".to_string(),
            span: Span::new(1, 0, 5, 1),
            scope: None,
            preview: "fn test_func() {}".to_string(),
        };

        writer.add(&symbol);

        assert_eq!(writer.len(), 1);

        // Write to file
        writer.write(&symbols_path).unwrap();

        assert!(symbols_path.exists());

        // Check file size
        let metadata = std::fs::metadata(&symbols_path).unwrap();
        assert!(metadata.len() > 32); // At least header size
    }
}
