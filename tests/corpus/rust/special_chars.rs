//! Test Corpus: Special Characters and Operators
//!
//! This file tests searching for special characters and operators
//!
//! Expected: Various operator usages

pub fn logical_operators() {
    let a = true && false;
    let b = true || false;
    let c = !true;
}

fn comparison_operators() {
    let a = 1 == 2;
    let b = 1 != 2;
    let c = 1 < 2;
    let d = 1 > 2;
    let e = 1 <= 2;
    let f = 1 >= 2;
}

pub fn bitwise_operators() {
    let a = 5 & 3;
    let b = 5 | 3;
    let c = 5 ^ 3;
    let d = 5 << 1;
    let e = 5 >> 1;
}

fn arrow_operators() {
    let f = || -> i32 { 42 };
    let ptr: *const i32 = std::ptr::null();
}

pub fn path_separator() {
    let v = std::vec::Vec::<i32>::new();
    std::println!("test");
}

fn fat_arrow() {
    match 42 {
        0 => println!("zero"),
        1 => println!("one"),
        _ => println!("other"),
    }
}

pub fn question_mark() -> Result<i32, String> {
    let x = Some(42).ok_or("error")?;
    Ok(x)
}

fn ampersand_ref() {
    let x = 42;
    let r = &x;
    let mr = &mut 10;
}

pub fn dereference() {
    let x = Box::new(42);
    let y = *x;
}

fn range_operators() {
    let a = 0..10;
    let b = 0..=10;
    let c = ..10;
    let d = 10..;
    let e = ..;
}
