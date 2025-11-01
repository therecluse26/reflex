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
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// A trigram is 3 consecutive bytes, packed into a u32 for efficient hashing
pub type Trigram = u32;

/// Location of a trigram occurrence in the codebase
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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

/// Trigram-based inverted index
///
/// Maps each trigram to a sorted list of locations where it appears.
/// Posting lists are kept sorted by (file_id, line_no) for efficient intersection.
pub struct TrigramIndex {
    /// Inverted index: trigram → sorted locations
    index: HashMap<Trigram, Vec<FileLocation>>,
    /// File ID to file path mapping
    files: Vec<PathBuf>,
}

impl TrigramIndex {
    /// Create a new empty trigram index
    pub fn new() -> Self {
        Self {
            index: HashMap::new(),
            files: Vec::new(),
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
    pub fn index_file(&mut self, file_id: u32, content: &str) {
        let trigrams = extract_trigrams_with_locations(content, file_id);

        for (trigram, location) in trigrams {
            self.index
                .entry(trigram)
                .or_insert_with(Vec::new)
                .push(location);
        }
    }

    /// Finalize the index by sorting all posting lists
    ///
    /// Must be called after all files are indexed, before querying.
    pub fn finalize(&mut self) {
        for list in self.index.values_mut() {
            list.sort_unstable();
            list.dedup(); // Remove duplicates (same trigram appearing multiple times on same line)
        }
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

        // Get posting lists for each trigram
        let mut posting_lists: Vec<&Vec<FileLocation>> = trigrams
            .iter()
            .filter_map(|t| self.index.get(t))
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
        self.index.get(&trigram)
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
    let mut line_start = 0;

    for (i, &byte) in bytes.iter().enumerate() {
        // Track newlines
        if byte == b'\n' {
            line_no += 1;
            line_start = i + 1;
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

/// Intersect multiple sorted posting lists
///
/// Returns locations that appear in ALL lists.
/// Uses efficient multi-way merge algorithm.
fn intersect_all_lists(lists: &[&Vec<FileLocation>]) -> Vec<FileLocation> {
    if lists.is_empty() {
        return vec![];
    }

    if lists.len() == 1 {
        return lists[0].clone();
    }

    // Start with smallest list
    let mut result = lists[0].clone();

    // Intersect with each subsequent list
    for &list in &lists[1..] {
        result = intersect_two_lists(&result, list);
        if result.is_empty() {
            // Early exit if no candidates remain
            break;
        }
    }

    result
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

/// Intersect two sorted lists in O(n+m) time
fn intersect_two_lists(a: &[FileLocation], b: &[FileLocation]) -> Vec<FileLocation> {
    let mut result = Vec::new();
    let (mut i, mut j) = (0, 0);

    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            std::cmp::Ordering::Equal => {
                result.push(a[i]);
                i += 1;
                j += 1;
            }
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
        }
    }

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
    fn test_intersect_two_lists() {
        let a = vec![
            FileLocation::new(0, 1, 0),
            FileLocation::new(0, 2, 10),
            FileLocation::new(0, 3, 20),
        ];

        let b = vec![
            FileLocation::new(0, 2, 10),
            FileLocation::new(0, 3, 20),
            FileLocation::new(0, 4, 30),
        ];

        let result = intersect_two_lists(&a, &b);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0], FileLocation::new(0, 2, 10));
        assert_eq!(result[1], FileLocation::new(0, 3, 20));
    }

    #[test]
    fn test_intersect_no_overlap() {
        let a = vec![FileLocation::new(0, 1, 0), FileLocation::new(0, 2, 10)];

        let b = vec![FileLocation::new(0, 3, 20), FileLocation::new(0, 4, 30)];

        let result = intersect_two_lists(&a, &b);
        assert!(result.is_empty());
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
}
