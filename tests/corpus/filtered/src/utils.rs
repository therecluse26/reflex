/// Utility functions for string manipulation
pub mod string_utils {
    pub fn extract_pattern(text: &str, delimiter: char) -> Vec<&str> {
        text.split(delimiter).collect()
    }

    pub fn format_output(data: &[String]) -> String {
        // TODO: Add formatting options
        data.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract() {
        let result = string_utils::extract_pattern("a,b,c", ',');
        assert_eq!(result, vec!["a", "b", "c"]);
    }
}
