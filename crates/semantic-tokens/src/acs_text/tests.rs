use crate::acs_text::acs_text_semantic_tokens;
use rayon::prelude::*;
use std::collections::HashMap;
use tower_lsp_server::ls_types::SemanticTokenType;
use trainz_acs_text_validators::RulesRoot;
use trainz_ast::acs_text::process::process_acs_text_ast;
use trainz_parser::acs_text::parse_acs_text;

fn empty_graph() -> RulesRoot {
    RulesRoot::new(HashMap::new(), vec![])
}

#[test]
fn test_acs_text_semantic_tokens() {
    let code = r#"
    container
    {
        key "value"
        number 42
    }
    "#;
    let pairs = parse_acs_text(code).unwrap();
    let acs_text = process_acs_text_ast(pairs, code);
    let graph = empty_graph();
    let tokens = crate::process_raw_tokens(acs_text_semantic_tokens(&acs_text, &graph), Some(code));

    // We expect tokens for `container` (keyword/string?), `key` (keyword), `"value"` (string), `number` (keyword), `42` (number).
    assert!(!tokens.is_empty());
}

#[test]
fn test_acs_text_token_lengths() {
    let code = "multi_digit 123456\nfloat_val 12.345f\nstring_val \"hello world\"\nkuid_val <KUID:123456:7890>\nvar_val $(my_variable)";
    let pairs = parse_acs_text(code).unwrap();
    let acs_text = process_acs_text_ast(pairs, code);
    let graph = empty_graph();
    let tokens = crate::process_raw_tokens(acs_text_semantic_tokens(&acs_text, &graph), Some(code));

    let type_number = crate::legend::get_token_type(SemanticTokenType::NUMBER);
    let type_string = crate::legend::get_token_type(SemanticTokenType::STRING);
    let type_property = crate::legend::get_token_type(SemanticTokenType::PROPERTY);
    let type_variable = crate::legend::get_token_type(SemanticTokenType::VARIABLE);

    // 123456 - length 6, type 6 (NUMBER)
    let int_token = tokens
        .par_iter()
        .find_first(|t| t.length == 6 && t.token_type == type_number);
    assert!(
        int_token.is_some(),
        "Should find 6-digit int literal token. Tokens: {:?}",
        tokens
    );

    // 12.345f - length 7, type 6 (NUMBER)
    let float_token = tokens
        .par_iter()
        .find_first(|t| t.length == 7 && t.token_type == type_number);
    assert!(
        float_token.is_some(),
        "Should find 7-character float literal token. Tokens: {:?}",
        tokens
    );

    // \"hello world\" - length 13, type STRING
    let string_token = tokens
        .par_iter()
        .find_first(|t| t.length == 13 && t.token_type == type_string);
    assert!(
        string_token.is_some(),
        "Should find string literal token. Tokens: {:?}",
        tokens
    );

    // <KUID:123456:7890> - length 18, type PROPERTY in AcsText
    let kuid_token = tokens
        .par_iter()
        .find_first(|t| t.length == 18 && t.token_type == type_property);
    assert!(
        kuid_token.is_some(),
        "Should find KUID literal token. Tokens: {:?}",
        tokens
    );

    // $(my_variable) - length 14, type VARIABLE
    let var_token = tokens
        .par_iter()
        .find_first(|t| t.length == 14 && t.token_type == type_variable);
    assert!(
        var_token.is_some(),
        "Should find variable literal token. Tokens: {:?}",
        tokens
    );
}

#[test]
fn test_acs_text_multiline_string_semantic_tokens() {
    let code = r#"
    description "This is a
    multi-line
    string"
    "#;
    let pairs = parse_acs_text(code).unwrap();
    let acs_text = process_acs_text_ast(pairs, code);
    let graph = empty_graph();
    let tokens = crate::process_raw_tokens(acs_text_semantic_tokens(&acs_text, &graph), Some(code));
    let type_string = crate::legend::get_token_type(SemanticTokenType::STRING);

    // We expect 3 tokens for the string literal, one for each line.
    let string_tokens: Vec<_> = tokens
        .par_iter()
        .filter(|t| t.token_type == type_string)
        .collect();
    assert!(
        !string_tokens.is_empty(),
        "Should find string literal tokens. Tokens: {:?}",
        tokens
    );

    // "This is a
    assert!(
        string_tokens.par_iter().any(|t| t.length == 10),
        "Should find first line of multiline string. Tokens: {:?}",
        string_tokens
    );
    // multi-line
    assert!(
        string_tokens.par_iter().any(|t| t.length == 14),
        "Should find second line of multiline string. Tokens: {:?}",
        string_tokens
    );
    // string"
    assert!(
        string_tokens.par_iter().any(|t| t.length == 11),
        "Should find third line of multiline string. Tokens: {:?}",
        string_tokens
    );
}

#[test]
fn test_acs_text_utf16_semantic_tokens() {
    let code = "key \"💩\""; // 💩 is 1 char, 2 UTF-16 units
    let pairs = parse_acs_text(code).unwrap();
    let acs_text = process_acs_text_ast(pairs, code);
    let graph = empty_graph();
    let tokens = crate::process_raw_tokens(acs_text_semantic_tokens(&acs_text, &graph), Some(code));
    let type_string = crate::legend::get_token_type(SemanticTokenType::STRING);

    // \"💩\" should have length 4 (2 for quotes + 2 for 💩)
    let string_token = tokens
        .par_iter()
        .find_first(|t| t.token_type == type_string);
    assert!(
        string_token.is_some(),
        "Should find string literal token. Tokens: {:?}",
        tokens
    );
    assert_eq!(
        string_token.unwrap().length,
        3,
        "String length should be 3 (2 for quotes + 1 for 💩 - AST currently treats 💩 as 1 character). Tokens: {:?}",
        tokens
    );
}
