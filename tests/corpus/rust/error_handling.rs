//! Test Corpus: Error Handling Patterns
//!
//! Expected symbols: 12+ functions demonstrating error handling
//!
//! Real-world patterns tested:
//! - Result<T, E> usage
//! - Option<T> usage
//! - unwrap, expect, ?
//! - Error propagation

use std::fs::File;
use std::io::{self, Read};

pub fn returns_result() -> Result<i32, String> {
    Ok(42)
}

fn returns_option() -> Option<String> {
    Some("value".to_string())
}

pub fn uses_unwrap() -> i32 {
    Some(42).unwrap()
}

fn uses_expect() -> String {
    Some("test".to_string()).expect("Should have value")
}

pub fn uses_question_mark() -> Result<String, io::Error> {
    let mut file = File::open("test.txt")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

fn unwrap_or() -> i32 {
    None.unwrap_or(42)
}

pub fn unwrap_or_else() -> String {
    None.unwrap_or_else(|| "default".to_string())
}

fn ok_or() -> Result<i32, String> {
    Some(42).ok_or("error".to_string())
}

pub fn and_then_usage() -> Option<i32> {
    Some(42).and_then(|x| Some(x * 2))
}

fn map_usage() -> Option<String> {
    Some(42).map(|x| x.to_string())
}

pub fn match_result() -> i32 {
    match returns_result() {
        Ok(val) => val,
        Err(e) => {
            eprintln!("Error: {}", e);
            0
        }
    }
}

fn if_let_pattern() -> i32 {
    if let Some(x) = Some(42) {
        x
    } else {
        0
    }
}
