//! Content store for memory-mapped file access
//!
//! This module stores the full contents of all indexed files in a single
//! memory-mapped file. This enables zero-copy access to file contents for:
//! - Verifying trigram matches
//! - Extracting context around matches
//! - Fast content retrieval without disk I/O
//!
//! # Binary Format (content.bin)
//!
//! ```text
//! Header (32 bytes):
//!   magic: "RFCT" (4 bytes)
//!   version: 1 (u32)
//!   num_files: N (u64)
//!   index_offset: offset to file index (u64)
//!   reserved: 8 bytes
//!
//! File Contents (variable):
//!   [Concatenated file contents]
//!
//! File Index (at index_offset):
//!   For each file:
//!     path_len: u32
//!     path: UTF-8 string
//!     offset: u64 (byte offset to file content)
//!     length: u64 (file size in bytes)
//! ```

use anyhow::{Context, Result};
use memmap2::Mmap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

const MAGIC: &[u8; 4] = b"RFCT";
const VERSION: u32 = 1;
const HEADER_SIZE: usize = 32; // 4 (magic) + 4 (version) + 8 (num_files) + 8 (index_offset) + 8 (reserved)

/// Metadata for a file in the content store
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// File path
    pub path: PathBuf,
    /// Byte offset in content.bin where this file's content starts
    pub offset: u64,
    /// Length of this file's content in bytes
    pub length: u64,
}

/// Writer for building content.bin
///
/// Accumulates file contents and builds the index, then writes everything
/// to disk in one pass.
pub struct ContentWriter {
    files: Vec<FileEntry>,
    content: Vec<u8>,
}

impl ContentWriter {
    /// Create a new content writer
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            content: Vec::new(),
        }
    }

    /// Add a file to the content store
    ///
    /// Returns the file_id (index into files array)
    pub fn add_file(&mut self, path: PathBuf, content: &str) -> u32 {
        let file_id = self.files.len() as u32;
        let offset = self.content.len() as u64;
        let length = content.len() as u64;

        // Append content
        self.content.extend_from_slice(content.as_bytes());

        // Record metadata
        self.files.push(FileEntry {
            path,
            offset,
            length,
        });

        file_id
    }

    /// Write the content store to disk
    pub fn write(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)
            .with_context(|| format!("Failed to create {}", path.display()))?;

        // Calculate index offset (after header + content)
        let index_offset = HEADER_SIZE as u64 + self.content.len() as u64;

        // Write header
        file.write_all(MAGIC)?;
        file.write_all(&VERSION.to_le_bytes())?;
        file.write_all(&(self.files.len() as u64).to_le_bytes())?;
        file.write_all(&index_offset.to_le_bytes())?;
        file.write_all(&[0u8; 8])?; // reserved

        // Write file contents
        file.write_all(&self.content)?;

        // Write file index
        for entry in &self.files {
            let path_str = entry.path.to_string_lossy();
            let path_bytes = path_str.as_bytes();

            file.write_all(&(path_bytes.len() as u32).to_le_bytes())?;
            file.write_all(path_bytes)?;
            file.write_all(&entry.offset.to_le_bytes())?;
            file.write_all(&entry.length.to_le_bytes())?;
        }

        file.flush()?;
        Ok(())
    }

    /// Get the number of files
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Get total content size
    pub fn content_size(&self) -> usize {
        self.content.len()
    }
}

impl Default for ContentWriter {
    fn default() -> Self {
        Self::new()
    }
}

/// Reader for memory-mapped content.bin
///
/// Provides zero-copy access to file contents.
pub struct ContentReader {
    _file: File,
    mmap: Mmap,
    files: Vec<FileEntry>,
}

impl ContentReader {
    /// Open and memory-map content.bin
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();

        let file = File::open(path)
            .with_context(|| format!("Failed to open {}", path.display()))?;

        let mmap = unsafe {
            Mmap::map(&file)
                .with_context(|| format!("Failed to mmap {}", path.display()))?
        };

        // Validate header
        if mmap.len() < HEADER_SIZE {
            anyhow::bail!("content.bin too small (expected at least {} bytes)", HEADER_SIZE);
        }

        if &mmap[0..4] != MAGIC {
            anyhow::bail!("Invalid content.bin (wrong magic bytes)");
        }

        let version = u32::from_le_bytes([mmap[4], mmap[5], mmap[6], mmap[7]]);
        if version != VERSION {
            anyhow::bail!("Unsupported content.bin version: {}", version);
        }

        let num_files = u64::from_le_bytes([
            mmap[8], mmap[9], mmap[10], mmap[11],
            mmap[12], mmap[13], mmap[14], mmap[15],
        ]);

        let index_offset = u64::from_le_bytes([
            mmap[16], mmap[17], mmap[18], mmap[19],
            mmap[20], mmap[21], mmap[22], mmap[23],
        ]) as usize;

        // Read file index
        let mut files = Vec::new();
        let mut pos = index_offset;

        for i in 0..num_files {
            if pos + 4 > mmap.len() {
                anyhow::bail!("Truncated file index at file {} (pos={}, mmap.len()={})", i, pos, mmap.len());
            }

            let path_len = u32::from_le_bytes([
                mmap[pos],
                mmap[pos + 1],
                mmap[pos + 2],
                mmap[pos + 3],
            ]) as usize;
            pos += 4;

            if pos + path_len + 16 > mmap.len() {
                anyhow::bail!("Truncated file entry at file {} (pos={}, path_len={}, need={}, mmap.len()={})",
                    i, pos, path_len, pos + path_len + 16, mmap.len());
            }

            let path_bytes = &mmap[pos..pos + path_len];
            let path_str = std::str::from_utf8(path_bytes)
                .context("Invalid UTF-8 in file path")?;
            let path = PathBuf::from(path_str);
            pos += path_len;

            let offset = u64::from_le_bytes([
                mmap[pos],
                mmap[pos + 1],
                mmap[pos + 2],
                mmap[pos + 3],
                mmap[pos + 4],
                mmap[pos + 5],
                mmap[pos + 6],
                mmap[pos + 7],
            ]);
            pos += 8;

            let length = u64::from_le_bytes([
                mmap[pos],
                mmap[pos + 1],
                mmap[pos + 2],
                mmap[pos + 3],
                mmap[pos + 4],
                mmap[pos + 5],
                mmap[pos + 6],
                mmap[pos + 7],
            ]);
            pos += 8;

            files.push(FileEntry {
                path,
                offset,
                length,
            });
        }

        Ok(Self {
            _file: file,
            mmap,
            files,
        })
    }

    /// Get file content by file_id
    pub fn get_file_content(&self, file_id: u32) -> Result<&str> {
        let entry = self.files
            .get(file_id as usize)
            .ok_or_else(|| anyhow::anyhow!("Invalid file_id: {}", file_id))?;

        let start = HEADER_SIZE + entry.offset as usize;
        let end = start + entry.length as usize;

        if end > self.mmap.len() {
            anyhow::bail!("File content out of bounds");
        }

        let bytes = &self.mmap[start..end];
        std::str::from_utf8(bytes).context("Invalid UTF-8 in file content")
    }

    /// Get file path by file_id
    pub fn get_file_path(&self, file_id: u32) -> Option<&Path> {
        self.files.get(file_id as usize).map(|e| e.path.as_path())
    }

    /// Get number of files
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Get content at a specific byte offset
    pub fn get_content_at_offset(&self, file_id: u32, byte_offset: u32, length: usize) -> Result<&str> {
        let entry = self.files
            .get(file_id as usize)
            .ok_or_else(|| anyhow::anyhow!("Invalid file_id: {}", file_id))?;

        let start = HEADER_SIZE + entry.offset as usize + byte_offset as usize;
        let end = start + length;

        if end > self.mmap.len() {
            anyhow::bail!("Content out of bounds");
        }

        let bytes = &self.mmap[start..end];
        std::str::from_utf8(bytes).context("Invalid UTF-8 in content")
    }

    /// Get context around a byte offset (for showing match results)
    ///
    /// Returns (lines_before, matching_line, lines_after)
    pub fn get_context(&self, file_id: u32, byte_offset: u32, context_lines: usize) -> Result<(Vec<String>, String, Vec<String>)> {
        let content = self.get_file_content(file_id)?;
        let lines: Vec<&str> = content.lines().collect();

        // Find which line contains this byte offset
        let mut current_offset = 0;
        let mut line_idx = 0;

        for (idx, line) in lines.iter().enumerate() {
            let line_end = current_offset + line.len() + 1; // +1 for newline
            if byte_offset as usize >= current_offset && (byte_offset as usize) < line_end {
                line_idx = idx;
                break;
            }
            current_offset = line_end;
        }

        // Extract context
        let start = line_idx.saturating_sub(context_lines);
        let end = (line_idx + context_lines + 1).min(lines.len());

        let before: Vec<String> = lines[start..line_idx]
            .iter()
            .map(|s| s.to_string())
            .collect();

        let matching = lines.get(line_idx)
            .map(|s| s.to_string())
            .unwrap_or_default();

        let after: Vec<String> = lines[line_idx + 1..end]
            .iter()
            .map(|s| s.to_string())
            .collect();

        Ok((before, matching, after))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_content_writer_basic() {
        let mut writer = ContentWriter::new();

        let file1_id = writer.add_file(PathBuf::from("test1.txt"), "Hello, world!");
        let file2_id = writer.add_file(PathBuf::from("test2.txt"), "Goodbye, world!");

        assert_eq!(file1_id, 0);
        assert_eq!(file2_id, 1);
        assert_eq!(writer.file_count(), 2);
    }

    #[test]
    fn test_content_roundtrip() {
        let temp = TempDir::new().unwrap();
        let content_path = temp.path().join("content.bin");

        // Write
        let mut writer = ContentWriter::new();
        writer.add_file(PathBuf::from("file1.txt"), "First file content");
        writer.add_file(PathBuf::from("file2.txt"), "Second file content");
        writer.write(&content_path).unwrap();

        // Read
        let reader = ContentReader::open(&content_path).unwrap();

        assert_eq!(reader.file_count(), 2);
        assert_eq!(reader.get_file_content(0).unwrap(), "First file content");
        assert_eq!(reader.get_file_content(1).unwrap(), "Second file content");
        assert_eq!(reader.get_file_path(0).unwrap(), Path::new("file1.txt"));
        assert_eq!(reader.get_file_path(1).unwrap(), Path::new("file2.txt"));
    }

    #[test]
    fn test_get_context() {
        let temp = TempDir::new().unwrap();
        let content_path = temp.path().join("content.bin");

        let mut writer = ContentWriter::new();
        writer.add_file(
            PathBuf::from("test.txt"),
            "Line 1\nLine 2\nLine 3 with match\nLine 4\nLine 5",
        );
        writer.write(&content_path).unwrap();

        let reader = ContentReader::open(&content_path).unwrap();

        // Byte offset of "Line 3" (14 = "Line 1\n" + "Line 2\n")
        let (before, matching, after) = reader.get_context(0, 14, 1).unwrap();

        assert_eq!(before.len(), 1);
        assert_eq!(before[0], "Line 2");
        assert_eq!(matching, "Line 3 with match");
        assert_eq!(after.len(), 1);
        assert_eq!(after[0], "Line 4");
    }

    #[test]
    fn test_multiline_file() {
        let temp = TempDir::new().unwrap();
        let content_path = temp.path().join("content.bin");

        let content = "fn main() {\n    println!(\"Hello\");\n}\n";

        let mut writer = ContentWriter::new();
        writer.add_file(PathBuf::from("main.rs"), content);
        writer.write(&content_path).unwrap();

        let reader = ContentReader::open(&content_path).unwrap();
        assert_eq!(reader.get_file_content(0).unwrap(), content);
    }
}
