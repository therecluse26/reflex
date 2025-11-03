//! Test Corpus: Comments and Documentation
//!
//! Expected symbols: 8 functions
//!
//! Edge cases tested:
//! - Line comments
//! - Block comments
//! - Doc comments (///, //!)
//! - Inner doc comments
//! - Nested block comments
//! - Comments with special characters

/// This function has documentation
pub fn documented() {
    // Regular line comment
    println!("Hello");
}

/**
 * Block comment style documentation
 */
fn block_doc_comment() {
    /* Block comment */
    println!("Test");
}

pub fn with_inner_doc() {
    //! Inner doc comment
    println!("Inner");
}

// TODO: Implement this
// FIXME: Bug in this function
// NOTE: Important information
fn with_special_comments() {
    println!("Special");
}

fn with_nested_block() {
    /* Outer /* Inner */ comment */
    println!("Nested");
}

pub fn with_urls() {
    // See: https://example.com/docs
    // File: /path/to/file.rs
    // Email: test@example.com
    println!("URLs");
}

fn with_code_in_comments() {
    // Example: let x = 42;
    // let y = "string";
    println!("Code");
}

/// # Examples
///
/// ```
/// let x = example_with_code();
/// assert_eq!(x, 42);
/// ```
pub fn example_with_code() -> i32 {
    42
}
