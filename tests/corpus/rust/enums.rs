//! Test Corpus: Rust Enums
//!
//! Expected symbols: 8 enums
//! - 2 C-like enums (Direction, Status)
//! - 3 data-carrying enums (Option2, Result2, Message)
//! - 2 generic enums (Either, Tree)
//! - 1 enum with methods (Color)
//!
//! Edge cases tested:
//! - Unit variants
//! - Tuple variants
//! - Struct variants
//! - Generic parameters
//! - Discriminant values

pub enum Direction {
    North,
    South,
    East,
    West,
}

enum Status {
    Pending = 0,
    Active = 1,
    Completed = 2,
    Failed = -1,
}

// Data-carrying enums
pub enum Option2<T> {
    Some2(T),
    None2,
}

enum Result2<T, E> {
    Ok2(T),
    Err2(E),
}

pub enum Message {
    Quit,
    Move { x: i32, y: i32 },
    Write(String),
    ChangeColor(i32, i32, i32),
}

// Generic enum
pub enum Either<L, R> {
    Left(L),
    Right(R),
}

enum Tree<T> {
    Leaf(T),
    Node {
        left: Box<Tree<T>>,
        right: Box<Tree<T>>,
        value: T,
    },
}

// Enum with implementation
pub enum Color {
    Red,
    Green,
    Blue,
    Custom { r: u8, g: u8, b: u8 },
}

impl Color {
    pub fn to_hex(&self) -> String {
        String::from("#000000")
    }
}
