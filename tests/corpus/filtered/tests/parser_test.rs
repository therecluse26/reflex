use corpus_filtered::Parser;

#[test]
fn test_extract_pattern() {
    let parser = Parser::new();
    // TODO: Add more test cases
    assert!(parser.extract_pattern("test").is_none());
}

#[test]
fn test_parser_initialization() {
    let parser = Parser::new();
    assert!(parser.tokens.is_empty());
}
