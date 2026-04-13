use crate::gs::semantic_tokens;
use pest::Parser;
use rayon::prelude::*;
use trainz_ast::gs::process::process_trainz_ast;
use trainz_parser::gs::grammar::{GameScriptParser, Rule};

#[tracing::instrument]
fn get_token_type(target: tower_lsp_server::ls_types::SemanticTokenType) -> u32 {
    let (types, _) = crate::legend::get_legend();
    types.par_iter().position_first(|t| *t == target).unwrap() as u32
}

#[tracing::instrument]
fn get_tokens_for_src(src: &str) -> Vec<tower_lsp_server::ls_types::SemanticToken> {
    let pairs = GameScriptParser::parse(Rule::program, src).unwrap();
    let program = process_trainz_ast(pairs, src);
    crate::process_raw_tokens(semantic_tokens(&program), Some(src))
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
        !type_keyword_len4_tokens.is_empty(),
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
    let tokens = crate::process_raw_tokens(semantic_tokens(&program), Some(src));

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
    let tokens = crate::process_raw_tokens(semantic_tokens(&program), Some(src));

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
fn test_gs_complex_constructs_tokens() {
    let src = r#"
        include "test.gs";
        include "other.gs";

        class Test isclass Super {
            public static int a = 1, b = 2;
            string s = "multi-line
            string";

            public void Main(string[] args) {
                int x = 10;
                if (x > 0) {
                    x = cast<int>(123);
                    x = (int)456;
                    x++;
                    --x;
                }
                while (false) {
                    break;
                }
                int i;
                for (i = 0; i < 10; i++) {
                    continue;
                }
                switch (x) {
                    case 1:
                        x = 1;
                        break;
                    default:
                        x = 2;
                }
                on "event", "target" {
                    x = 0;
                }
                wait() {
                    x = -1;
                }
            }
        };
    "#;
    let tokens = get_tokens_for_src(src);
    assert!(!tokens.is_empty());
}

#[test]
fn test_multi_digit_number_tokens() {
    let src = "class Test { void Main() { int i = 12345; float f = 123.45f; } };";
    let pairs = GameScriptParser::parse(Rule::program, src).unwrap();
    let program = process_trainz_ast(pairs, src);
    let tokens = crate::process_raw_tokens(semantic_tokens(&program), Some(src));

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
    for class in ast.classes.values() {
        for ms in class.methods.values() {
            for method in ms {
                if let Some(body) = &method.body {
                    for stmt in &body.statements {
                        crate::gs::stmt::collect_stmt_tokens(stmt, &mut tokens, &known_classes);
                    }
                }
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
fn test_switch_case_body_tokens() {
    let src = r#"
        class Test {
            void Main() {
                int x = 10;
                switch (x) {
                    case 1:
                        int y = 20;
                        break;
                    default:
                        int z = 30;
                }
            }
        };
    "#;
    let tokens = get_tokens_for_src(src);

    let type_keyword = get_token_type(tower_lsp_server::ls_types::SemanticTokenType::KEYWORD);
    let type_variable = get_token_type(tower_lsp_server::ls_types::SemanticTokenType::VARIABLE);
    let type_number = get_token_type(tower_lsp_server::ls_types::SemanticTokenType::NUMBER);

    // Find 'case' keyword
    let case_token = tokens
        .iter()
        .find(|t| t.token_type == type_keyword && t.length == 4);
    assert!(case_token.is_some(), "Should find 'case' keyword");

    // Find 'default' keyword
    let default_token = tokens
        .iter()
        .find(|t| t.token_type == type_keyword && t.length == 7);
    assert!(default_token.is_some(), "Should find 'default' keyword");

    // Verify all variables are present (x, y, z)
    let var_tokens: Vec<_> = tokens
        .iter()
        .filter(|t| t.token_type == type_variable && t.length == 1)
        .collect();
    assert!(
        var_tokens.len() >= 3,
        "Should find at least 3 variable tokens (x, y, z). Found: {}",
        var_tokens.len()
    );

    // Verify all numbers are present (10, 20, 30)
    let num_tokens: Vec<_> = tokens
        .iter()
        .filter(|t| t.token_type == type_number && t.length == 2)
        .collect();
    assert!(
        num_tokens.len() >= 3,
        "Should find at least 3 number tokens (10, 20, 30). Found: {}",
        num_tokens.len()
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
    let tokens = crate::process_raw_tokens(crate::gs::semantic_tokens(&program), Some(src));
    println!(
        "{:#?}",
        program
            .classes
            .values()
            .next()
            .unwrap()
            .methods
            .values()
            .next()
            .unwrap()[0]
    );
    println!("{:#?}", tokens);
}

#[test]
fn test_method_modifiers() {
    use tower_lsp_server::ls_types::SemanticTokenModifier;

    let src = r#"
        class Test {
            void MethodImplementation() { }
            native void MethodDefinition();
        };
    "#;
    let pairs = GameScriptParser::parse(Rule::program, src).unwrap();
    let program = process_trainz_ast(pairs, src);
    let raw_tokens = crate::gs::semantic_tokens(&program);

    let impl_token = raw_tokens
        .iter()
        .find(|(range, _, _)| {
            let start = range.start.character as usize;
            let end = range.end.character as usize;
            // The source code in the test is multi-line, but the name is on its own line.
            // We'll just look for the name in the source.
            src.contains("MethodImplementation") && (end - start) == "MethodImplementation".len()
        })
        .unwrap();

    let def_token = raw_tokens
        .iter()
        .find(|(range, _, _)| {
            let start = range.start.character as usize;
            let end = range.end.character as usize;
            src.contains("MethodDefinition") && (end - start) == "MethodDefinition".len()
        })
        .unwrap();

    // Implementation should have both DEFINITION and DECLARATION
    let impl_modifiers = &impl_token.2;
    assert!(impl_modifiers.contains(&SemanticTokenModifier::DEFINITION));
    assert!(impl_modifiers.contains(&SemanticTokenModifier::DECLARATION));

    // Native should have both DEFINITION and DECLARATION
    let def_modifiers = &def_token.2;
    assert!(def_modifiers.contains(&SemanticTokenModifier::DEFINITION));
    assert!(def_modifiers.contains(&SemanticTokenModifier::DECLARATION));
}

#[test]
fn test_method_modifiers_separate_decl() {
    use tower_lsp_server::ls_types::SemanticTokenModifier;

    let src = r#"
        class Test {
            void Method();
            void Method() { }
        };
    "#;
    let pairs = GameScriptParser::parse(Rule::program, src).unwrap();
    let program = process_trainz_ast(pairs, src);
    let raw_tokens = crate::gs::semantic_tokens(&program);

    let decl_token = raw_tokens
        .iter()
        .find(|(range, token_type, _)| {
            range.start.line == 2
                && *token_type == tower_lsp_server::ls_types::SemanticTokenType::METHOD
        })
        .unwrap();

    let impl_token = raw_tokens
        .iter()
        .find(|(range, token_type, _)| {
            range.start.line == 3
                && *token_type == tower_lsp_server::ls_types::SemanticTokenType::METHOD
        })
        .unwrap();

    // Declaration should only have DECLARATION
    let decl_modifiers = &decl_token.2;
    assert!(decl_modifiers.contains(&SemanticTokenModifier::DECLARATION));
    assert!(!decl_modifiers.contains(&SemanticTokenModifier::DEFINITION));

    // Implementation with separate declaration should only have DEFINITION
    let impl_modifiers = &impl_token.2;
    assert!(impl_modifiers.contains(&SemanticTokenModifier::DEFINITION));
    assert!(!impl_modifiers.contains(&SemanticTokenModifier::DECLARATION));
}

#[test]
fn test_method_modifiers_only_declaration() {
    use tower_lsp_server::ls_types::SemanticTokenModifier;

    let src = r#"
        class Test {
            void OnlyDeclaration();
        };
    "#;
    let pairs = GameScriptParser::parse(Rule::program, src).unwrap();
    let program = process_trainz_ast(pairs, src);
    let raw_tokens = crate::gs::semantic_tokens(&program);

    let decl_token = raw_tokens
        .iter()
        .find(|(range, _token_type, _)| {
            let start = range.start.character as usize;
            let end = range.end.character as usize;
            src.contains("OnlyDeclaration") && (end - start) == "OnlyDeclaration".len()
        })
        .unwrap();

    // Should only have DECLARATION
    let modifiers = &decl_token.2;
    assert!(modifiers.contains(&SemanticTokenModifier::DECLARATION));
    assert!(!modifiers.contains(&SemanticTokenModifier::DEFINITION));
}

#[test]
fn test_method_modifiers_multiple_implementations_no_decl() {
    use tower_lsp_server::ls_types::SemanticTokenModifier;

    let src = r#"
        class Test {
            void Method() { }
            void Method() { }
        };
    "#;
    let pairs = GameScriptParser::parse(Rule::program, src).unwrap();
    let program = process_trainz_ast(pairs, src);
    let raw_tokens = crate::gs::semantic_tokens(&program);

    let method_tokens: Vec<_> = raw_tokens
        .iter()
        .filter(|(range, token_type, _)| {
            let start = range.start.character as usize;
            let end = range.end.character as usize;
            src.contains("Method")
                && (end - start) == "Method".len()
                && *token_type == tower_lsp_server::ls_types::SemanticTokenType::METHOD
        })
        .collect();

    assert_eq!(method_tokens.len(), 2);

    // Both should have both DEFINITION and DECLARATION
    for token in method_tokens {
        let modifiers = &token.2;
        assert!(modifiers.contains(&SemanticTokenModifier::DEFINITION));
        assert!(modifiers.contains(&SemanticTokenModifier::DECLARATION));
    }
}

#[test]
fn test_method_modifiers_native_with_body() {
    use tower_lsp_server::ls_types::SemanticTokenModifier;

    let src = r#"
        class Test {
            native void NativeWithBody() { }
        };
    "#;
    let pairs = GameScriptParser::parse(Rule::program, src).unwrap();
    let program = process_trainz_ast(pairs, src);
    let raw_tokens = crate::gs::semantic_tokens(&program);

    let method_token = raw_tokens
        .iter()
        .find(|(range, _, _)| {
            let start = range.start.character as usize;
            let end = range.end.character as usize;
            src.contains("NativeWithBody") && (end - start) == "NativeWithBody".len()
        })
        .unwrap();

    // Should have both DEFINITION and DECLARATION
    let modifiers = &method_token.2;
    assert!(modifiers.contains(&SemanticTokenModifier::DEFINITION));
    assert!(modifiers.contains(&SemanticTokenModifier::DECLARATION));
}

#[test]
fn test_signal_gs_tokens() {
    let src = "
    class Signal {
        void Init() {
            me.Init();
            cast<Vehicle>(nextObject);
            new Object();
            new int[10];
        }
    };";
    let tokens = get_tokens_for_src(src);
    let type_keyword = get_token_type(tower_lsp_server::ls_types::SemanticTokenType::KEYWORD);

    // Check for 'me'
    let me_token = tokens
        .iter()
        .find(|t| t.token_type == type_keyword && t.length == 2);
    assert!(me_token.is_some(), "Should find 'me' keyword token");

    // Check for 'cast'
    let cast_token = tokens
        .iter()
        .find(|t| t.token_type == type_keyword && t.length == 4);
    assert!(cast_token.is_some(), "Should find 'cast' keyword token");

    // Check for 'new' (there should be two)
    let new_tokens: Vec<_> = tokens
        .iter()
        .filter(|t| t.token_type == type_keyword && t.length == 3)
        .collect();
    assert!(
        new_tokens.len() >= 2,
        "Should find at least two 'new' keyword tokens"
    );
}

#[test]
fn test_logical_keywords() {
    let src = "class Test { void Main() { bool b = true and false; bool c = true or false; } };";
    let tokens = get_tokens_for_src(src);
    let type_keyword = get_token_type(tower_lsp_server::ls_types::SemanticTokenType::KEYWORD);

    // Find 'and' keyword
    let and_token = tokens
        .iter()
        .find(|t| t.token_type == type_keyword && t.length == 3);
    assert!(and_token.is_some(), "Should find 'and' keyword token");

    // Find 'or' keyword
    let or_token = tokens
        .iter()
        .find(|t| t.token_type == type_keyword && t.length == 2);
    assert!(or_token.is_some(), "Should find 'or' keyword token");
}
