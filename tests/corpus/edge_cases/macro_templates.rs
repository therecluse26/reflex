// Test Corpus: Macro Templates
//
// Expected symbols: 2 macros + 1 actual struct
// - define_struct (macro)
// - define_function (macro)
// - ActualStruct (struct created by macro expansion)
//
// Expected behavior:
// - When searching for "struct" with --symbols:
//   - Should find 1 struct (ActualStruct)
//   - Should NOT count the template inside define_struct!() macro
// - Tree-sitter parses macro templates as part of macro definition,
//   not as actual struct definitions
//
// Edge cases tested:
// - Keywords in macro_rules! templates
// - Macro-generated code (define_struct!(ActualStruct))
// - Distinguishing templates from actual definitions

// Macro that CONTAINS "struct" keyword in template
// The "struct $name;" inside is a template, NOT a real struct
macro_rules! define_struct {
    ($name:ident) => {
        struct $name;  // This is a TEMPLATE, not a definition!
    };
}

// Macro that CONTAINS "fn" keyword in template
// The "fn $name()" inside is a template, NOT a real function
macro_rules! define_function {
    ($name:ident) => {
        fn $name() {  // This is a TEMPLATE, not a definition!
            println!("Generated function");
        }
    };
}

// Macro that generates multiple keywords in template
macro_rules! define_module {
    ($name:ident) => {
        mod $name {  // TEMPLATE
            struct Inner;  // TEMPLATE
            fn helper() {}  // TEMPLATE
        }
    };
}

// This ACTUALLY creates a struct (via macro expansion)
// Tree-sitter should see this as struct ActualStruct;
define_struct!(ActualStruct);

// This ACTUALLY creates a function (via macro expansion)
define_function!(ActualFunction);

// Verification:
// - "struct" keyword search should find:
//   - ActualStruct (1 struct)
// - Should NOT find:
//   - The template "struct $name;" inside define_struct!()
//   - The template "struct Inner;" inside define_module!()
//
// - "fn" keyword search should find:
//   - ActualFunction (1 function)
// - Should NOT find:
//   - The template "fn $name()" inside define_function!()
//   - The template "fn helper()" inside define_module!()

// Additional edge case: macro with complex patterns
macro_rules! complex_struct {
    (
        $vis:vis struct $name:ident {
            $($field:ident: $ty:ty),*
        }
    ) => {
        $vis struct $name {  // TEMPLATE
            $($field: $ty),*
        }
    };
}

// Usage example (this does create a real struct)
complex_struct!(
    pub struct Config {
        host: String,
        port: i32
    }
);

// Expected final count:
// - Structs: 2 (ActualStruct, Config)
// - Functions: 1 (ActualFunction)
// - Macros: 4 (define_struct, define_function, define_module, complex_struct)
