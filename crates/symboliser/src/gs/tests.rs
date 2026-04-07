#[cfg(test)]
mod tests {
    use crate::gs::gs_symboliser;
    use gs_ast::gs::process::process_gs_ast;
    use gs_parser::gs::grammar::{GameScriptParser, Rule};
    use pest::Parser;
    use tower_lsp_server::ls_types::{SymbolKind, SymbolTag};

    fn get_symbols(src: &str) -> Vec<tower_lsp_server::ls_types::DocumentSymbol> {
        let pairs = GameScriptParser::parse(Rule::program, src)
            .unwrap_or_else(|e| panic!("Parse failed: {}", e));
        let program = process_gs_ast(pairs, src);
        gs_symboliser(&program)
    }

    #[test]
    fn test_class_and_members() {
        let src = r#"
            class Test {
                int field1;
                string field2, field3;

                void Method1() { }
                obsolete(1) void Method2() { }
                native void NativeMethod();
            };
            obsolete(1) class ObsoleteClass { };
        "#;
        let symbols = get_symbols(src);

        assert_eq!(symbols.len(), 2);

        let test_class = &symbols[0];
        assert_eq!(test_class.name, "Test");
        assert_eq!(test_class.kind, SymbolKind::CLASS);

        let children = test_class.children.as_ref().unwrap();
        // field1, field2, field3, Method1, Method2, NativeMethod = 6
        assert_eq!(children.len(), 6);

        assert_eq!(children[0].name, "field1");
        assert_eq!(children[0].kind, SymbolKind::PROPERTY);

        assert_eq!(children[1].name, "field2");
        assert_eq!(children[2].name, "field3");

        assert_eq!(children[3].name, "Method1");
        assert_eq!(children[3].kind, SymbolKind::METHOD);
        assert_eq!(children[3].tags, None);

        assert_eq!(children[4].name, "Method2");
        #[allow(deprecated)]
        {
            assert_eq!(children[4].tags, Some(vec![SymbolTag::DEPRECATED]));
            assert_eq!(children[4].deprecated, Some(true));
        }

        assert_eq!(children[5].name, "NativeMethod");
        assert_eq!(children[5].kind, SymbolKind::METHOD);

        let obsolete_class = &symbols[1];
        assert_eq!(obsolete_class.name, "ObsoleteClass");
        #[allow(deprecated)]
        {
            assert_eq!(obsolete_class.tags, Some(vec![SymbolTag::DEPRECATED]));
            assert_eq!(obsolete_class.deprecated, Some(true));
        }
    }

    #[test]
    fn test_statements() {
        let src = r#"
            class Test {
                void Main() {
                    int a = 1;
                    label1:
                    if (a == 1) {
                        a = 2;
                    }
                    while (a < 10) {
                        a = a + 1;
                    }
                    int i;
                    for (i = 0; i < 5; i = i + 1) {
                        wait () {
                            on "event", "target" {
                                return;
                            }
                        }
                    }
                    switch (a) {
                        case 1: { break; }
                        default: { }
                    }
                }
            };
        "#;
        let symbols = get_symbols(src);
        let main_method = &symbols[0].children.as_ref().unwrap()[0];
        let body_symbols = main_method.children.as_ref().unwrap();

        // Let's verify some key ones
        assert!(
            body_symbols
                .iter()
                .any(|s| s.name == "a" && s.kind == SymbolKind::VARIABLE)
        );
        assert!(
            body_symbols
                .iter()
                .any(|s| s.name == "label1" && s.kind == SymbolKind::VARIABLE)
        );

        assert!(
            body_symbols
                .iter()
                .any(|s| s.name == "if" && s.kind == SymbolKind::NAMESPACE)
        );
        assert!(
            body_symbols
                .iter()
                .any(|s| s.name == "while" && s.kind == SymbolKind::NAMESPACE)
        );

        assert!(
            body_symbols
                .iter()
                .any(|s| s.name == "for" && s.kind == SymbolKind::NAMESPACE)
        );

        assert!(
            body_symbols
                .iter()
                .any(|s| s.name == "switch" && s.kind == SymbolKind::NAMESPACE)
        );
    }

    #[test]
    fn test_expressions() {
        let src = r#"
            class Test {
                void Main() {
                    int a;
                    a = 1 + 2;
                }
            };
        "#;
        let symbols = get_symbols(src);
        let main_method = &symbols[0].children.as_ref().unwrap()[0];
        let body_symbols = main_method.children.as_ref().unwrap();

        // a = 1 + 2 -> "assign" symbol
        assert!(
            body_symbols
                .iter()
                .any(|s| s.name == "assign" && s.kind == SymbolKind::VARIABLE)
        );
    }

    #[test]
    fn test_includes() {
        let src = r#"
            include "test.gs"
            include "utils.gs"

            class Test { };
        "#;
        let symbols = get_symbols(src);

        // 2 includes + 1 class
        assert_eq!(symbols.len(), 3);

        assert_eq!(symbols[0].name, "test.gs");
        assert_eq!(symbols[0].kind, SymbolKind::MODULE);

        assert_eq!(symbols[1].name, "utils.gs");
        assert_eq!(symbols[1].kind, SymbolKind::MODULE);

        assert_eq!(symbols[2].name, "Test");
        assert_eq!(symbols[2].kind, SymbolKind::CLASS);
    }
    #[test]
    fn test_inherited_and_unary_not() {
        let src = r#"
            class Test {
                void Main(int pid) {
                    inherited(pid);
                    bool use_metric;
                    !use_metric;
                    if (use_metric) return "test" + pid;
                    string out_description = "start";
                    out_description = out_description;
                    Soup soup;
                    soup.SetNamedTag("speed-normal", speed_limit_normal);
                    float f = -1.0;
                }
            };
        "#;
        let symbols = get_symbols(src);

        // Find Main method
        let test_class = &symbols[0];
        let main_method = &test_class.children.as_ref().unwrap()[0];
        let body_symbols = main_method.children.as_ref().unwrap();

        // 1. Check if 'pid' and 'inherited' are found
        assert!(
            body_symbols
                .iter()
                .any(|s| s.name == "inherited" && s.kind == SymbolKind::VARIABLE)
        );
        assert!(
            body_symbols
                .iter()
                .any(|s| s.name == "pid" && s.kind == SymbolKind::VARIABLE)
        );

        // 2. Check if 'use_metric' is found as a variable after '!'
        assert!(
            body_symbols
                .iter()
                .any(|s| s.name == "use_metric" && s.kind == SymbolKind::VARIABLE)
        );

        // 3. Ensure '!' is NOT a variable
        assert!(!body_symbols.iter().any(|s| s.name == "!"));

        // 4. Check if single-line if return is handled
        let if_symbol = body_symbols
            .iter()
            .find(|s| s.name == "if")
            .expect("Should find 'if' symbol");
        let if_children = if_symbol
            .children
            .as_ref()
            .expect("If should have children");
        // Should contain 'use_metric' (from cond) and symbols from return statement
        assert!(
            if_children
                .iter()
                .any(|s| s.name == "use_metric" && s.kind == SymbolKind::VARIABLE)
        );
        assert!(
            if_children
                .iter()
                .any(|s| s.name == "pid" && s.kind == SymbolKind::VARIABLE)
        );

        // 5. Check out_description = out_description
        let out_desc_symbols: Vec<_> = body_symbols
            .iter()
            .filter(|s| s.name == "out_description")
            .collect();
        // One from declaration, two from assignment = 3
        assert_eq!(out_desc_symbols.len(), 3);

        // 6. Check soup.SetNamedTag
        assert!(body_symbols.iter().any(|s| s.name == "soup"));
        assert!(body_symbols.iter().any(|s| s.name == "SetNamedTag"));
        // string literals are formatted as Literal(String("..."))
        assert!(body_symbols.iter().any(|s| s.name.contains("speed-normal")));

        // 7. Check -1.0
        assert!(
            body_symbols
                .iter()
                .any(|s| s.name.contains("-1.0") || s.name == "-1.0")
        );
    }
}
