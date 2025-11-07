// Test Corpus: Keywords with Mixed Case
//
// Expected symbols: 6 symbols (3 lowercase keyword-triggered + 3 regular identifiers)
// - 1 struct (lowercase_struct)
// - 1 function (lowercase_function)
// - 1 enum (lowercase_enum)
// - 1 struct (UPPERCASE_STRUCT) - NOT keyword triggered
// - 1 function (MixedCaseFunction) - NOT keyword triggered
// - 1 struct (StructBuilder) - NOT keyword triggered (partial match)
//
// Expected behavior:
// - Keywords are LOWERCASE in all languages (struct, fn, class, etc.)
// - Searching for "struct" (lowercase) should trigger keyword mode
// - Searching for "STRUCT" (uppercase) should NOT trigger keyword mode
// - Searching for "Struct" (mixed case) should NOT trigger keyword mode
//
// Edge cases tested:
// - Lowercase keywords (should trigger keyword detection)
// - Uppercase identifiers (should NOT trigger)
// - Mixed case identifiers (should NOT trigger)
// - Partial keyword matches in identifiers

// LOWERCASE KEYWORDS - Should trigger keyword mode when searched

struct lowercase_struct {
    value: i32,
}

fn lowercase_function() {
    println!("lowercase");
}

enum lowercase_enum {
    Variant1,
    Variant2,
}

// UPPERCASE/MIXED CASE - These are identifiers, NOT keywords
// Searching for "STRUCT" should NOT trigger keyword mode
// Searching for "Struct" should NOT trigger keyword mode

struct UPPERCASE_STRUCT {
    value: i32,
}

fn MixedCaseFunction() {
    println!("mixed case");
}

// Identifiers that CONTAIN keywords but aren't keywords themselves
struct StructBuilder {
    name: String,
}

// More examples
fn fn_pointer() -> fn() {
    || println!("closure")
}

struct StructWrapper<T> {
    inner: T,
}

// Test case verification:
//
// Query: "struct" with --symbols (lowercase)
// Expected: Should trigger keyword mode
// Results: 4 structs (lowercase_struct, UPPERCASE_STRUCT, StructBuilder, StructWrapper)
//
// Query: "STRUCT" with --symbols (uppercase)
// Expected: Should NOT trigger keyword mode (would use trigram search)
// Results: Depends on trigram matches, not keyword mode
//
// Query: "Struct" with --symbols (mixed case)
// Expected: Should NOT trigger keyword mode
// Results: Depends on trigram matches
//
// Query: "fn" with --symbols (lowercase)
// Expected: Should trigger keyword mode
// Results: 3 functions (lowercase_function, MixedCaseFunction, fn_pointer)
//
// Query: "FN" with --symbols (uppercase)
// Expected: Should NOT trigger keyword mode

// Additional examples with trait/impl (also lowercase keywords)
trait lowercase_trait {
    fn method(&self);
}

impl lowercase_trait for lowercase_struct {
    fn method(&self) {
        println!("impl");
    }
}

// Verify case-sensitive keyword detection:
// - "struct", "fn", "enum", "trait", "impl" → trigger keyword mode
// - "Struct", "Fn", "Enum", "Trait", "Impl" → do NOT trigger
// - "STRUCT", "FN", "ENUM", "TRAIT", "IMPL" → do NOT trigger
