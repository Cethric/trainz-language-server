use crate::gs::process::process_trainz_ast;
use pest::Parser;
use tower_lsp_server::ls_types::Position;
use trainz_parser::gs::grammar::{GameScriptParser, Rule};

fn parse_gs(src: &str) -> crate::gs::Program {
    let pairs = GameScriptParser::parse(Rule::program, src)
        .unwrap_or_else(|e| panic!("Parse failed: {}", e));
    process_trainz_ast(pairs, src)
}

#[test]
fn test_scope_tree_traversal() {
    let src = "class Test {
    int field;
    void Method(int param) {
        int local = 1;
        if (true) {
            int inner = 2;
            // Position 1: inside if block (line 6, col 12 approx)
        }
        // Position 2: inside method, outside if block (line 8, col 8 approx)
    }
};
";
    let program = parse_gs(src);

    // Test Position 1: inside if block (line 6, col 12 approx)
    let pos1 = Position::new(6, 12);
    let _scope1 = program
        .find_narrowest_scope(pos1)
        .expect("Should find narrowest scope");

    // Should find 'inner'
    assert!(
        program.find_variable_declaration("inner", pos1).is_some(),
        "Should find 'inner' at pos1"
    );
    // Should find 'local' (parent scope)
    assert!(
        program.find_variable_declaration("local", pos1).is_some(),
        "Should find 'local' at pos1"
    );
    // Should find 'param' (grandparent scope)
    assert!(
        program.find_variable_declaration("param", pos1).is_some(),
        "Should find 'param' at pos1"
    );
    // Should find 'field' (great-grandparent scope)
    assert!(
        program.find_variable_declaration("field", pos1).is_some(),
        "Should find 'field' at pos1"
    );

    // Test Position 2: inside method, outside if block (line 8, col 8 approx)
    let pos2 = Position::new(8, 8);
    let _scope2 = program
        .find_narrowest_scope(pos2)
        .expect("Should find narrowest scope at pos2");

    // Should NOT find 'inner' (it's out of scope)
    assert!(
        program.find_variable_declaration("inner", pos2).is_none(),
        "Should NOT find 'inner' at pos2"
    );
    // Should find 'local'
    assert!(
        program.find_variable_declaration("local", pos2).is_some(),
        "Should find 'local' at pos2"
    );
    // Should find 'param'
    assert!(
        program.find_variable_declaration("param", pos2).is_some(),
        "Should find 'param' at pos2"
    );
    // Should find 'field'
    assert!(
        program.find_variable_declaration("field", pos2).is_some(),
        "Should find 'field' at pos2"
    );

    // Test that it doesn't find a variable declared after the position
    let pos_before_local = Position::new(3, 4); // Before 'int local = 1;' (it's on line 3, col 8)
    assert!(
        program
            .find_variable_declaration("local", pos_before_local)
            .is_none(),
        "Should NOT find 'local' before its declaration"
    );
}

#[test]
fn test_shadowing_lookup() {
    let src = "class Test {
    int x;
    void Method(int x) {
        int x = 1;
        if (true) {
            int x = 2;
            // Position: inside if block
        }
    }
};
";
    let program = parse_gs(src);
    let pos = Position::new(6, 12);

    let (_ty, id) = program
        .find_variable_declaration("x", pos)
        .expect("Should find 'x'");
    // It should find the one in the current scope (int x = 2)
    // We can check the line number of the found identifier.
    assert_eq!(
        id.range.start.line, 5,
        "Should find the shadowed 'x' on line 5"
    );
}

#[test]
fn test_no_declaration_found() {
    let src = "class Test {
    void Method() {
        int a = 1;
    }
};
";
    let program = parse_gs(src);
    let pos = Position::new(2, 8);

    assert!(
        program
            .find_variable_declaration("nonexistent", pos)
            .is_none()
    );
}
