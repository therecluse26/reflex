//! Test Corpus: Rust Structs
//!
//! Expected symbols: 11 structs
//! - 3 regular structs (Point, Person, Config)
//! - 2 tuple structs (Color, Pair)
//! - 1 unit struct (Marker)
//! - 3 generic structs (Container, GenericPair, PhantomStruct)
//! - 2 structs with lifetimes (RefHolder, ComplexRefs)
//!
//! Edge cases tested:
//! - Named fields
//! - Tuple fields
//! - Unit structs
//! - Generic parameters
//! - Lifetime parameters
//! - PhantomData usage

pub struct Point {
    pub x: f64,
    pub y: f64,
}

struct Person {
    name: String,
    age: u32,
    email: Option<String>,
}

pub struct Config {
    pub host: String,
    pub port: u16,
    pub timeout_ms: u64,
}

// Tuple structs
pub struct Color(pub u8, pub u8, pub u8);

struct Pair(i32, i32);

// Unit struct
pub struct Marker;

// Generic structs
pub struct Container<T> {
    value: T,
}

struct GenericPair<T, U> {
    first: T,
    second: U,
}

pub struct PhantomStruct<T> {
    marker: std::marker::PhantomData<T>,
    data: Vec<u8>,
}

// Structs with lifetimes
pub struct RefHolder<'a> {
    data: &'a str,
}

struct ComplexRefs<'a, 'b, T>
where
    T: 'a + 'b,
{
    first: &'a T,
    second: &'b T,
}
