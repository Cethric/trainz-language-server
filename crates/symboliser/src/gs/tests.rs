#[cfg(test)]
mod tests {
    use crate::gs::trainz_symboliser;
    use pest::Parser;
    use rayon::prelude::*;
    use tower_lsp_server::ls_types::{SymbolKind, SymbolTag};
    use trainz_ast::gs::process::process_trainz_ast;
    use trainz_parser::gs::grammar::{GameScriptParser, Rule};

    fn get_symbols(src: &str) -> Vec<tower_lsp_server::ls_types::DocumentSymbol> {
        let pairs = GameScriptParser::parse(Rule::program, src)
            .unwrap_or_else(|e| panic!("Parse failed: {}", e));
        let program = process_trainz_ast(pairs, src);
        let mut symbols = trainz_symboliser(&program, &program);
        if symbols.len() == 1 && symbols[0].name == "file" {
            symbols.remove(0).children.unwrap_or_default()
        } else {
            symbols
        }
    }

    #[test]
    fn test_all_symbols_have_contained_selection_range() {
        let src = r#"
            include "test.gs"
            class Test {
                int field1;
                void Method1(int param1) { }
            };
        "#;
        let symbols = get_symbols(src);

        fn check_symbols(symbols: &[tower_lsp_server::ls_types::DocumentSymbol]) {
            for symbol in symbols {
                // Check if selection_range is contained in range
                let r = symbol.range;
                let s = symbol.selection_range;

                // Assert start position
                assert!(
                    r.start.line < s.start.line
                        || (r.start.line == s.start.line && r.start.character <= s.start.character),
                    "Symbol {} selection_range start ({:?}) not in range start ({:?})",
                    symbol.name,
                    s.start,
                    r.start
                );

                // Assert end position
                assert!(
                    r.end.line > s.end.line
                        || (r.end.line == s.end.line && r.end.character >= s.end.character),
                    "Symbol {} selection_range end ({:?}) not in range end ({:?})",
                    symbol.name,
                    s.end,
                    r.end
                );

                if let Some(children) = &symbol.children {
                    check_symbols(children);
                }
            }
        }

        check_symbols(&symbols);
    }

    #[test]
    fn test_type_symbolisation() {
        let src = "
    class Foo {};
    class Test {
        Foo m_foo;
        void Main(Foo param) {
            Foo local = cast<Foo>(null);
            Foo[] arr = new Foo[1];
        }
    };";
        let symbols = get_symbols(src);
        // Find Test class
        let test_class = symbols
            .iter()
            .find(|s| s.name == "Test")
            .expect("Should find Test class");
        let test_children = test_class
            .children
            .as_ref()
            .expect("Test class should have children");

        // m_foo should have Foo as a child symbol (they are siblings in our implementation)
        assert!(
            test_children
                .iter()
                .any(|s| s.name == "Foo" && s.kind == SymbolKind::CLASS),
            "Should find Foo class symbol for field type"
        );

        let main_method = test_children
            .iter()
            .find(|s| s.name == "Main")
            .expect("Should find Main method");
        let main_children = main_method
            .children
            .as_ref()
            .expect("Main method should have children");

        assert!(
            main_children
                .iter()
                .any(|s| s.name == "Foo" && s.kind == SymbolKind::CLASS),
            "Should find Foo class symbol for param/local type"
        );
    }

    #[test]
    fn test_nested_scoping() {
        let src = "
    class Test {
        void MyMethod(int p) {
            int x;
            if (true) {
                int y;
            }
            {
                int z;
            }
        }
    };";
        let pairs = GameScriptParser::parse(Rule::program, src)
            .unwrap_or_else(|e| panic!("Parse failed: {}", e));
        let program = process_trainz_ast(pairs, src);
        let symbols = trainz_symboliser(&program, &program);

        // Top-level symbol should be "file"
        assert_eq!(symbols.len(), 1);
        let file_symbol = &symbols[0];
        assert_eq!(file_symbol.name, "file");

        // Child of file is "Test" class
        let children = file_symbol.children.as_ref().unwrap();
        assert_eq!(children.len(), 1);
        let class_symbol = &children[0];
        assert_eq!(class_symbol.name, "Test");

        // Child of Test is "MyMethod"
        let class_children = class_symbol.children.as_ref().unwrap();
        assert_eq!(class_children.len(), 1);
        let method_symbol = &class_children[0];
        assert_eq!(method_symbol.name, "MyMethod");

        // Children of MyMethod: p, x, if, scope
        let method_children = method_symbol.children.as_ref().unwrap();
        assert!(method_children.iter().any(|s| s.name == "p"));
        assert!(method_children.iter().any(|s| s.name == "x"));

        let if_symbol = method_children.iter().find(|s| s.name == "if").unwrap();
        let if_children = if_symbol.children.as_ref().unwrap();
        assert!(if_children.iter().any(|s| s.name == "y"));

        let bare_block_symbol = method_children.iter().find(|s| s.name == "scope").unwrap();
        let bare_block_children = bare_block_symbol.children.as_ref().unwrap();
        assert!(bare_block_children.iter().any(|s| s.name == "z"));
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
        assert_eq!(
            children[3].detail,
            Some("void (declaration, definition)".to_string())
        );
        assert_eq!(children[3].tags, None);

        assert_eq!(children[4].name, "Method2");
        assert_eq!(
            children[4].detail,
            Some("void (declaration, definition)".to_string())
        );
        #[allow(deprecated)]
        {
            assert_eq!(children[4].tags, Some(vec![SymbolTag::DEPRECATED]));
        }

        assert_eq!(children[5].name, "NativeMethod");
        assert_eq!(children[5].kind, SymbolKind::METHOD);
        assert_eq!(
            children[5].detail,
            Some("void (declaration, definition)".to_string())
        );

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
                .par_iter()
                .any(|s| s.name == "a" && s.kind == SymbolKind::VARIABLE)
        );
        assert!(
            body_symbols
                .par_iter()
                .any(|s| s.name == "label1" && s.kind == SymbolKind::VARIABLE)
        );

        assert!(
            body_symbols
                .par_iter()
                .any(|s| s.name == "if" && s.kind == SymbolKind::NAMESPACE)
        );
        assert!(
            body_symbols
                .par_iter()
                .any(|s| s.name == "while" && s.kind == SymbolKind::NAMESPACE)
        );

        assert!(
            body_symbols
                .par_iter()
                .any(|s| s.name == "for" && s.kind == SymbolKind::NAMESPACE)
        );

        assert!(
            body_symbols
                .par_iter()
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
                .par_iter()
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
                .par_iter()
                .any(|s| s.name == "inherited" && s.kind == SymbolKind::VARIABLE)
        );
        assert!(
            body_symbols
                .par_iter()
                .any(|s| s.name == "pid" && s.kind == SymbolKind::VARIABLE)
        );

        // 2. Check if 'use_metric' is found as a variable after '!'
        assert!(
            body_symbols
                .par_iter()
                .any(|s| s.name == "use_metric" && s.kind == SymbolKind::VARIABLE)
        );

        // 3. Ensure '!' is NOT a variable
        assert!(!body_symbols.par_iter().any(|s| s.name == "!"));

        // 4. Check if single-line if return is handled
        let if_symbol = body_symbols
            .par_iter()
            .find_first(|s| s.name == "if")
            .expect("Should find 'if' symbol");
        let if_children = if_symbol
            .children
            .as_ref()
            .expect("If should have children");
        // Should contain 'use_metric' (from cond) and symbols from return statement
        assert!(
            if_children
                .par_iter()
                .any(|s| s.name == "use_metric" && s.kind == SymbolKind::VARIABLE)
        );
        assert!(
            if_children
                .par_iter()
                .any(|s| s.name == "pid" && s.kind == SymbolKind::VARIABLE)
        );

        // 5. Check out_description = out_description
        let out_desc_symbols: Vec<_> = body_symbols
            .par_iter()
            .filter(|s| s.name == "out_description")
            .collect();
        // One from declaration, two from assignment = 3
        assert_eq!(out_desc_symbols.len(), 3);

        // 6. Check soup.SetNamedTag
        assert!(body_symbols.par_iter().any(|s| s.name == "soup"));
        assert!(body_symbols.par_iter().any(|s| s.name == "SetNamedTag"));
        // string literals are formatted as Literal(String("..."))
        assert!(
            body_symbols
                .par_iter()
                .any(|s| s.name.contains("speed-normal"))
        );

        // 7. Check -1.0
        assert!(
            body_symbols
                .par_iter()
                .any(|s| s.name.contains("-1.0") || s.name == "-1.0")
        );
    }

    #[test]
    fn test_method_classification_symbols() {
        let src = r#"
            class Test {
                void OnlyDeclaration();
                void OnlyImplementation() { int i; }
                void SeparateDeclaration();
                void SeparateDeclaration() { int a; }
                native void NativeMethod();
                native void NativeWithBody() { int b; }
            };
        "#;
        let symbols = get_symbols(src);
        let test_class = &symbols[0];
        let children = test_class.children.as_ref().unwrap();

        // 1. OnlyDeclaration
        let only_decl = &children[0];
        assert_eq!(only_decl.name, "OnlyDeclaration");
        assert_eq!(only_decl.detail, Some("void (declaration)".to_string()));
        assert!(only_decl.children.is_none());

        // 2. OnlyImplementation
        let only_impl = &children[1];
        assert_eq!(only_impl.name, "OnlyImplementation");
        assert_eq!(
            only_impl.detail,
            Some("void (declaration, definition)".to_string())
        );
        assert!(
            only_impl
                .children
                .as_ref()
                .unwrap()
                .iter()
                .any(|s| s.name == "i")
        );

        // 3. SeparateDeclaration (declaration part)
        let sep_decl = &children[2];
        assert_eq!(sep_decl.name, "SeparateDeclaration");
        assert_eq!(sep_decl.detail, Some("void (declaration)".to_string()));
        assert!(sep_decl.children.is_none());

        // 4. SeparateDeclaration (implementation part)
        let sep_impl = &children[3];
        assert_eq!(sep_impl.name, "SeparateDeclaration");
        assert_eq!(sep_impl.detail, Some("void (definition)".to_string()));
        assert!(
            sep_impl
                .children
                .as_ref()
                .unwrap()
                .iter()
                .any(|s| s.name == "a")
        );

        // 5. NativeMethod
        let native = &children[4];
        assert_eq!(native.name, "NativeMethod");
        assert_eq!(
            native.detail,
            Some("void (declaration, definition)".to_string())
        );
        assert!(native.children.is_none());

        // 6. NativeWithBody
        let native_body = &children[5];
        assert_eq!(native_body.name, "NativeWithBody");
        assert_eq!(
            native_body.detail,
            Some("void (declaration, definition)".to_string())
        );
        assert!(
            native_body
                .children
                .as_ref()
                .unwrap()
                .iter()
                .any(|s| s.name == "b")
        );
    }
}
