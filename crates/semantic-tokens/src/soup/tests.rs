#[cfg(test)]
mod tests {
    use crate::soup::soup_semantic_tokens;
    use gs_ast::soup::process::process_soup_ast;
    use gs_parser::soup::parse_soup;
    use std::path::Path;

    #[test]
    fn test_soup_semantic_tokens() {
        let code = r#"
        container
        {
            key "value"
            number 42
        }
        "#;
        let pairs = parse_soup(code).unwrap();
        let soup = process_soup_ast(pairs, code, Path::new(""), &vec![], &vec![]);
        let tokens = crate::process_raw_tokens(soup_semantic_tokens(&soup, None));

        // We expect tokens for `container` (keyword/string?), `key` (keyword), `"value"` (string), `number` (keyword), `42` (number).
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_soup_token_lengths() {
        let code = "multi_digit 123456\nfloat_val 12.345f\nstring_val \"hello world\"\nkuid_val <KUID:123456:7890>\nvar_val $(my_variable)";
        let pairs = parse_soup(code).unwrap();
        let soup = process_soup_ast(pairs, code, Path::new(""), &vec![], &vec![]);
        let tokens = crate::process_raw_tokens(soup_semantic_tokens(&soup, None));

        // 123456 - length 6, type 6 (NUMBER)
        let int_token = tokens.iter().find(|t| t.length == 6 && t.token_type == 6);
        assert!(
            int_token.is_some(),
            "Should find 6-digit int literal token. Tokens: {:?}",
            tokens
        );

        // 12.345f - length 7, type 6 (NUMBER)
        let float_token = tokens.iter().find(|t| t.length == 7 && t.token_type == 6);
        assert!(
            float_token.is_some(),
            "Should find 7-character float literal token. Tokens: {:?}",
            tokens
        );

        // \"hello world\" - length 13, type 2 (STRING)
        let string_token = tokens.iter().find(|t| t.length == 13 && t.token_type == 2);
        assert!(
            string_token.is_some(),
            "Should find string literal token. Tokens: {:?}",
            tokens
        );

        // <KUID:123456:7890> - length 18, type 7 (PROPERTY in Soup)
        let kuid_token = tokens.iter().find(|t| t.length == 18 && t.token_type == 7);
        assert!(
            kuid_token.is_some(),
            "Should find KUID literal token. Tokens: {:?}",
            tokens
        );

        // $(my_variable) - length 14, type 10 (VARIABLE)
        let var_token = tokens.iter().find(|t| t.length == 14 && t.token_type == 10);
        assert!(
            var_token.is_some(),
            "Should find variable literal token. Tokens: {:?}",
            tokens
        );
    }

    #[test]
    fn test_soup_multiline_string_semantic_tokens() {
        let code = r#"
        description "This is a
        multi-line
        string"
        "#;
        let pairs = parse_soup(code).unwrap();
        let soup = process_soup_ast(pairs, code, Path::new(""), &vec![], &vec![]);
        let tokens = crate::process_raw_tokens(soup_semantic_tokens(&soup, None));

        // We expect 3 tokens for the string literal, one for each line.
        let string_tokens: Vec<_> = tokens.iter().filter(|t| t.token_type == 2).collect();
        assert!(
            !string_tokens.is_empty(),
            "Should find string literal tokens. Tokens: {:?}",
            tokens
        );

        // "This is a
        assert!(
            string_tokens.iter().any(|t| t.length == 10),
            "Should find first line of multiline string. Tokens: {:?}",
            string_tokens
        );
        // multi-line
        assert!(
            string_tokens.iter().any(|t| t.length == 18),
            "Should find second line of multiline string. Tokens: {:?}",
            string_tokens
        );
        // string"
        assert!(
            string_tokens.iter().any(|t| t.length == 15),
            "Should find third line of multiline string. Tokens: {:?}",
            string_tokens
        );
    }

    #[test]
    fn test_soup_semantic_tokens_deprecation() {
        use gs_diagnostics::soup::{ContainerRule, ContainerValidator, Validators};
        use tower_lsp_server::ls_types::SemanticTokenModifier;

        let code = r#"
        kind "test-container"
        obsolete-key "value"
        "#;
        let pairs = parse_soup(code).unwrap();
        let soup = process_soup_ast(pairs, code, Path::new("test.soup"), &vec![], &vec![]);

        let mut validators = Validators::default();
        validators.containers.push(ContainerValidator {
            container_name: "test-container".to_string(),
            top_level: true,
            rules: vec![ContainerRule {
                key: "obsolete-key".to_string(),
                obsolete_tag: Some(true),
                ..Default::default()
            }],
            ..Default::default()
        });

        let raw_tokens = soup_semantic_tokens(&soup, Some(&validators));
        let obsolete_token = raw_tokens
            .iter()
            .find(|(_r, _t, m)| m.contains(&SemanticTokenModifier::DEPRECATED));

        assert!(
            obsolete_token.is_some(),
            "Expected to find a token with DEPRECATED modifier. Tokens: {:?}",
            raw_tokens
        );
    }
}
