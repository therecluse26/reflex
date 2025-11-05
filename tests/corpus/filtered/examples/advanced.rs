/// Advanced example with error handling
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // TODO: Add command-line argument parsing
    let pattern = "test_extract";
    process_pattern(pattern)?;
    Ok(())
}

fn process_pattern(pattern: &str) -> Result<(), Box<dyn Error>> {
    println!("Processing: {}", pattern);
    Ok(())
}
