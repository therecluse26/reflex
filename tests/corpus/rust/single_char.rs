//! Test Corpus: Single Character Identifiers
//!
//! Expected symbols: 10+ functions with single-char names
//!
//! Edge cases tested:
//! - Single letter function names
//! - Mathematical/Greek letter names
//! - Underscore identifiers

pub fn a() {
    println!("a");
}

fn b() {
    println!("b");
}

pub fn x() -> i32 {
    42
}

fn y() -> i32 {
    100
}

pub fn f(x: i32) -> i32 {
    x * 2
}

fn g(x: i32) -> i32 {
    x + 1
}

pub fn _() {
    println!("underscore");
}

fn z() {
    println!("z");
}

pub fn i() -> usize {
    0
}

fn j() -> usize {
    1
}
