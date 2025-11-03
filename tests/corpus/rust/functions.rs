//! Test Corpus: Rust Functions
//!
//! Expected symbols: 14 functions
//! - 3 public functions (public_function, pub_async_function, pub_const_function)
//! - 5 private functions (private_function, async_function, generic_function, function_with_where, unsafe_function)
//! - 2 const functions (pub_const_function, const_function)
//! - 3 async functions (pub_async_function, async_function, async_generic)
//! - 1 unsafe function (unsafe_function)
//! - 3 generic functions (generic_function, function_with_where, async_generic)
//!
//! Edge cases tested:
//! - Various visibility modifiers
//! - Generic type parameters
//! - Where clauses
//! - Async/await patterns
//! - Unsafe functions

pub fn public_function() {
    println!("Public function");
}

fn private_function() {
    println!("Private function");
}

pub async fn pub_async_function() -> Result<(), std::io::Error> {
    Ok(())
}

async fn async_function() -> Result<String, String> {
    Ok("async result".to_string())
}

fn generic_function<T: Clone>(value: T) -> T {
    value.clone()
}

fn function_with_where<T>(value: T) -> T
where
    T: Clone + std::fmt::Debug,
{
    println!("{:?}", value);
    value
}

pub const fn pub_const_function() -> i32 {
    42
}

const fn const_function() -> u32 {
    100
}

unsafe fn unsafe_function() {
    // Unsafe operations here
}

async fn async_generic<T: Send>(value: T) -> Result<T, ()> {
    Ok(value)
}

pub fn function_with_many_params(
    param1: String,
    param2: i32,
    param3: bool,
    param4: Vec<u8>,
    param5: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

fn function_with_lifetimes<'a, 'b>(s1: &'a str, s2: &'b str) -> &'a str
where
    'b: 'a,
{
    s1
}

pub fn function_returning_impl() -> impl Iterator<Item = i32> {
    vec![1, 2, 3].into_iter()
}

fn function_with_default_params<T: Default>() -> T {
    T::default()
}
