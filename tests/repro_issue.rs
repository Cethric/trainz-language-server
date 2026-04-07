#[cfg(test)]
mod tests {
    use gs_ast::gs::process::process_gs_ast;
    use gs_parser::gs::grammar::{GameScriptParser, Rule};
    use pest::Parser;
    use symboliser::gs::gs_symboliser;
    use tower_lsp_server::ls_types::SymbolKind;

    fn get_symbols(src: &str) -> Vec<tower_lsp_server::ls_types::DocumentSymbol> {
        let pairs = GameScriptParser::parse(Rule::program, src)
            .unwrap_or_else(|e| panic!("Parse failed: {}", e));
        let program = process_gs_ast(pairs, src);
        gs_symboliser(&program)
    }

    #[test]
    fn test_inherited_and_unary_not() {
        let src = r#"
            class Test {
                void Main(int pid) {
                    inherited(pid);
                    bool use_metric;
                    !use_metric;
                }
            };
        "#;
        let symbols = get_symbols(src);
        
        // Find Main method
        let test_class = &symbols[0];
        let main_method = &test_class.children.as_ref().unwrap()[0];
        let body_symbols = main_method.children.as_ref().unwrap();

        // 1. Check if 'pid' is found within inherited call
        // inherited(pid) -> Postfix -> [Identifier("inherited"), PostfixOp::Call([Identifier("pid")])]
        // Postfix handling should produce children for call
        let inherited_call = body_symbols.iter().find(|s| s.name == "call").expect("Should find 'call' symbol");
        let call_children = inherited_call.children.as_ref().expect("Call should have children");
        assert!(call_children.iter().any(|s| s.name == "pid" && s.kind == SymbolKind::VARIABLE));

        // 2. Check if 'use_metric' is found as a variable after '!'
        // !use_metric; -> Unary -> Identifier("use_metric")
        // Unary should just extend with symbols from operand
        assert!(body_symbols.iter().any(|s| s.name == "use_metric" && s.kind == SymbolKind::VARIABLE));
        
        // 3. Ensure '!' is NOT a variable
        assert!(!body_symbols.iter().any(|s| s.name == "!"));
    }
}
