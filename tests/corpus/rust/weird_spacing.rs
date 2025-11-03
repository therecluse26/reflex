//! Test Corpus: Unusual Whitespace and Formatting
//!
//! Expected symbols: 10 functions
//!
//! Edge cases tested:
//! - Multiple blank lines
//! - Unusual indentation
//! - Tabs vs spaces
//! - Trailing whitespace
//! - No spaces around operators

pub fn normal_spacing() {
    println!("Normal");
}



fn multiple_blank_lines_before() {
    println!("Blank lines");
}


pub    fn    extra_spaces() {
    println!("Extra");
}

fn	tab_indented() {
	println!("Tabs");
}

pub fn no_spaces_in_params(x:i32,y:i32)->i32{x+y}

fn trailing_spaces_exist() {
    let x = 42;
    println!("{}", x);
}

pub fn mixed_tabs_and_spaces() {
	let x = 1;
    let y = 2;
	let z = 3;
    println!("{} {} {}", x, y, z);
}

fn
multiline_signature(
x:i32,
y:i32
)->i32{
x+y
}

pub fn cramped(){let x=1;let y=2;x+y}

fn wide_signature(
    parameter_one: String,
    parameter_two: i32,
    parameter_three: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    Ok(format!("{} {}", parameter_one, parameter_two))
}
