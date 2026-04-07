use crate::gs::semantic_tokens;
use gs_ast::gs::process::process_gs_ast;
use gs_parser::gs::grammar::{GameScriptParser, Rule};
use pest::Parser;

#[test]
fn test_literal_tokens() {
    let src = "class Test { void Main() { bool b = true; bool f = false; object n = null; char c = 'a'; float fl = 1.0f; hex h = 0x123; } };";
    let pairs = GameScriptParser::parse(Rule::program, src).unwrap();
    let program = process_gs_ast(pairs, src);
    let tokens = crate::process_raw_tokens(semantic_tokens(&program));

    // true - length 4, token_type 0 (KEYWORD)
    // false - length 5, token_type 0 (KEYWORD)
    // null - length 4, token_type 0 (KEYWORD)
    // 'a' - length 3, token_type 2 (STRING)
    // 1.0f - length 4, token_type 6 (NUMBER)
    // 0x123 - length 5, token_type 6 (NUMBER)

    let true_token = tokens.iter().find(|t| t.length == 4 && t.token_type == 0);
    assert!(true_token.is_some(), "Should find 'true' keyword token");

    let false_token = tokens.iter().find(|t| t.length == 5 && t.token_type == 0);
    assert!(false_token.is_some(), "Should find 'false' keyword token");

    let _null_token = tokens.iter().find(|t| t.length == 4 && t.token_type == 0);
    // Note: both 'true' and 'null' are length 4 and type 0.
    let type0_len4_tokens: Vec<_> = tokens
        .iter()
        .filter(|t| t.length == 4 && t.token_type == 0)
        .collect();
    assert_eq!(
        type0_len4_tokens.len(),
        2,
        "Should find 'true' and 'null' keyword tokens"
    );

    let char_token = tokens.iter().find(|t| t.length == 3 && t.token_type == 2);
    assert!(char_token.is_some(), "Should find char literal token");

    let float_token = tokens.iter().find(|t| t.length == 4 && t.token_type == 6);
    assert!(float_token.is_some(), "Should find float literal token");

    let hex_token = tokens.iter().find(|t| t.length == 5 && t.token_type == 6);
    assert!(hex_token.is_some(), "Should find hex literal token");
}

#[test]
fn test_string_literal_tokens() {
    let src = "class Test { void Main() { string s = \"hello\"; } };";
    let pairs = GameScriptParser::parse(Rule::program, src).unwrap();
    let program = process_gs_ast(pairs, src);
    let tokens = crate::process_raw_tokens(semantic_tokens(&program));

    // \"hello\" - length 7, token_type 2 (STRING)
    let string_token = tokens.iter().find(|t| t.length == 7 && t.token_type == 2);
    assert!(
        string_token.is_some(),
        "Should find string literal token. Tokens: {:?}",
        tokens
    );
}

#[test]
fn test_field_literal_tokens() {
    let src = "class Test { bool b = true; string s = \"hello\"; float f = 1.0f; int i = 123; };";
    let pairs = GameScriptParser::parse(Rule::program, src).unwrap();
    let program = process_gs_ast(pairs, src);
    let tokens = crate::process_raw_tokens(semantic_tokens(&program));

    let true_token = tokens.iter().find(|t| t.length == 4 && t.token_type == 0);
    assert!(
        true_token.is_some(),
        "Should find 'true' keyword token for field"
    );

    let string_token = tokens.iter().find(|t| t.length == 7 && t.token_type == 2);
    assert!(
        string_token.is_some(),
        "Should find string literal token for field"
    );

    let float_token = tokens.iter().find(|t| t.length == 4 && t.token_type == 6);
    assert!(
        float_token.is_some(),
        "Should find float literal token for field"
    );

    let int_token = tokens.iter().find(|t| t.length == 3 && t.token_type == 6);
    assert!(
        int_token.is_some(),
        "Should find int literal token for field"
    );
}

#[test]
fn test_multi_digit_number_tokens() {
    let src = "class Test { void Main() { int i = 12345; float f = 123.45f; } };";
    let pairs = GameScriptParser::parse(Rule::program, src).unwrap();
    let program = process_gs_ast(pairs, src);
    let tokens = crate::process_raw_tokens(semantic_tokens(&program));

    // 12345 - length 5, token_type 6 (NUMBER)
    let int_token = tokens.iter().find(|t| t.length == 5 && t.token_type == 6);
    assert!(
        int_token.is_some(),
        "Should find 5-digit int literal token. Tokens: {:?}",
        tokens
    );

    // 123.45f - length 7, token_type 6 (NUMBER)
    let float_token = tokens.iter().find(|t| t.length == 7 && t.token_type == 6);
    assert!(
        float_token.is_some(),
        "Should find 7-character float literal token. Tokens: {:?}",
        tokens
    );
}

#[test]
fn test_str_tokens() {
    let source = "class X { public void Test(void) { Str.Tokens(pid, \"_\"); } };";
    let program = gs_parser::gs::parse(source).unwrap();
    let ast = gs_ast::gs::process::process_gs_ast(program, source);

    let mut tokens = Vec::new();
    let known_classes = std::collections::HashSet::new();
    for class in &ast.classes {
        for method in &class.methods {
            for stmt in &method.body.statements {
                crate::gs::stmt::collect_stmt_tokens(stmt, &mut tokens, &known_classes);
            }
        }
    }
    println!("{:#?}", tokens);
}

#[test]
fn test_method_call_tokens() {
    let source = "class X { public void Test(void) { me.GetIsDistant(); } };";
    let program = gs_parser::gs::parse(source).unwrap();
    let ast = gs_ast::gs::process::process_gs_ast(program, source);

    let raw_tokens = crate::gs::semantic_tokens(&ast);
    let method_token = raw_tokens.iter().find(|(range, token_type, _)| {
        range.end.character - range.start.character == 12
            && *token_type == tower_lsp_server::ls_types::SemanticTokenType::METHOD
    });
    assert!(
        method_token.is_some(),
        "Should find GetIsDistant as METHOD token"
    );
}

#[test]
fn test_link_prop_tokens() {
    let src = "class Test { public void LinkPropertyValue(string pid) { inherited(pid); } };";
    let pairs =
        gs_parser::gs::grammar::GameScriptParser::parse(gs_parser::gs::grammar::Rule::program, src)
            .unwrap();
    let program = gs_ast::gs::process::process_gs_ast(pairs, src);
    let tokens = crate::process_raw_tokens(crate::gs::semantic_tokens(&program));
    println!("{:#?}", program.classes[0].methods[0]);
    println!("{:#?}", tokens);
}
