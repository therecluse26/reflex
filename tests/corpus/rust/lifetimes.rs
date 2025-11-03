//! Test Corpus: Complex Lifetimes
//!
//! Expected symbols: 8 functions with lifetime parameters
//!
//! Edge cases tested:
//! - Multiple lifetime parameters
//! - Lifetime bounds
//! - Higher-Rank Trait Bounds (HRTB)
//! - 'static lifetimes

pub fn single_lifetime<'a>(s: &'a str) -> &'a str {
    s
}

fn two_lifetimes<'a, 'b>(s1: &'a str, s2: &'b str) -> &'a str
where
    'b: 'a,
{
    s1
}

pub fn lifetime_in_struct<'a>() -> Holder<'a> {
    Holder { data: &[] }
}

struct Holder<'a> {
    data: &'a [u8],
}

fn complex_lifetimes<'a, 'b, 'c>(
    x: &'a str,
    y: &'b str,
    z: &'c str,
) -> &'a str
where
    'b: 'a,
    'c: 'b,
{
    x
}

pub fn static_lifetime(s: &'static str) -> &'static str {
    s
}

fn elided_lifetime(s: &str) -> &str {
    s
}

// HRTB (Higher-Rank Trait Bounds)
pub fn with_hrtb<F>(f: F) -> String
where
    F: for<'a> Fn(&'a str) -> &'a str,
{
    f("test").to_string()
}

fn multiple_refs<'a>(v: &'a Vec<&'a str>) -> &'a str {
    v[0]
}
