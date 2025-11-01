//! Symbol reader for reading symbols from symbols.bin with rkyv deserialization

use anyhow::{Context, Result};
use memmap2::Mmap;
use rkyv::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

use crate::cache::symbol_writer::SymbolEntry;
use crate::models::{Language, SearchResult, Span, SymbolKind};

/// Reads symbols from symbols.bin using memory-mapped I/O and rkyv
pub struct SymbolReader {
    _file: File,
    mmap: Mmap,
    symbol_index: HashMap<String, Vec<usize>>,
}

impl SymbolReader {
    /// Open and memory-map symbols.bin for reading
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();

        // Open file for reading
        let file = File::open(path)
            .with_context(|| format!("Failed to open symbols file: {}", path.display()))?;

        // Memory-map the file
        let mmap = unsafe {
            Mmap::map(&file)
                .context("Failed to memory-map symbols file")?
        };

        // Read and validate header
        if mmap.len() < 32 {
            anyhow::bail!("symbols.bin is too small (expected at least 32 bytes for header)");
        }

        // Check magic bytes
        if &mmap[0..4] != b"RFLX" {
            anyhow::bail!("Invalid symbols.bin file (wrong magic bytes)");
        }

        // Read version
        let version = u32::from_le_bytes([mmap[4], mmap[5], mmap[6], mmap[7]]);
        if version != 1 {
            anyhow::bail!("Unsupported symbols.bin version: {}", version);
        }

        // Read symbol count
        let symbol_count = u64::from_le_bytes([
            mmap[8], mmap[9], mmap[10], mmap[11],
            mmap[12], mmap[13], mmap[14], mmap[15],
        ]);

        // Read index offset
        let index_offset = u64::from_le_bytes([
            mmap[16], mmap[17], mmap[18], mmap[19],
            mmap[20], mmap[21], mmap[22], mmap[23],
        ]) as usize;

        log::debug!(
            "Opened symbols.bin: version={}, symbols={}, index_offset={}",
            version,
            symbol_count,
            index_offset
        );

        // Read index from the end of the file
        let symbol_index = if index_offset > 0 && index_offset < mmap.len() {
            let index_bytes = &mmap[index_offset..];
            serde_json::from_slice(index_bytes)
                .context("Failed to deserialize symbol index")?
        } else {
            HashMap::new()
        };

        log::debug!("Loaded symbol index with {} entries", symbol_index.len());

        Ok(Self {
            _file: file,
            mmap,
            symbol_index,
        })
    }

    /// Read all symbols from the file
    pub fn read_all(&self) -> Result<Vec<SearchResult>> {
        // Symbol data starts after the 32-byte header
        let data_start = 32;

        // Find where index starts (or end of file if no index)
        let index_offset = u64::from_le_bytes([
            self.mmap[16], self.mmap[17], self.mmap[18], self.mmap[19],
            self.mmap[20], self.mmap[21], self.mmap[22], self.mmap[23],
        ]) as usize;

        let data_end = if index_offset > 0 {
            index_offset
        } else {
            self.mmap.len()
        };

        if data_start >= data_end {
            // No symbol data
            return Ok(Vec::new());
        }

        // Deserialize rkyv data
        let data_bytes = &self.mmap[data_start..data_end];

        // Deserialize the archived vector
        let symbols: Vec<SymbolEntry> = rkyv::from_bytes::<_, rkyv::rancor::Error>(data_bytes)
            .context("Failed to deserialize symbols from rkyv")?;

        // Convert entries to SearchResult
        let mut results = Vec::new();
        for entry in symbols {
            results.push(self.entry_to_result(&entry)?);
        }

        Ok(results)
    }

    /// Find symbols by name (exact match)
    pub fn find_by_name(&self, name: &str) -> Result<Vec<SearchResult>> {
        // Check index for symbols with this name
        if let Some(indices) = self.symbol_index.get(name) {
            let all_symbols = self.read_all()?;
            let mut results = Vec::new();

            for &idx in indices {
                if idx < all_symbols.len() {
                    results.push(all_symbols[idx].clone());
                }
            }

            Ok(results)
        } else {
            Ok(Vec::new())
        }
    }

    /// Find symbols by name prefix
    pub fn find_by_prefix(&self, prefix: &str) -> Result<Vec<SearchResult>> {
        let all_symbols = self.read_all()?;

        Ok(all_symbols
            .into_iter()
            .filter(|s| s.symbol.starts_with(prefix))
            .collect())
    }

    /// Find symbols containing substring in either symbol name OR preview content
    pub fn find_by_substring(&self, substring: &str) -> Result<Vec<SearchResult>> {
        let all_symbols = self.read_all()?;

        Ok(all_symbols
            .into_iter()
            .filter(|s| s.symbol.contains(substring) || s.preview.contains(substring))
            .collect())
    }

    /// Find symbols containing substring in symbol name only (filtered search)
    pub fn find_by_symbol_name_only(&self, substring: &str) -> Result<Vec<SearchResult>> {
        let all_symbols = self.read_all()?;

        Ok(all_symbols
            .into_iter()
            .filter(|s| s.symbol.contains(substring))
            .collect())
    }

    /// Find symbols containing substring in preview content only (filtered search)
    pub fn find_by_preview_only(&self, substring: &str) -> Result<Vec<SearchResult>> {
        let all_symbols = self.read_all()?;

        Ok(all_symbols
            .into_iter()
            .filter(|s| s.preview.contains(substring))
            .collect())
    }

    /// Convert SymbolEntry to SearchResult
    fn entry_to_result(&self, entry: &SymbolEntry) -> Result<SearchResult> {
        // Parse the kind string back to SymbolKind
        let kind = match entry.kind.as_str() {
            "Function" => SymbolKind::Function,
            "Struct" => SymbolKind::Struct,
            "Enum" => SymbolKind::Enum,
            "Trait" => SymbolKind::Trait,
            "Method" => SymbolKind::Method,
            "Constant" => SymbolKind::Constant,
            "Module" => SymbolKind::Module,
            "Type" => SymbolKind::Type,
            _ => {
                log::warn!("Unknown symbol kind: {}", entry.kind);
                SymbolKind::Function // Default fallback
            }
        };

        // Detect language from file extension
        let lang = if let Some(ext) = Path::new(&entry.path).extension() {
            Language::from_extension(ext.to_str().unwrap_or(""))
        } else {
            Language::Unknown
        };

        Ok(SearchResult {
            path: entry.path.clone(),
            lang,
            kind,
            symbol: entry.name.clone(),
            span: Span::new(
                entry.start_line,
                entry.start_col,
                entry.end_line,
                entry.end_col,
            ),
            scope: entry.scope.clone(),
            preview: entry.preview.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::symbol_writer::SymbolWriter;
    use crate::models::SymbolKind;
    use tempfile::TempDir;

    #[test]
    fn test_symbol_reader() {
        let temp = TempDir::new().unwrap();
        let symbols_path = temp.path().join("symbols.bin");

        // Write some test symbols
        let mut writer = SymbolWriter::new();

        let symbol1 = SearchResult {
            path: "test.rs".to_string(),
            lang: Language::Rust,
            kind: SymbolKind::Function,
            symbol: "test_func".to_string(),
            span: Span::new(1, 0, 5, 1),
            scope: None,
            preview: "fn test_func() {}".to_string(),
        };

        let symbol2 = SearchResult {
            path: "test.rs".to_string(),
            lang: Language::Rust,
            kind: SymbolKind::Struct,
            symbol: "TestStruct".to_string(),
            span: Span::new(10, 0, 15, 1),
            scope: None,
            preview: "struct TestStruct {}".to_string(),
        };

        writer.add(&symbol1);
        writer.add(&symbol2);
        writer.write(&symbols_path).unwrap();

        // Read symbols back
        let reader = SymbolReader::open(&symbols_path).unwrap();
        let symbols = reader.read_all().unwrap();

        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].symbol, "test_func");
        assert_eq!(symbols[1].symbol, "TestStruct");
    }

    #[test]
    fn test_find_by_name() {
        let temp = TempDir::new().unwrap();
        let symbols_path = temp.path().join("symbols.bin");

        let mut writer = SymbolWriter::new();

        let symbol = SearchResult {
            path: "test.rs".to_string(),
            lang: Language::Rust,
            kind: SymbolKind::Function,
            symbol: "my_function".to_string(),
            span: Span::new(1, 0, 5, 1),
            scope: None,
            preview: "fn my_function() {}".to_string(),
        };

        writer.add(&symbol);
        writer.write(&symbols_path).unwrap();

        let reader = SymbolReader::open(&symbols_path).unwrap();
        let results = reader.find_by_name("my_function").unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].symbol, "my_function");
    }
}
