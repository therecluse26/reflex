// Test Corpus: Partial Keyword Matches
//
// Expected symbols: 13 functions + 5 structs
// Functions:
// - struct_builder, destructure, restructure
// - fn_pointer, define_macro
// - interface_impl, class_factory
// - enum_parser, const_value
// - is_function, to_struct, from_enum
// - helper_fn
// Structs:
// - StructBuilder, StructWrapper, InnerStruct
// - FnPointer, ConstValue
//
// Expected behavior:
// - When searching for "struct" with --symbols:
//   - Should trigger keyword mode
//   - Should find ALL structs (5 total)
//   - Should NOT find functions that have "struct" in their name
// - When searching for "fn" with --symbols:
//   - Should trigger keyword mode
//   - Should find ALL functions (13 total)
//   - Should NOT find struct names containing "fn"
//
// Edge cases tested:
// - Function names containing keywords
// - Struct names containing keywords
// - Keywords as substrings in identifiers
// - Underscored combinations (struct_builder, fn_pointer)
// - Camel case combinations (StructBuilder, FnPointer)

// Functions with "struct" in their names
fn struct_builder() -> StructBuilder {
    StructBuilder { name: "test".to_string() }
}

fn destructure(value: (i32, i32)) -> i32 {
    let (a, b) = value;
    a + b
}

fn restructure(data: Vec<i32>) -> Vec<i32> {
    data.into_iter().rev().collect()
}

// Structs with "struct" in their names
struct StructBuilder {
    name: String,
}

struct StructWrapper<T> {
    inner: T,
}

struct InnerStruct {
    value: i32,
}

// Functions with "fn" in their names
fn fn_pointer() -> fn() {
    || println!("closure")
}

fn define_macro() {
    println!("defining");
}

fn helper_fn() {
    println!("helper");
}

// Struct with "fn" in name
struct FnPointer {
    ptr: fn(),
}

// Functions with other keywords
fn interface_impl() {
    println!("implementing interface");
}

fn class_factory() -> String {
    "factory".to_string()
}

fn enum_parser() {
    println!("parsing enum");
}

fn const_value() -> i32 {
    42
}

// Struct with other keywords
struct ConstValue {
    value: i32,
}

// More examples with keywords as substrings
fn is_function() -> bool {
    true
}

fn to_struct(value: i32) -> InnerStruct {
    InnerStruct { value }
}

fn from_enum(value: i32) -> i32 {
    value
}

// Test case verification:
//
// Query: "struct" with --symbols
// Expected behavior:
// - Triggers keyword mode (scans all Rust files)
// - Auto-infers --kind struct
// - Finds ONLY struct definitions: 5 structs
//   - StructBuilder
//   - StructWrapper
//   - InnerStruct
//   - FnPointer
//   - ConstValue
// - Does NOT find functions containing "struct":
//   - struct_builder ❌
//   - destructure ❌
//   - restructure ❌
//   - to_struct ❌
//
// Query: "fn" with --symbols
// Expected behavior:
// - Triggers keyword mode
// - Auto-infers --kind function
// - Finds ONLY function definitions: 13 functions
//   - All functions listed above
// - Does NOT find structs containing "fn":
//   - FnPointer ❌
//
// Query: "struct_builder" (NOT a keyword)
// Expected behavior:
// - Does NOT trigger keyword mode
// - Uses trigram/full-text search
// - Finds both the function and the struct

// Verify that keyword detection is exact match only:
// - "struct" → keyword mode (finds 5 structs)
// - "fn" → keyword mode (finds 13 functions)
// - "struct_builder" → NOT keyword mode (full-text search)
// - "destructure" → NOT keyword mode (full-text search)
