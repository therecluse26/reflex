//! User-facing output utilities for clean, colored terminal messages
//!
//! This module provides functions for displaying warnings and errors to users
//! in a friendly, colored format without internal logging noise (timestamps,
//! log levels, crate names, etc.).

use owo_colors::OwoColorize;

/// Display a warning message to the user in yellow with padding
///
/// Format: blank line + yellow message + blank line
///
/// # Example
/// ```ignore
/// output::warn("Pattern matched 4951 files - parsing may take some time.");
/// ```
pub fn warn(message: &str) {
    eprintln!("\n{}\n", message.yellow());
}

/// Display an error message to the user in red with padding
///
/// Format: blank line + red message + blank line
///
/// # Example
/// ```ignore
/// output::error("Index not found. Run 'rfx index' to build the cache first.");
/// ```
pub fn error(message: &str) {
    eprintln!("\n{}\n", message.red());
}

/// Display an informational message to the user in default color with padding
///
/// Format: blank line + message + blank line
///
/// # Example
/// ```ignore
/// output::info("Indexing completed successfully.");
/// ```
pub fn info(message: &str) {
    eprintln!("\n{}\n", message);
}
