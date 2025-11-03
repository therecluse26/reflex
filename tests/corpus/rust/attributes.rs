//! Test Corpus: Attributes
//!
//! Expected symbols: 12+ functions/structs with various attributes
//!
//! Edge cases tested:
//! - Derive macros
//! - cfg attributes
//! - inline attributes
//! - test attributes
//! - doc comments as attributes
//! - Custom attributes

#[derive(Debug, Clone, PartialEq)]
pub struct Point {
    x: f64,
    y: f64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
struct Config {
    enabled: bool,
}

/// This is a documented function
#[inline]
pub fn inlined_function() -> i32 {
    42
}

#[inline(always)]
fn always_inlined() -> i32 {
    100
}

#[cfg(target_os = "linux")]
pub fn linux_only() {
    println!("Linux");
}

#[cfg(not(target_os = "windows"))]
fn not_windows() {
    println!("Not Windows");
}

#[cfg(all(unix, target_pointer_width = "64"))]
fn unix_64bit() {
    println!("Unix 64-bit");
}

#[test]
fn test_something() {
    assert_eq!(1 + 1, 2);
}

#[test]
#[ignore]
fn ignored_test() {
    panic!("Should be ignored");
}

#[allow(dead_code)]
fn unused_function() {
    println!("Unused");
}

#[deprecated(since = "1.0.0", note = "Use new_function instead")]
pub fn old_function() {
    println!("Deprecated");
}

/// Main documentation
///
/// # Examples
///
/// ```
/// documented_function();
/// ```
#[must_use]
pub fn documented_function() -> i32 {
    42
}

#[cold]
fn rarely_called() {
    panic!("Error");
}
