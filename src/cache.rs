//! Cache management and memory-mapped I/O
//!
//! The cache module handles the `.reflex/` directory structure:
//! - `meta.db`: Metadata, file hashes, and configuration (SQLite)
//! - `tokens.bin`: Compressed lexical tokens (binary)
//! - `content.bin`: Memory-mapped file contents (binary)
//! - `trigrams.bin`: Trigram inverted index (bincode binary)
//! - `config.toml`: Index settings (TOML text)

use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::models::IndexedFile;

/// Default cache directory name
pub const CACHE_DIR: &str = ".reflex";

/// File names within the cache directory
pub const META_DB: &str = "meta.db";
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

        // Create empty tokens.bin with header
        self.init_tokens_bin()?;

        // Create default config.toml
        self.init_config_toml()?;

        // Note: hashes.json is deprecated - hashes are now stored in meta.db

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
                token_count INTEGER DEFAULT 0,
                line_count INTEGER DEFAULT 0
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

        // Create branch tracking tables for git-aware indexing
        conn.execute(
            "CREATE TABLE IF NOT EXISTS file_branches (
                path TEXT NOT NULL,
                branch TEXT NOT NULL,
                hash TEXT NOT NULL,
                commit_sha TEXT,
                last_indexed INTEGER NOT NULL,
                PRIMARY KEY (path, branch)
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_branch_lookup ON file_branches(branch, path)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_hash_lookup ON file_branches(hash)",
            [],
        )?;

        // Create branches metadata table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS branches (
                branch TEXT PRIMARY KEY,
                commit_sha TEXT NOT NULL,
                last_indexed INTEGER NOT NULL,
                file_count INTEGER DEFAULT 0,
                is_dirty INTEGER DEFAULT 0
            )",
            [],
        )?;

        log::debug!("Created meta.db with schema");
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
    ///
    /// DEPRECATED: Hashes are now stored in SQLite (meta.db).
    /// This function is kept for backward compatibility but is not called by init().
    #[deprecated(note = "Hashes are now stored in SQLite")]
    #[allow(dead_code)]
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
parallel_threads = 0  # 0 = auto (80% of available cores), or set a specific number
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

    /// Load file hashes for incremental indexing from SQLite
    pub fn load_hashes(&self) -> Result<HashMap<String, String>> {
        let db_path = self.cache_path.join(META_DB);

        if !db_path.exists() {
            return Ok(HashMap::new());
        }

        let conn = Connection::open(&db_path)
            .context("Failed to open meta.db")?;

        let mut stmt = conn.prepare("SELECT path, hash FROM files")?;
        let hashes: HashMap<String, String> = stmt.query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?
        .collect::<Result<HashMap<_, _>, _>>()?;

        log::debug!("Loaded {} file hashes from SQLite", hashes.len());
        Ok(hashes)
    }

    /// Save file hashes for incremental indexing
    ///
    /// DEPRECATED: Hashes are now saved directly to SQLite via update_file().
    /// This method is kept for backward compatibility but does nothing.
    #[deprecated(note = "Hashes are now stored in SQLite via update_file()")]
    pub fn save_hashes(&self, _hashes: &HashMap<String, String>) -> Result<()> {
        // No-op: hashes are now persisted to SQLite in update_file()
        Ok(())
    }

    /// Update file metadata in the files table
    pub fn update_file(&self, path: &str, hash: &str, language: &str, symbol_count: usize, line_count: usize) -> Result<()> {
        let db_path = self.cache_path.join(META_DB);
        let conn = Connection::open(&db_path)
            .context("Failed to open meta.db for file update")?;

        let now = chrono::Utc::now().timestamp();

        conn.execute(
            "INSERT OR REPLACE INTO files (path, hash, last_indexed, language, symbol_count, line_count)
             VALUES (?, ?, ?, ?, ?, ?)",
            [path, hash, &now.to_string(), language, &symbol_count.to_string(), &line_count.to_string()],
        )?;

        Ok(())
    }

    /// Batch update multiple files in a single transaction for performance
    pub fn batch_update_files(&self, files: &[(String, String, String, usize, usize)]) -> Result<()> {
        let db_path = self.cache_path.join(META_DB);
        let mut conn = Connection::open(&db_path)
            .context("Failed to open meta.db for batch update")?;

        let now = chrono::Utc::now().timestamp();
        let now_str = now.to_string();

        // Use a transaction for batch inserts
        let tx = conn.transaction()?;

        for (path, hash, language, symbol_count, line_count) in files {
            tx.execute(
                "INSERT OR REPLACE INTO files (path, hash, last_indexed, language, symbol_count, line_count)
                 VALUES (?, ?, ?, ?, ?, ?)",
                [path.as_str(), hash.as_str(), &now_str, language.as_str(), &symbol_count.to_string(), &line_count.to_string()],
            )?;
        }

        tx.commit()?;
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
                files_by_language: std::collections::HashMap::new(),
                symbols_by_language: std::collections::HashMap::new(),
                lines_by_language: std::collections::HashMap::new(),
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

        // Calculate total cache size (all binary files)
        let mut index_size_bytes: u64 = 0;

        for file_name in [META_DB, TOKENS_BIN, CONFIG_TOML, "content.bin", "trigrams.bin"] {
            let file_path = self.cache_path.join(file_name);
            if let Ok(metadata) = std::fs::metadata(&file_path) {
                index_size_bytes += metadata.len();
            }
        }

        // Get file count breakdown by language
        let mut files_by_language = std::collections::HashMap::new();
        let mut stmt = conn.prepare("SELECT language, COUNT(*) FROM files GROUP BY language")?;
        let lang_counts = stmt.query_map([], |row| {
            let language: String = row.get(0)?;
            let count: i64 = row.get(1)?;
            Ok((language, count as usize))
        })?;

        for result in lang_counts {
            let (language, count) = result?;
            files_by_language.insert(language, count);
        }

        // Get symbol count breakdown by language
        let mut symbols_by_language = std::collections::HashMap::new();
        let mut stmt = conn.prepare("SELECT language, SUM(symbol_count) FROM files GROUP BY language")?;
        let symbol_counts = stmt.query_map([], |row| {
            let language: String = row.get(0)?;
            let count: i64 = row.get(1)?;
            Ok((language, count as usize))
        })?;

        for result in symbol_counts {
            let (language, count) = result?;
            symbols_by_language.insert(language, count);
        }

        // Get line count breakdown by language
        let mut lines_by_language = std::collections::HashMap::new();
        let mut stmt = conn.prepare("SELECT language, SUM(line_count) FROM files GROUP BY language")?;
        let line_counts = stmt.query_map([], |row| {
            let language: String = row.get(0)?;
            let count: i64 = row.get(1)?;
            Ok((language, count as usize))
        })?;

        for result in line_counts {
            let (language, count) = result?;
            lines_by_language.insert(language, count);
        }

        Ok(crate::models::IndexStats {
            total_files,
            total_symbols,
            index_size_bytes,
            last_updated,
            files_by_language,
            symbols_by_language,
            lines_by_language,
        })
    }

    // ===== Branch-aware indexing methods =====

    /// Record a file's hash for a specific branch
    pub fn record_branch_file(
        &self,
        path: &str,
        branch: &str,
        hash: &str,
        commit_sha: Option<&str>,
    ) -> Result<()> {
        let db_path = self.cache_path.join(META_DB);
        let conn = Connection::open(&db_path)
            .context("Failed to open meta.db for branch file recording")?;

        let now = chrono::Utc::now().timestamp();

        conn.execute(
            "INSERT OR REPLACE INTO file_branches (path, branch, hash, commit_sha, last_indexed)
             VALUES (?, ?, ?, ?, ?)",
            [
                path,
                branch,
                hash,
                commit_sha.unwrap_or(""),
                &now.to_string(),
            ],
        )?;

        Ok(())
    }

    /// Batch record multiple files for a specific branch in a single transaction
    pub fn batch_record_branch_files(
        &self,
        files: &[(String, String)],  // (path, hash)
        branch: &str,
        commit_sha: Option<&str>,
    ) -> Result<()> {
        let db_path = self.cache_path.join(META_DB);
        let mut conn = Connection::open(&db_path)
            .context("Failed to open meta.db for batch branch recording")?;

        let now = chrono::Utc::now().timestamp();
        let now_str = now.to_string();
        let commit = commit_sha.unwrap_or("");

        // Use a transaction for batch inserts
        let tx = conn.transaction()?;

        for (path, hash) in files {
            tx.execute(
                "INSERT OR REPLACE INTO file_branches (path, branch, hash, commit_sha, last_indexed)
                 VALUES (?, ?, ?, ?, ?)",
                [path.as_str(), branch, hash.as_str(), commit, &now_str],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// Get all files indexed for a specific branch
    ///
    /// Returns a HashMap of path → hash for all files in the branch.
    pub fn get_branch_files(&self, branch: &str) -> Result<HashMap<String, String>> {
        let db_path = self.cache_path.join(META_DB);

        if !db_path.exists() {
            return Ok(HashMap::new());
        }

        let conn = Connection::open(&db_path)
            .context("Failed to open meta.db")?;

        let mut stmt = conn.prepare("SELECT path, hash FROM file_branches WHERE branch = ?")?;
        let files: HashMap<String, String> = stmt
            .query_map([branch], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<HashMap<_, _>, _>>()?;

        log::debug!(
            "Loaded {} files for branch '{}' from file_branches table",
            files.len(),
            branch
        );
        Ok(files)
    }

    /// Check if a branch has any indexed files
    ///
    /// Fast existence check using LIMIT 1 for O(1) performance.
    pub fn branch_exists(&self, branch: &str) -> Result<bool> {
        let db_path = self.cache_path.join(META_DB);

        if !db_path.exists() {
            return Ok(false);
        }

        let conn = Connection::open(&db_path)
            .context("Failed to open meta.db")?;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM file_branches WHERE branch = ? LIMIT 1",
                [branch],
                |row| row.get(0),
            )
            .unwrap_or(0);

        Ok(count > 0)
    }

    /// Get branch metadata (commit, last_indexed, file_count, dirty status)
    pub fn get_branch_info(&self, branch: &str) -> Result<BranchInfo> {
        let db_path = self.cache_path.join(META_DB);

        if !db_path.exists() {
            anyhow::bail!("Database not initialized");
        }

        let conn = Connection::open(&db_path)
            .context("Failed to open meta.db")?;

        let info = conn.query_row(
            "SELECT commit_sha, last_indexed, file_count, is_dirty FROM branches WHERE branch = ?",
            [branch],
            |row| {
                Ok(BranchInfo {
                    branch: branch.to_string(),
                    commit_sha: row.get(0)?,
                    last_indexed: row.get(1)?,
                    file_count: row.get(2)?,
                    is_dirty: row.get::<_, i64>(3)? != 0,
                })
            },
        )?;

        Ok(info)
    }

    /// Update branch metadata after indexing
    pub fn update_branch_metadata(
        &self,
        branch: &str,
        commit_sha: Option<&str>,
        file_count: usize,
        is_dirty: bool,
    ) -> Result<()> {
        let db_path = self.cache_path.join(META_DB);
        let conn = Connection::open(&db_path)
            .context("Failed to open meta.db for branch metadata update")?;

        let now = chrono::Utc::now().timestamp();

        conn.execute(
            "INSERT OR REPLACE INTO branches (branch, commit_sha, last_indexed, file_count, is_dirty)
             VALUES (?, ?, ?, ?, ?)",
            [
                branch,
                commit_sha.unwrap_or("unknown"),
                &now.to_string(),
                &file_count.to_string(),
                &(if is_dirty { 1 } else { 0 }).to_string(),
            ],
        )?;

        log::debug!(
            "Updated branch metadata for '{}': commit={}, files={}, dirty={}",
            branch,
            commit_sha.unwrap_or("unknown"),
            file_count,
            is_dirty
        );
        Ok(())
    }

    /// Find a file with a specific hash (for symbol reuse optimization)
    ///
    /// Returns the path and branch where this hash was first seen,
    /// enabling reuse of parsed symbols across branches.
    pub fn find_file_with_hash(&self, hash: &str) -> Result<Option<(String, String)>> {
        let db_path = self.cache_path.join(META_DB);

        if !db_path.exists() {
            return Ok(None);
        }

        let conn = Connection::open(&db_path)
            .context("Failed to open meta.db")?;

        let result = conn
            .query_row(
                "SELECT path, branch FROM file_branches WHERE hash = ? LIMIT 1",
                [hash],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;

        Ok(result)
    }
}

/// Branch metadata information
#[derive(Debug, Clone)]
pub struct BranchInfo {
    pub branch: String,
    pub commit_sha: String,
    pub last_indexed: i64,
    pub file_count: usize,
    pub is_dirty: bool,
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
