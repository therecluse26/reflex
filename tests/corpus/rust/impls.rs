//! Test Corpus: Rust Implementations
//!
//! This file tests method detection within impl blocks.
//!
//! Expected methods: 15+ methods across various impl blocks
//!
//! Edge cases tested:
//! - Inherent implementations
//! - Trait implementations
//! - Generic implementations
//! - Associated functions (static methods)
//! - Methods with self/&self/&mut self

pub struct Point {
    x: f64,
    y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Point { x, y }
    }

    pub fn distance(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn move_by(&mut self, dx: f64, dy: f64) {
        self.x += dx;
        self.y += dy;
    }

    fn private_method(&self) -> String {
        format!("({}, {})", self.x, self.y)
    }
}

pub trait Drawable {
    fn draw(&self);
}

impl Drawable for Point {
    fn draw(&self) {
        println!("Drawing point at ({}, {})", self.x, self.y);
    }
}

// Generic implementation
pub struct Container<T> {
    value: T,
}

impl<T> Container<T> {
    pub fn new(value: T) -> Self {
        Container { value }
    }

    pub fn get(&self) -> &T {
        &self.value
    }

    pub fn into_inner(self) -> T {
        self.value
    }
}

impl<T: Clone> Container<T> {
    pub fn clone_value(&self) -> T {
        self.value.clone()
    }
}

impl<T: Default> Default for Container<T> {
    fn default() -> Self {
        Container {
            value: T::default(),
        }
    }
}

// Static methods
pub struct Utils;

impl Utils {
    pub fn helper_function() -> i32 {
        42
    }

    pub fn another_static() -> String {
        String::from("static")
    }
}
