//! Test Corpus: Rust Traits
//!
//! Expected symbols: 8 traits
//! - 2 simple traits (Drawable, Serializable)
//! - 2 traits with associated types (Iterator2, Container2)
//! - 2 traits with default implementations (Logger, Validator)
//! - 1 trait with generic parameters (Converter)
//! - 1 trait with supertraits (AdvancedDrawable)
//!
//! Edge cases tested:
//! - Associated types
//! - Default implementations
//! - Generic parameters
//! - Trait bounds
//! - Supertraits

pub trait Drawable {
    fn draw(&self);
}

trait Serializable {
    fn serialize(&self) -> String;
    fn deserialize(data: &str) -> Self;
}

// Trait with associated types
pub trait Iterator2 {
    type Item;
    fn next(&mut self) -> Option<Self::Item>;
}

trait Container2 {
    type Item;
    type Error;

    fn get(&self, index: usize) -> Result<&Self::Item, Self::Error>;
}

// Trait with default implementation
pub trait Logger {
    fn log(&self, message: &str) {
        println!("LOG: {}", message);
    }

    fn error(&self, message: &str) {
        eprintln!("ERROR: {}", message);
    }
}

trait Validator {
    fn validate(&self) -> bool {
        true
    }
}

// Generic trait
pub trait Converter<T> {
    fn convert(&self) -> T;
}

// Trait with supertrait
pub trait AdvancedDrawable: Drawable + Clone {
    fn draw_advanced(&self);
}
