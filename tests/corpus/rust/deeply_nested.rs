//! Test Corpus: Deeply Nested Structures
//!
//! Expected symbols: nested modules, functions, and scopes
//!
//! Edge cases tested:
//! - Deep module nesting (10+ levels)
//! - Nested impl blocks
//! - Nested closures
//! - Deep scope nesting

pub mod level1 {
    pub mod level2 {
        pub mod level3 {
            pub mod level4 {
                pub mod level5 {
                    pub mod level6 {
                        pub mod level7 {
                            pub mod level8 {
                                pub mod level9 {
                                    pub mod level10 {
                                        pub fn deeply_nested_function() {
                                            println!("Very deep");
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn deeply_nested_scopes() {
    if true {
        if true {
            if true {
                if true {
                    if true {
                        if true {
                            if true {
                                if true {
                                    if true {
                                        if true {
                                            println!("10 levels deep");
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn nested_closures() {
    let f1 = || {
        let f2 = || {
            let f3 = || {
                let f4 = || {
                    let f5 = || {
                        println!("Deeply nested closure");
                    };
                    f5();
                };
                f4();
            };
            f3();
        };
        f2();
    };
    f1();
}

pub fn nested_match() {
    let x = Some(Some(Some(Some(Some(42)))));
    match x {
        Some(a) => match a {
            Some(b) => match b {
                Some(c) => match c {
                    Some(d) => match d {
                        Some(e) => println!("{}", e),
                        None => println!("None at level 5"),
                    },
                    None => println!("None at level 4"),
                },
                None => println!("None at level 3"),
            },
            None => println!("None at level 2"),
        },
        None => println!("None at level 1"),
    }
}
