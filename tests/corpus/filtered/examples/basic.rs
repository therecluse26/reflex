/// Example demonstrating pattern extraction
fn main() {
    let data = "apple,banana,cherry";
    let fruits = extract_pattern(data);

    for fruit in fruits {
        println!("{}", fruit);
    }
}

fn extract_pattern(input: &str) -> Vec<String> {
    input.split(',').map(|s| s.to_string()).collect()
}
