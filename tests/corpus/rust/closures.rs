//! Test Corpus: Closure Patterns
//!
//! Expected symbols: 10+ functions using closures
//!
//! Real-world patterns tested:
//! - Various closure syntaxes
//! - Fn, FnMut, FnOnce traits
//! - Closure captures

pub fn simple_closure() {
    let add = |x, y| x + y;
    let result = add(1, 2);
}

fn closure_with_types() {
    let multiply: fn(i32, i32) -> i32 = |x, y| x * y;
    let result = multiply(3, 4);
}

pub fn closure_captures() {
    let x = 42;
    let get_x = || x;
    let result = get_x();
}

fn closure_mut_capture() {
    let mut x = 0;
    let mut increment = || {
        x += 1;
        x
    };
    increment();
}

pub fn closure_move() {
    let s = String::from("hello");
    let take_string = move || {
        println!("{}", s);
    };
    take_string();
}

fn higher_order_fn<F>(f: F) -> i32
where
    F: Fn(i32) -> i32,
{
    f(42)
}

pub fn use_higher_order() -> i32 {
    higher_order_fn(|x| x * 2)
}

fn returns_closure() -> impl Fn(i32) -> i32 {
    |x| x + 1
}

pub fn closure_in_iterator() -> Vec<i32> {
    vec![1, 2, 3].into_iter().map(|x| x * 2).collect()
}

fn multi_line_closure() {
    let complex = |x: i32| -> i32 {
        let doubled = x * 2;
        let squared = doubled * doubled;
        squared
    };
    complex(5);
}
