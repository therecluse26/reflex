fn main() {
    println!("Hello, world!");
    // TODO: Add error handling
}

fn extract_pattern(input: &str) -> Vec<String> {
    input.split(',').map(|s| s.trim().to_string()).collect()
}
