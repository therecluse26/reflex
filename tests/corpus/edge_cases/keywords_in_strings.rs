// Test Corpus: Keywords in Strings
//
// Expected symbols: 5 functions
// - test_keywords_in_strings
// - keywords_in_comments
// - doc_comment_examples
// - multiline_strings
// - format_strings
//
// Expected behavior:
// - When searching for "struct" with --symbols:
//   - Should find 0 structs (keywords in strings/comments don't count)
// - When searching for "fn" with --symbols:
//   - Should find 5 functions (the actual function definitions above)
//   - Should NOT count "fn" mentions in strings/comments
//
// Edge cases tested:
// - Keywords in string literals
// - Keywords in doc comments
// - Keywords in regular comments
// - Keywords in multiline strings
// - Keywords in format strings
// - Keywords in code examples within comments

// Function with keywords in string literals
fn test_keywords_in_strings() {
    // These strings contain keywords but are NOT actual definitions
    let msg = "struct Point { x: i32, y: i32 }";
    let code = "fn main() { println!(\"hello\"); }";
    let class_str = "class User { name: string; }";
    let interface_str = "interface IUser { getName(): string; }";
    let enum_str = "enum Color { Red, Green, Blue }";

    println!("{}", msg);
    println!("{}", code);
}

// Function with keywords in comments
fn keywords_in_comments() {
    // struct Foo - this is NOT a struct definition
    /* enum Bar - this is NOT an enum */
    // fn example() - this is NOT a function
    /* class Example - NOT a class */

    /*
     * Multi-line comment with keywords:
     * struct Point
     * fn calculate
     * enum Status
     */

    println!("Comments should not trigger keyword detection");
}

/// Function with keywords in doc comments
///
/// Example code in doc comments:
/// ```
/// struct Example {
///     value: i32,
/// }
///
/// fn example() {
///     let e = Example { value: 42 };
/// }
/// ```
///
/// The above keywords should NOT be counted as definitions
fn doc_comment_examples() {
    println!("Doc comments with code examples");
}

// Multiline strings with keywords
fn multiline_strings() {
    let code_block = r#"
        struct Database {
            connection: String,
        }

        fn connect() -> Database {
            Database {
                connection: "localhost".to_string(),
            }
        }

        enum Status {
            Connected,
            Disconnected,
        }
    "#;

    // None of the above keywords should trigger
    println!("{}", code_block);
}

// Format strings with keywords
fn format_strings() {
    let struct_name = "Point";
    let fn_name = "calculate";

    let message = format!(
        "struct {} was defined, and fn {} was implemented",
        struct_name, fn_name
    );

    println!("{}", message);
}

// Verify: searching for "struct" should find 0 struct definitions
// Verify: searching for "fn" should find exactly 5 function definitions
// Verify: keywords in strings/comments are properly ignored
