use crate::gs::semantic_tokens;
use pest::Parser;
use rayon::prelude::*;
use trainz_ast::gs::process::process_trainz_ast;
use trainz_parser::gs::grammar::{GameScriptParser, Rule};

fn get_token_type(target: tower_lsp_server::ls_types::SemanticTokenType) -> u32 {
    let (types, _) = crate::legend::get_legend();
    types.par_iter().position_first(|t| *t == target).unwrap() as u32
}

fn get_tokens_for_src(src: &str) -> Vec<tower_lsp_server::ls_types::SemanticToken> {
    let pairs = GameScriptParser::parse(Rule::program, src).unwrap();
    let program = process_trainz_ast(pairs, src);
    crate::process_raw_tokens(semantic_tokens(&program))
}

#[test]
fn test_keyword_literal_tokens() {
    let src = "class Test { void Main() { bool b = true; bool f = false; object n = null; } };";
    let tokens = get_tokens_for_src(src);

    let type_keyword = get_token_type(tower_lsp_server::ls_types::SemanticTokenType::KEYWORD);

    let true_token = tokens
        .par_iter()
        .find_first(|t| t.token_type == type_keyword && t.length == 4);
    assert!(true_token.is_some(), "Should find 'true' keyword token");

    let false_token = tokens
        .par_iter()
        .find_first(|t| t.token_type == type_keyword && t.length == 5);
    assert!(false_token.is_some(), "Should find 'false' keyword token");

    let type_keyword_len4_tokens: Vec<_> = tokens
        .par_iter()
        .filter(|t| t.token_type == type_keyword && t.length == 4)
        .collect();
    assert!(
        type_keyword_len4_tokens.len() >= 1,
        "Should find 'true' or 'null' as keyword tokens"
    );
}

#[test]
fn test_char_literal_token() {
    let src = "class Test { void Main() { char c = 'a'; } };";
    let tokens = get_tokens_for_src(src);

    let type_string = get_token_type(tower_lsp_server::ls_types::SemanticTokenType::STRING);
    let char_token = tokens
        .par_iter()
        .find_first(|t| t.token_type == type_string && t.length == 3);
    assert!(char_token.is_some(), "Should find char literal token");
}

#[test]
fn test_float_and_hex_literal_tokens() {
    let src = "class Test { void Main() { float fl = 1.0f; hex h = 0x123; } };";
    let tokens = get_tokens_for_src(src);

    let type_number = get_token_type(tower_lsp_server::ls_types::SemanticTokenType::NUMBER);

    let float_token = tokens
        .par_iter()
        .find_first(|t| t.token_type == type_number && t.length == 4);
    assert!(float_token.is_some(), "Should find float literal token");

    let hex_token = tokens
        .par_iter()
        .find_first(|t| t.token_type == type_number && t.length == 5);
    assert!(hex_token.is_some(), "Should find hex literal token");
}

#[test]
fn test_string_literal_tokens() {
    let src = "class Test { void Main() { string s = \"hello\"; } };";
    let pairs = GameScriptParser::parse(Rule::program, src).unwrap();
    let program = process_trainz_ast(pairs, src);
    let tokens = crate::process_raw_tokens(semantic_tokens(&program));

    // \"hello\" - length 7, token_type STRING
    let type_string = get_token_type(tower_lsp_server::ls_types::SemanticTokenType::STRING);
    let string_token = tokens
        .par_iter()
        .find_first(|t| t.length == 7 && t.token_type == type_string);
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
    let program = process_trainz_ast(pairs, src);
    let tokens = crate::process_raw_tokens(semantic_tokens(&program));

    let type_keyword = get_token_type(tower_lsp_server::ls_types::SemanticTokenType::KEYWORD);
    let type_string = get_token_type(tower_lsp_server::ls_types::SemanticTokenType::STRING);
    let type_number = get_token_type(tower_lsp_server::ls_types::SemanticTokenType::NUMBER);

    let true_token = tokens
        .par_iter()
        .find_first(|t| t.length == 4 && t.token_type == type_keyword);
    assert!(
        true_token.is_some(),
        "Should find 'true' keyword token for field"
    );

    let string_token = tokens
        .par_iter()
        .find_first(|t| t.length == 7 && t.token_type == type_string);
    assert!(
        string_token.is_some(),
        "Should find string literal token for field"
    );

    let float_token = tokens
        .par_iter()
        .find_first(|t| t.length == 4 && t.token_type == type_number);
    assert!(
        float_token.is_some(),
        "Should find float literal token for field"
    );

    let int_token = tokens
        .par_iter()
        .find_first(|t| t.length == 3 && t.token_type == type_number);
    assert!(
        int_token.is_some(),
        "Should find int literal token for field"
    );
}

#[test]
fn test_multi_digit_number_tokens() {
    let src = "class Test { void Main() { int i = 12345; float f = 123.45f; } };";
    let pairs = GameScriptParser::parse(Rule::program, src).unwrap();
    let program = process_trainz_ast(pairs, src);
    let tokens = crate::process_raw_tokens(semantic_tokens(&program));

    // 12345 - length 5, token_type NUMBER
    let type_number = get_token_type(tower_lsp_server::ls_types::SemanticTokenType::NUMBER);
    let int_token = tokens
        .par_iter()
        .find_first(|t| t.length == 5 && t.token_type == type_number);
    assert!(
        int_token.is_some(),
        "Should find 5-digit int literal token. Tokens: {:?}",
        tokens
    );

    // 123.45f - length 7, token_type NUMBER
    let float_token = tokens
        .par_iter()
        .find_first(|t| t.length == 7 && t.token_type == type_number);
    assert!(
        float_token.is_some(),
        "Should find 7-character float literal token. Tokens: {:?}",
        tokens
    );
}

#[test]
fn test_str_tokens() {
    let source = "class X { public void Test(void) { Str.Tokens(pid, \"_\"); } };";
    let program = trainz_parser::gs::parse(source).unwrap();
    let ast = trainz_ast::gs::process::process_trainz_ast(program, source);

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
    let program = trainz_parser::gs::parse(source).unwrap();
    let ast = trainz_ast::gs::process::process_trainz_ast(program, source);

    let raw_tokens = crate::gs::semantic_tokens(&ast);
    let method_token = raw_tokens.par_iter().find_first(|(range, token_type, _)| {
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
    let pairs = trainz_parser::gs::grammar::GameScriptParser::parse(
        trainz_parser::gs::grammar::Rule::program,
        src,
    )
    .unwrap();
    let program = trainz_ast::gs::process::process_trainz_ast(pairs, src);
    let tokens = crate::process_raw_tokens(crate::gs::semantic_tokens(&program));
    println!("{:#?}", program.classes[0].methods[0]);
    println!("{:#?}", tokens);
}
