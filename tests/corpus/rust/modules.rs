//! Test Corpus: Rust Modules
//!
//! Expected symbols: 7 modules
//! - 3 inline modules (public_module, private_module, nested)
//! - Nested submodules (inner, deeply_nested)
//!
//! Edge cases tested:
//! - Nested modules
//! - pub(crate) visibility
//! - pub(super) visibility
//! - Re-exports

pub mod public_module {
    pub fn public_fn() {}

    pub(crate) fn crate_visible() {}

    pub(super) fn parent_visible() {}
}

mod private_module {
    pub fn still_private_outside() {}
}

pub(crate) mod crate_module {
    pub fn func() {}
}

pub mod nested {
    pub mod inner {
        pub fn inner_function() {}

        pub mod deeply_nested {
            pub fn deep_function() {}
        }
    }

    // Re-export from inner module
    pub use inner::inner_function;
}

pub mod utils {
    pub fn util_fn() {}
}

// Re-export from utils
pub use utils::util_fn;
