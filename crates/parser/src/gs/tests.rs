use crate::gs::grammar::{GameScriptParser, Rule};
use pest::Parser;

#[test]
fn test_identifier_fails_on_keyword() {
    // "default" is a keyword, so it should NOT be parsed as an identifier
    let result = GameScriptParser::parse(Rule::identifier, "default");
    assert!(
        result.is_err(),
        "Expected 'default' to be rejected as an identifier, but it was accepted."
    );
}

#[test]
fn test_identifier_accepts_valid_identifier() {
    // "default_default" is not a keyword, so it should be parsed as an identifier
    let result = GameScriptParser::parse(Rule::identifier, "default_default");
    assert!(
        result.is_ok(),
        "Expected 'default_default' to be accepted as an identifier, but it was rejected."
    );
}
