//! Test Corpus: Iterator Patterns
//!
//! Expected symbols: 12+ functions using iterators
//!
//! Real-world patterns tested:
//! - map, filter, fold, collect
//! - Iterator chains
//! - Custom iterators

pub fn map_example() -> Vec<i32> {
    vec![1, 2, 3].iter().map(|x| x * 2).collect()
}

fn filter_example() -> Vec<i32> {
    vec![1, 2, 3, 4, 5]
        .into_iter()
        .filter(|x| x % 2 == 0)
        .collect()
}

pub fn fold_example() -> i32 {
    vec![1, 2, 3, 4].iter().fold(0, |acc, x| acc + x)
}

fn chain_example() -> Vec<i32> {
    vec![1, 2, 3]
        .into_iter()
        .chain(vec![4, 5, 6])
        .collect()
}

pub fn flat_map_example() -> Vec<i32> {
    vec![vec![1, 2], vec![3, 4]]
        .into_iter()
        .flat_map(|x| x)
        .collect()
}

fn zip_example() -> Vec<(i32, i32)> {
    vec![1, 2, 3].into_iter().zip(vec![4, 5, 6]).collect()
}

pub fn take_skip_example() -> Vec<i32> {
    vec![1, 2, 3, 4, 5]
        .into_iter()
        .skip(1)
        .take(3)
        .collect()
}

fn enumerate_example() {
    for (i, val) in vec![10, 20, 30].iter().enumerate() {
        println!("{}: {}", i, val);
    }
}

pub fn find_example() -> Option<i32> {
    vec![1, 2, 3, 4, 5].into_iter().find(|x| x % 2 == 0)
}

fn any_all_example() -> bool {
    let any_even = vec![1, 2, 3].iter().any(|x| x % 2 == 0);
    let all_positive = vec![1, 2, 3].iter().all(|x| *x > 0);
    any_even && all_positive
}

pub fn partition_example() -> (Vec<i32>, Vec<i32>) {
    vec![1, 2, 3, 4, 5]
        .into_iter()
        .partition(|x| x % 2 == 0)
}

fn complex_chain() -> Vec<String> {
    vec![1, 2, 3, 4, 5]
        .into_iter()
        .filter(|x| x % 2 == 0)
        .map(|x| x * 2)
        .filter(|x| x > 4)
        .map(|x| x.to_string())
        .collect()
}
