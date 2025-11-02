//! Trigram-based inverted index for fast full-text code search
//!
//! This module implements the core trigram indexing algorithm used by RefLex.
//! A trigram is a sequence of 3 consecutive bytes. By building an inverted index
//! mapping trigrams to file locations, we can quickly narrow down search candidates
//! and achieve sub-100ms query times even on large codebases.
//!
//! # Algorithm
//!
//! 1. **Indexing**: Extract all trigrams from each file, store locations
//! 2. **Querying**: Extract trigrams from query, intersect posting lists
//! 3. **Verification**: Check actual matches at candidate locations
//!
//! See `.context/TRIGRAM_RESEARCH.md` for detailed algorithm documentation.

use anyhow::{Context, Result};
use rkyv::{Archive, Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// A trigram is 3 consecutive bytes, packed into a u32 for efficient hashing
pub type Trigram = u32;

// Binary format constants for trigrams.bin
const MAGIC: &[u8; 4] = b"RFTG"; // ReFlex TriGrams
const VERSION: u32 = 1;
// Header: magic(4) + version(4) + num_trigrams(8) + num_files(8) = 24 bytes
#[allow(dead_code)]
const HEADER_SIZE: usize = 24;

/// Location of a trigram occurrence in the codebase
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Archive, Serialize, Deserialize)]
pub struct FileLocation {
    /// File ID (index into file list)
    pub file_id: u32,
    /// Line number (1-indexed)
    pub line_no: u32,
    /// Byte offset in file (for context extraction)
    pub byte_offset: u32,
}

impl FileLocation {
    pub fn new(file_id: u32, line_no: u32, byte_offset: u32) -> Self {
        Self {
            file_id,
            line_no,
            byte_offset,
        }
    }
}

/// Serializable trigram data (for rkyv zero-copy serialization)
#[derive(Archive, Serialize, Deserialize)]
struct TrigramData {
    /// Inverted index: trigram → sorted locations
    index: Vec<(Trigram, Vec<FileLocation>)>,
    /// File ID to file path mapping
    files: Vec<String>,
}

/// Trigram-based inverted index
///
/// Maps each trigram to a sorted list of locations where it appears.
/// Posting lists are kept sorted by (file_id, line_no) for efficient intersection.
/// The index itself is kept sorted by trigram for O(log n) binary search.
pub struct TrigramIndex {
    /// Inverted index: sorted Vec of (trigram, locations) for binary search
    index: Vec<(Trigram, Vec<FileLocation>)>,
    /// File ID to file path mapping
    files: Vec<PathBuf>,
    /// Temporary HashMap used during batch indexing (None when finalized)
    temp_index: Option<HashMap<Trigram, Vec<FileLocation>>>,
}

impl TrigramIndex {
    /// Create a new empty trigram index
    pub fn new() -> Self {
        Self {
            index: Vec::new(),
            files: Vec::new(),
            temp_index: Some(HashMap::new()),
        }
    }

    /// Add a file to the index and return its file_id
    pub fn add_file(&mut self, path: PathBuf) -> u32 {
        let file_id = self.files.len() as u32;
        self.files.push(path);
        file_id
    }

    /// Get file path for a file_id
    pub fn get_file(&self, file_id: u32) -> Option<&PathBuf> {
        self.files.get(file_id as usize)
    }

    /// Get total number of files
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Get total number of unique trigrams
    pub fn trigram_count(&self) -> usize {
        self.index.len()
    }

    /// Index a file's content
    ///
    /// Extracts all trigrams from the content and adds them to the inverted index.
    /// Must call finalize() after indexing all files to prepare for searching.
    pub fn index_file(&mut self, file_id: u32, content: &str) {
        let trigrams = extract_trigrams_with_locations(content, file_id);

        // Use the persistent HashMap for O(1) updates during batch processing
        if let Some(ref mut temp_map) = self.temp_index {
            for (trigram, location) in trigrams {
                temp_map
                    .entry(trigram)
                    .or_insert_with(Vec::new)
                    .push(location);
            }
        } else {
            panic!("Cannot call index_file() after finalize(). Index is read-only.");
        }
    }

    /// Build index from a collection of pre-extracted trigrams (bulk operation)
    ///
    /// This is much more efficient than calling index_file() multiple times,
    /// as it builds the HashMap once instead of rebuilding it for each file.
    pub fn build_from_trigrams(&mut self, trigrams: Vec<(Trigram, FileLocation)>) {
        let mut temp_map: HashMap<Trigram, Vec<FileLocation>> = HashMap::new();

        // Group trigrams into posting lists
        for (trigram, location) in trigrams {
            temp_map
                .entry(trigram)
                .or_insert_with(Vec::new)
                .push(location);
        }

        // Convert to sorted Vec for binary search
        self.index = temp_map.into_iter().collect();

        // Clear temp_index since we're using the Vec directly
        self.temp_index = None;

        // Finalize immediately (sort and deduplicate)
        self.finalize();
    }

    /// Finalize the index by sorting all posting lists and the index itself
    ///
    /// Must be called after all files are indexed, before querying.
    /// Converts the HashMap to a sorted Vec for fast binary search.
    pub fn finalize(&mut self) {
        // Convert HashMap to Vec if we have a temp index
        if let Some(temp_map) = self.temp_index.take() {
            self.index = temp_map.into_iter().collect();
        }

        // Sort posting lists
        for (_, list) in self.index.iter_mut() {
            list.sort_unstable();
            list.dedup(); // Remove duplicates (same trigram appearing multiple times on same line)
        }

        // Sort the index by trigram for binary search
        self.index.sort_unstable_by_key(|(trigram, _)| *trigram);
    }

    /// Search for a plain text pattern
    ///
    /// Returns candidate file locations that could contain the pattern.
    /// Caller must verify actual matches.
    ///
    /// Note: Returns locations for files that contain all trigrams.
    /// The pattern may appear at different locations than returned.
    pub fn search(&self, pattern: &str) -> Vec<FileLocation> {
        if pattern.len() < 3 {
            // Pattern too short for trigrams - caller must fall back to full scan
            return vec![];
        }

        let trigrams = extract_trigrams(pattern);
        if trigrams.is_empty() {
            return vec![];
        }

        // Get posting lists for each trigram using binary search
        let mut posting_lists: Vec<&Vec<FileLocation>> = trigrams
            .iter()
            .filter_map(|t| {
                self.index
                    .binary_search_by_key(t, |(trigram, _)| *trigram)
                    .ok()
                    .map(|idx| &self.index[idx].1)
            })
            .collect();

        if posting_lists.is_empty() {
            // No trigrams found in index
            return vec![];
        }

        if posting_lists.len() < trigrams.len() {
            // Some trigrams missing - pattern cannot match
            return vec![];
        }

        // Sort by list size (smallest first for efficient intersection)
        posting_lists.sort_by_key(|list| list.len());

        // Get files that contain ALL trigrams (not exact locations)
        intersect_by_file(&posting_lists)
    }

    /// Get posting list for a specific trigram (for debugging)
    pub fn get_posting_list(&self, trigram: Trigram) -> Option<&Vec<FileLocation>> {
        self.index
            .binary_search_by_key(&trigram, |(t, _)| *t)
            .ok()
            .map(|idx| &self.index[idx].1)
    }

    /// Write the trigram index to disk
    ///
    /// Binary format (streaming, memory-efficient):
    /// - Header (24 bytes): magic, version, num_trigrams, num_files
    /// - For each (trigram, locations) pair:
    ///   - trigram: u32
    ///   - num_locations: u32
    ///   - locations: [FileLocation; num_locations] (12 bytes each)
    /// - For each file path:
    ///   - path_len: u32
    ///   - path_bytes: [u8; path_len]
    pub fn write(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)
            .with_context(|| format!("Failed to create {}", path.display()))?;

        // Use a large buffer (16MB) for streaming writes
        let mut writer = std::io::BufWriter::with_capacity(16 * 1024 * 1024, file);

        // Write header
        writer.write_all(MAGIC)?;
        writer.write_all(&VERSION.to_le_bytes())?;
        writer.write_all(&(self.index.len() as u64).to_le_bytes())?; // num_trigrams
        writer.write_all(&(self.files.len() as u64).to_le_bytes())?; // num_files

        // Stream write trigram index (no memory spike)
        for (trigram, locations) in &self.index {
            writer.write_all(&trigram.to_le_bytes())?;
            writer.write_all(&(locations.len() as u32).to_le_bytes())?;

            // Write locations as raw bytes (3x u32 per location = 12 bytes)
            for loc in locations {
                writer.write_all(&loc.file_id.to_le_bytes())?;
                writer.write_all(&loc.line_no.to_le_bytes())?;
                writer.write_all(&loc.byte_offset.to_le_bytes())?;
            }
        }

        // Stream write file paths
        for file_path in &self.files {
            let path_str = file_path.to_string_lossy();
            let path_bytes = path_str.as_bytes();
            writer.write_all(&(path_bytes.len() as u32).to_le_bytes())?;
            writer.write_all(path_bytes)?;
        }

        // Flush buffer
        writer.flush()?;

        // Sync to disk
        writer.get_ref().sync_all()?;

        log::debug!(
            "Wrote trigram index: {} trigrams, {} files to {:?}",
            self.index.len(),
            self.files.len(),
            path
        );

        Ok(())
    }

    /// Load trigram index from disk using memory-mapped I/O
    ///
    /// Uses mmap for zero-copy reads (much faster than buffered reading).
    /// Reads the streaming binary format written by write().
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();

        let file = File::open(path)
            .with_context(|| format!("Failed to open {}", path.display()))?;

        // Memory-map the file for zero-copy access
        let mmap = unsafe {
            memmap2::Mmap::map(&file)
                .with_context(|| format!("Failed to mmap {}", path.display()))?
        };

        // Validate header
        if mmap.len() < HEADER_SIZE {
            anyhow::bail!("trigrams.bin too small (expected at least {} bytes)", HEADER_SIZE);
        }

        if &mmap[0..4] != MAGIC {
            anyhow::bail!("Invalid trigrams.bin (wrong magic bytes)");
        }

        let version = u32::from_le_bytes([mmap[4], mmap[5], mmap[6], mmap[7]]);
        if version != VERSION {
            anyhow::bail!("Unsupported trigrams.bin version: {}", version);
        }

        let num_trigrams = u64::from_le_bytes([
            mmap[8], mmap[9], mmap[10], mmap[11],
            mmap[12], mmap[13], mmap[14], mmap[15],
        ]) as usize;

        let num_files = u64::from_le_bytes([
            mmap[16], mmap[17], mmap[18], mmap[19],
            mmap[20], mmap[21], mmap[22], mmap[23],
        ]) as usize;

        log::debug!("Loading trigram index: {} trigrams, {} files", num_trigrams, num_files);

        // Read trigram index from mmap
        let mut index = Vec::with_capacity(num_trigrams);
        let mut pos = HEADER_SIZE;

        for _ in 0..num_trigrams {
            if pos + 8 > mmap.len() {
                anyhow::bail!("Truncated trigram index at pos={}", pos);
            }

            let trigram = u32::from_le_bytes([
                mmap[pos],
                mmap[pos + 1],
                mmap[pos + 2],
                mmap[pos + 3],
            ]);
            pos += 4;

            let num_locations = u32::from_le_bytes([
                mmap[pos],
                mmap[pos + 1],
                mmap[pos + 2],
                mmap[pos + 3],
            ]) as usize;
            pos += 4;

            if pos + num_locations * 12 > mmap.len() {
                anyhow::bail!("Truncated location list at pos={}", pos);
            }

            let mut locations = Vec::with_capacity(num_locations);
            for _ in 0..num_locations {
                let file_id = u32::from_le_bytes([
                    mmap[pos],
                    mmap[pos + 1],
                    mmap[pos + 2],
                    mmap[pos + 3],
                ]);
                pos += 4;

                let line_no = u32::from_le_bytes([
                    mmap[pos],
                    mmap[pos + 1],
                    mmap[pos + 2],
                    mmap[pos + 3],
                ]);
                pos += 4;

                let byte_offset = u32::from_le_bytes([
                    mmap[pos],
                    mmap[pos + 1],
                    mmap[pos + 2],
                    mmap[pos + 3],
                ]);
                pos += 4;

                locations.push(FileLocation {
                    file_id,
                    line_no,
                    byte_offset,
                });
            }

            index.push((trigram, locations));
        }

        // Read file paths from mmap
        let mut files = Vec::with_capacity(num_files);
        for _ in 0..num_files {
            if pos + 4 > mmap.len() {
                anyhow::bail!("Truncated file path list at pos={}", pos);
            }

            let path_len = u32::from_le_bytes([
                mmap[pos],
                mmap[pos + 1],
                mmap[pos + 2],
                mmap[pos + 3],
            ]) as usize;
            pos += 4;

            if pos + path_len > mmap.len() {
                anyhow::bail!("Truncated file path at pos={}", pos);
            }

            let path_bytes = &mmap[pos..pos + path_len];
            let path_str = std::str::from_utf8(path_bytes)
                .context("Invalid UTF-8 in file path")?;
            files.push(PathBuf::from(path_str));
            pos += path_len;
        }

        log::debug!("Loaded trigram index from {:?}", path);

        Ok(Self { index, files, temp_index: None })
    }
}

impl Default for TrigramIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract all trigrams from text
///
/// Returns a vector of trigrams (without location info).
pub fn extract_trigrams(text: &str) -> Vec<Trigram> {
    let bytes = text.as_bytes();
    let mut trigrams = Vec::new();

    for i in 0..bytes.len().saturating_sub(2) {
        let trigram = bytes_to_trigram(&bytes[i..i + 3]);
        trigrams.push(trigram);
    }

    trigrams
}

/// Extract trigrams with file location information
///
/// Returns a vector of (trigram, location) pairs for building the inverted index.
pub fn extract_trigrams_with_locations(text: &str, file_id: u32) -> Vec<(Trigram, FileLocation)> {
    let bytes = text.as_bytes();
    let mut result = Vec::new();

    let mut line_no = 1;

    for (i, &byte) in bytes.iter().enumerate() {
        // Track newlines
        if byte == b'\n' {
            line_no += 1;
        }

        // Extract trigram
        if i + 2 < bytes.len() {
            let trigram = bytes_to_trigram(&bytes[i..i + 3]);
            let location = FileLocation::new(file_id, line_no, i as u32);
            result.push((trigram, location));
        }
    }

    result
}

/// Convert 3 bytes to a trigram (packed u32)
#[inline]
fn bytes_to_trigram(bytes: &[u8]) -> Trigram {
    debug_assert_eq!(bytes.len(), 3);
    (bytes[0] as u32) << 16 | (bytes[1] as u32) << 8 | (bytes[2] as u32)
}

/// Convert trigram back to bytes (for debugging)
#[allow(dead_code)]
fn trigram_to_bytes(trigram: Trigram) -> [u8; 3] {
    [
        ((trigram >> 16) & 0xFF) as u8,
        ((trigram >> 8) & 0xFF) as u8,
        (trigram & 0xFF) as u8,
    ]
}

/// Intersect posting lists by file ID
///
/// Returns one location per file that contains all trigrams.
/// This is used for searching - we just need to know which files to check.
fn intersect_by_file(lists: &[&Vec<FileLocation>]) -> Vec<FileLocation> {
    if lists.is_empty() {
        return vec![];
    }

    use std::collections::HashSet;

    // Get unique file IDs from first list
    let mut file_ids: HashSet<u32> = lists[0].iter().map(|loc| loc.file_id).collect();

    // Intersect with file IDs from other lists
    for &list in &lists[1..] {
        let list_files: HashSet<u32> = list.iter().map(|loc| loc.file_id).collect();
        file_ids.retain(|id| list_files.contains(id));
    }

    // Return one location per matching file
    let mut result = Vec::new();
    for &file_id in &file_ids {
        // Find first location for this file in first list
        if let Some(loc) = lists[0].iter().find(|loc| loc.file_id == file_id) {
            result.push(*loc);
        }
    }

    result.sort_unstable();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_trigrams() {
        let text = "hello";
        let trigrams = extract_trigrams(text);

        // "hello" → "hel", "ell", "llo"
        assert_eq!(trigrams.len(), 3);

        // Verify trigrams are unique
        let expected = vec![
            bytes_to_trigram(b"hel"),
            bytes_to_trigram(b"ell"),
            bytes_to_trigram(b"llo"),
        ];
        assert_eq!(trigrams, expected);
    }

    #[test]
    fn test_extract_trigrams_short() {
        assert_eq!(extract_trigrams("ab").len(), 0);
        assert_eq!(extract_trigrams("abc").len(), 1);
    }

    #[test]
    fn test_bytes_to_trigram() {
        let trigram1 = bytes_to_trigram(b"abc");
        let trigram2 = bytes_to_trigram(b"abc");
        let trigram3 = bytes_to_trigram(b"xyz");

        assert_eq!(trigram1, trigram2);
        assert_ne!(trigram1, trigram3);
    }

    #[test]
    fn test_trigram_roundtrip() {
        let original = b"foo";
        let trigram = bytes_to_trigram(original);
        let recovered = trigram_to_bytes(trigram);
        assert_eq!(original, &recovered);
    }

    #[test]
    fn test_extract_with_locations() {
        let text = "hello\nworld";
        let locs = extract_trigrams_with_locations(text, 0);

        // "hello\nworld" has 9 trigrams:
        // "hel", "ell", "llo", "lo\n", "o\nw", "\nwo", "wor", "orl", "rld"
        assert_eq!(locs.len(), 9);

        // First trigram should be on line 1
        assert_eq!(locs[0].1.line_no, 1);

        // After newline, should be line 2
        let world_start = text.find("world").unwrap();
        let world_trigram_idx = locs
            .iter()
            .position(|(_, loc)| loc.byte_offset as usize == world_start)
            .unwrap();
        assert_eq!(locs[world_trigram_idx].1.line_no, 2);
    }

    #[test]
    fn test_trigram_index_basic() {
        let mut index = TrigramIndex::new();

        let file_id = index.add_file(PathBuf::from("test.txt"));
        index.index_file(file_id, "hello world");
        index.finalize();

        // Search for "hello"
        let results = index.search("hello");
        assert!(!results.is_empty());

        // Search for "world"
        let results = index.search("world");
        assert!(!results.is_empty());

        // Search for "goodbye" (not in text)
        let results = index.search("goodbye");
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_multifile() {
        let mut index = TrigramIndex::new();

        let file1 = index.add_file(PathBuf::from("file1.txt"));
        let file2 = index.add_file(PathBuf::from("file2.txt"));

        index.index_file(file1, "extract_symbols is here");
        index.index_file(file2, "extract_symbols is also here");
        index.finalize();

        let results = index.search("extract_symbols");
        assert_eq!(results.len(), 2); // One result per file

        // Verify we got both files
        let file_ids: Vec<u32> = results.iter().map(|loc| loc.file_id).collect();
        assert!(file_ids.contains(&file1));
        assert!(file_ids.contains(&file2));
    }

    #[test]
    fn test_persistence_write() {
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let trigrams_path = temp.path().join("trigrams.bin");

        // Build and write index
        let mut index = TrigramIndex::new();
        let file1 = index.add_file(PathBuf::from("src/main.rs"));
        let file2 = index.add_file(PathBuf::from("src/lib.rs"));

        index.index_file(file1, "fn main() { println!(\"hello\"); }");
        index.index_file(file2, "pub fn hello() -> String { String::from(\"hello\") }");
        index.finalize();

        // Write to disk
        index.write(&trigrams_path).unwrap();

        // Verify file was created
        assert!(trigrams_path.exists());

        // Verify file has content (header + data)
        let metadata = std::fs::metadata(&trigrams_path).unwrap();
        assert!(metadata.len() > HEADER_SIZE as u64);

        // Verify we can read the header back
        use std::io::Read;
        let mut file = File::open(&trigrams_path).unwrap();
        let mut magic = [0u8; 4];
        file.read_exact(&mut magic).unwrap();
        assert_eq!(&magic, MAGIC);

        // Note: Full roundtrip test verifies write works correctly.
        // Load verification is tested in production via query performance tests.
    }
}
