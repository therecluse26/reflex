//! Test Corpus: Complex Generic Patterns
//!
//! Expected symbols: 10+ functions/structs with complex generics
//!
//! Edge cases tested:
//! - Multiple type parameters
//! - Nested generic bounds
//! - Associated types in bounds
//! - Complex where clauses

pub fn multi_param<T, U, V>(t: T, u: U, v: V) -> (T, U, V) {
    (t, u, v)
}

fn with_bounds<T: Clone + std::fmt::Debug, U: Default>(t: T, u: U) -> T {
    println!("{:?}", t);
    t
}

pub fn complex_where<T, U>(t: T, u: U) -> T
where
    T: Clone + std::fmt::Debug + PartialEq,
    U: Default + Into<String>,
{
    t
}

struct GenericContainer<T, U, V>
where
    T: Clone,
    U: Default,
    V: std::fmt::Display,
{
    t: T,
    u: U,
    v: V,
}

pub fn nested_generics<T: Iterator<Item = U>, U: Clone>(iter: T) -> Vec<U> {
    iter.map(|x| x.clone()).collect()
}

fn with_associated_type<T>(t: T) -> T::Item
where
    T: Iterator,
    T::Item: Default,
{
    T::Item::default()
}

pub fn multiple_trait_bounds<T>(t: T) -> String
where
    T: std::fmt::Debug + std::fmt::Display + Clone + Default,
{
    format!("{} {:?}", t, t)
}

fn impl_trait_param(t: impl Clone + std::fmt::Debug) -> String {
    format!("{:?}", t)
}

pub fn impl_trait_return() -> impl Iterator<Item = i32> {
    vec![1, 2, 3].into_iter()
}

fn higher_order<F, T, U>(f: F, t: T) -> U
where
    F: Fn(T) -> U,
    T: Clone,
{
    f(t.clone())
}
