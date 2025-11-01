//! Cache management and memory-mapped I/O
//!
//! The cache module handles the `.reflex/` directory structure:
//! - `meta.db`: Metadata and configuration
//! - `symbols.bin`: Serialized symbol table
//! - `tokens.bin`: Compressed lexical tokens
//! - `hashes.json`: blake3 file hashes for incremental updates
//! - `config.toml`: Index settings

pub mod symbol_writer;
pub mod symbol_reader;

use anyhow::{Context, Result};
use rusqlite::Connection;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::models::IndexedFile;

pub use symbol_writer::SymbolWriter;
pub use symbol_reader::SymbolReader;

/// Default cache directory name
pub const CACHE_DIR: &str = ".reflex";

/// File names within the cache directory
pub const META_DB: &str = "meta.db";
pub const SYMBOLS_BIN: &str = "symbols.bin";
pub const TOKENS_BIN: &str = "tokens.bin";
pub const HASHES_JSON: &str = "hashes.json";
pub const CONFIG_TOML: &str = "config.toml";

/// Manages the RefLex cache directory
pub struct CacheManager {
    cache_path: PathBuf,
}

impl CacheManager {
    /// Create a new cache manager for the given root directory
    pub fn new(root: impl AsRef<Path>) -> Self {
        let cache_path = root.as_ref().join(CACHE_DIR);
        Self { cache_path }
    }

    /// Initialize the cache directory structure if it doesn't exist
    pub fn init(&self) -> Result<()> {
        log::info!("Initializing cache at {:?}", self.cache_path);

        if !self.cache_path.exists() {
            std::fs::create_dir_all(&self.cache_path)?;
        }

        // Create meta.db with schema
        self.init_meta_db()?;

        // Create empty symbols.bin with header
        self.init_symbols_bin()?;

        // Create empty tokens.bin with header
        self.init_tokens_bin()?;

        // Create hashes.json with empty map
        self.init_hashes_json()?;

        // Create default config.toml
        self.init_config_toml()?;

        log::info!("Cache initialized successfully");
        Ok(())
    }

    /// Initialize meta.db with SQLite schema
    fn init_meta_db(&self) -> Result<()> {
        let db_path = self.cache_path.join(META_DB);

        // Skip if already exists
        if db_path.exists() {
            return Ok(());
        }

        let conn = Connection::open(&db_path)
            .context("Failed to create meta.db")?;

        // Create files table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS files (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                path TEXT NOT NULL UNIQUE,
                hash TEXT NOT NULL,
                last_indexed INTEGER NOT NULL,
                language TEXT NOT NULL,
                symbol_count INTEGER DEFAULT 0,
                token_count INTEGER DEFAULT 0
            )",
            [],
        )?;

        conn.execute("CREATE INDEX IF NOT EXISTS idx_files_path ON files(path)", [])?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_files_hash ON files(hash)", [])?;

        // Create statistics table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS statistics (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at INTEGER NOT NULL
            )",
            [],
        )?;

        // Initialize default statistics
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "INSERT OR REPLACE INTO statistics (key, value, updated_at) VALUES (?, ?, ?)",
            ["total_files", "0", &now.to_string()],
        )?;
        conn.execute(
            "INSERT OR REPLACE INTO statistics (key, value, updated_at) VALUES (?, ?, ?)",
            ["total_symbols", "0", &now.to_string()],
        )?;
        conn.execute(
            "INSERT OR REPLACE INTO statistics (key, value, updated_at) VALUES (?, ?, ?)",
            ["cache_version", "1", &now.to_string()],
        )?;

        // Create config table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS config (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )?;

        log::debug!("Created meta.db with schema");
        Ok(())
    }

    /// Initialize symbols.bin with header
    fn init_symbols_bin(&self) -> Result<()> {
        let symbols_path = self.cache_path.join(SYMBOLS_BIN);

        if symbols_path.exists() {
            return Ok(());
        }

        let mut file = File::create(&symbols_path)?;

        // Write header: magic bytes + version + symbol count + index offset
        let magic_bytes = b"RFLX"; // RefLex magic
        let version: u32 = 1;
        let symbol_count: u64 = 0;
        let index_offset: u64 = 0;
        let reserved = [0u8; 8];

        file.write_all(magic_bytes)?;
        file.write_all(&version.to_le_bytes())?;
        file.write_all(&symbol_count.to_le_bytes())?;
        file.write_all(&index_offset.to_le_bytes())?;
        file.write_all(&reserved)?;

        log::debug!("Created empty symbols.bin");
        Ok(())
    }

    /// Initialize tokens.bin with header
    fn init_tokens_bin(&self) -> Result<()> {
        let tokens_path = self.cache_path.join(TOKENS_BIN);

        if tokens_path.exists() {
            return Ok(());
        }

        let mut file = File::create(&tokens_path)?;

        // Write header: magic bytes + version + compression type + sizes
        let magic_bytes = b"RFTK"; // RefLex Tokens
        let version: u32 = 1;
        let compression_type: u32 = 1; // 1 = zstd
        let uncompressed_size: u64 = 0;
        let token_count: u64 = 0;
        let reserved = [0u8; 8];

        file.write_all(magic_bytes)?;
        file.write_all(&version.to_le_bytes())?;
        file.write_all(&compression_type.to_le_bytes())?;
        file.write_all(&uncompressed_size.to_le_bytes())?;
        file.write_all(&token_count.to_le_bytes())?;
        file.write_all(&reserved)?;

        log::debug!("Created empty tokens.bin");
        Ok(())
    }

    /// Initialize hashes.json with empty map
    fn init_hashes_json(&self) -> Result<()> {
        let hashes_path = self.cache_path.join(HASHES_JSON);

        if hashes_path.exists() {
            return Ok(());
        }

        let empty_map: HashMap<String, String> = HashMap::new();
        let json = serde_json::to_string_pretty(&empty_map)?;
        std::fs::write(&hashes_path, json)?;

        log::debug!("Created empty hashes.json");
        Ok(())
    }

    /// Initialize config.toml with defaults
    fn init_config_toml(&self) -> Result<()> {
        let config_path = self.cache_path.join(CONFIG_TOML);

        if config_path.exists() {
            return Ok(());
        }

        let default_config = r#"[index]
languages = []  # Empty = all supported languages
max_file_size = 10485760  # 10 MB
follow_symlinks = false

[index.include]
patterns = []

[index.exclude]
patterns = []

[search]
default_limit = 100
fuzzy_threshold = 0.8

[performance]
parallel_threads = 0  # 0 = auto-detect
compression_level = 3  # zstd level
"#;

        std::fs::write(&config_path, default_config)?;

        log::debug!("Created default config.toml");
        Ok(())
    }

    /// Check if cache exists and is valid
    pub fn exists(&self) -> bool {
        self.cache_path.exists()
            && self.cache_path.join(META_DB).exists()
            && self.cache_path.join(SYMBOLS_BIN).exists()
    }

    /// Get the path to the cache directory
    pub fn path(&self) -> &Path {
        &self.cache_path
    }

    /// Clear the entire cache
    pub fn clear(&self) -> Result<()> {
        log::warn!("Clearing cache at {:?}", self.cache_path);

        if self.cache_path.exists() {
            std::fs::remove_dir_all(&self.cache_path)?;
        }

        Ok(())
    }

    /// Load file hashes for incremental indexing
    pub fn load_hashes(&self) -> Result<HashMap<String, String>> {
        let hash_path = self.cache_path.join(HASHES_JSON);

        if !hash_path.exists() {
            return Ok(HashMap::new());
        }

        let contents = std::fs::read_to_string(&hash_path)
            .context("Failed to read hashes.json")?;

        let hashes: HashMap<String, String> = serde_json::from_str(&contents)
            .context("Failed to parse hashes.json")?;

        log::debug!("Loaded {} file hashes", hashes.len());
        Ok(hashes)
    }

    /// Save file hashes for incremental indexing
    pub fn save_hashes(&self, hashes: &HashMap<String, String>) -> Result<()> {
        let hash_path = self.cache_path.join(HASHES_JSON);

        let json = serde_json::to_string_pretty(hashes)
            .context("Failed to serialize hashes")?;

        std::fs::write(&hash_path, json)
            .context("Failed to write hashes.json")?;

        log::debug!("Saved {} file hashes", hashes.len());
        Ok(())
    }

    /// Update file metadata in the files table
    pub fn update_file(&self, path: &str, hash: &str, language: &str, symbol_count: usize) -> Result<()> {
        let db_path = self.cache_path.join(META_DB);
        let conn = Connection::open(&db_path)
            .context("Failed to open meta.db for file update")?;

        let now = chrono::Utc::now().timestamp();

        conn.execute(
            "INSERT OR REPLACE INTO files (path, hash, last_indexed, language, symbol_count)
             VALUES (?, ?, ?, ?, ?)",
            [path, hash, &now.to_string(), language, &symbol_count.to_string()],
        )?;

        Ok(())
    }

    /// Update statistics after indexing by calculating totals from database
    pub fn update_stats(&self) -> Result<()> {
        let db_path = self.cache_path.join(META_DB);
        let conn = Connection::open(&db_path)
            .context("Failed to open meta.db for stats update")?;

        // Count total files from files table
        let total_files: usize = conn.query_row(
            "SELECT COUNT(*) FROM files",
            [],
            |row| row.get(0),
        ).unwrap_or(0);

        // Sum total symbols from files table
        let total_symbols: usize = conn.query_row(
            "SELECT SUM(symbol_count) FROM files",
            [],
            |row| row.get(0),
        ).unwrap_or(0);

        let now = chrono::Utc::now().timestamp();

        conn.execute(
            "INSERT OR REPLACE INTO statistics (key, value, updated_at) VALUES (?, ?, ?)",
            ["total_files", &total_files.to_string(), &now.to_string()],
        )?;

        conn.execute(
            "INSERT OR REPLACE INTO statistics (key, value, updated_at) VALUES (?, ?, ?)",
            ["total_symbols", &total_symbols.to_string(), &now.to_string()],
        )?;

        log::debug!("Updated statistics: {} files, {} symbols", total_files, total_symbols);
        Ok(())
    }

    /// Get list of all indexed files
    pub fn list_files(&self) -> Result<Vec<IndexedFile>> {
        let db_path = self.cache_path.join(META_DB);

        if !db_path.exists() {
            return Ok(Vec::new());
        }

        let conn = Connection::open(&db_path)
            .context("Failed to open meta.db")?;

        let mut stmt = conn.prepare(
            "SELECT path, language, symbol_count, last_indexed FROM files ORDER BY path"
        )?;

        let files = stmt.query_map([], |row| {
            let path: String = row.get(0)?;
            let language: String = row.get(1)?;
            let symbol_count: i64 = row.get(2)?;
            let last_indexed: i64 = row.get(3)?;

            Ok(IndexedFile {
                path,
                language,
                symbol_count: symbol_count as usize,
                last_indexed: chrono::DateTime::from_timestamp(last_indexed, 0)
                    .unwrap_or_else(chrono::Utc::now)
                    .to_rfc3339(),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

        Ok(files)
    }

    /// Get statistics about the current cache
    pub fn stats(&self) -> Result<crate::models::IndexStats> {
        let db_path = self.cache_path.join(META_DB);

        if !db_path.exists() {
            // Cache not initialized
            return Ok(crate::models::IndexStats {
                total_files: 0,
                total_symbols: 0,
                index_size_bytes: 0,
                last_updated: chrono::Utc::now().to_rfc3339(),
            });
        }

        let conn = Connection::open(&db_path)
            .context("Failed to open meta.db")?;

        // Read total files
        let total_files: usize = conn.query_row(
            "SELECT value FROM statistics WHERE key = 'total_files'",
            [],
            |row| {
                let value: String = row.get(0)?;
                Ok(value.parse().unwrap_or(0))
            },
        ).unwrap_or(0);

        // Read total symbols
        let total_symbols: usize = conn.query_row(
            "SELECT value FROM statistics WHERE key = 'total_symbols'",
            [],
            |row| {
                let value: String = row.get(0)?;
                Ok(value.parse().unwrap_or(0))
            },
        ).unwrap_or(0);

        // Read last updated timestamp
        let last_updated: String = conn.query_row(
            "SELECT updated_at FROM statistics WHERE key = 'total_files'",
            [],
            |row| {
                let timestamp: i64 = row.get(0)?;
                Ok(chrono::DateTime::from_timestamp(timestamp, 0)
                    .unwrap_or_else(chrono::Utc::now)
                    .to_rfc3339())
            },
        ).unwrap_or_else(|_| chrono::Utc::now().to_rfc3339());

        // Calculate total cache size
        let mut index_size_bytes: u64 = 0;

        for file_name in [META_DB, SYMBOLS_BIN, TOKENS_BIN, HASHES_JSON, CONFIG_TOML] {
            let file_path = self.cache_path.join(file_name);
            if let Ok(metadata) = std::fs::metadata(&file_path) {
                index_size_bytes += metadata.len();
            }
        }

        Ok(crate::models::IndexStats {
            total_files,
            total_symbols,
            index_size_bytes,
            last_updated,
        })
    }
}

// TODO: Implement memory-mapped readers for:
// - SymbolReader (reads from symbols.bin)
// - TokenReader (reads from tokens.bin)
// - MetaReader (reads from meta.db)

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_cache_init() {
        let temp = TempDir::new().unwrap();
        let cache = CacheManager::new(temp.path());

        assert!(!cache.exists());
        cache.init().unwrap();
        assert!(cache.path().exists());
    }
}
