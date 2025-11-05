#[test]
fn test_extract_with_delimiter() {
    let input = "foo,bar,baz";
    let result = extract_pattern(input);
    assert_eq!(result.len(), 3);
}

#[test]
fn test_extract_empty() {
    // TODO: Handle empty strings properly
    let result = extract_pattern("");
    assert_eq!(result.len(), 0);
}
