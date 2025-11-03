//! Test Corpus: Rust Macros
//!
//! This file tests macro definitions and usage patterns.
//!
//! Expected symbols: 5 macro definitions
//!
//! Edge cases tested:
//! - macro_rules! definitions
//! - Macro invocations
//! - Nested macros
//! - Proc macro attributes

#[macro_export]
macro_rules! say_hello {
    () => {
        println!("Hello!");
    };
    ($name:expr) => {
        println!("Hello, {}!", $name);
    };
}

macro_rules! create_function {
    ($func_name:ident) => {
        fn $func_name() {
            println!("Function {} called", stringify!($func_name));
        }
    };
}

pub macro_rules! vec_strs {
    ($($x:expr),*) => {
        vec![$(String::from($x)),*]
    };
}

macro_rules! calculate {
    (eval $e:expr) => {
        {
            let result: i32 = $e;
            result
        }
    };
}

macro_rules! complex_macro {
    ($x:expr, $y:expr, { $($z:tt)* }) => {
        {
            let a = $x;
            let b = $y;
            $($z)*
            a + b
        }
    };
}

// Macro usage examples
fn test_macros() {
    say_hello!();
    say_hello!("World");

    create_function!(my_function);

    let v = vec_strs!["a", "b", "c"];

    let result = calculate!(eval 5 + 5);

    let sum = complex_macro!(1, 2, {
        let c = 3;
    });
}

// Function that uses many macros
pub fn macro_heavy() {
    println!("Testing");
    assert_eq!(1, 1);
    vec![1, 2, 3];
    format!("Hello {}", "world");
    panic!("Error");
}
