//! Test Corpus: Raw Identifiers
//!
//! Expected symbols: 10 functions/structs with raw identifiers
//!
//! Edge cases tested:
//! - Keywords as identifiers (r#type, r#fn, r#match)
//! - Raw identifiers in different contexts

pub fn r#type() {
    println!("type is a keyword");
}

fn r#fn() {
    println!("fn is a keyword");
}

pub fn r#match() {
    println!("match is a keyword");
}

fn r#struct() {
    println!("struct is a keyword");
}

pub struct r#enum {
    value: i32,
}

struct r#trait {
    name: String,
}

pub fn r#impl() -> i32 {
    42
}

fn r#self() {
    println!("self is a keyword");
}

pub fn r#super() {
    println!("super is a keyword");
}

fn r#crate() {
    println!("crate is a keyword");
}
