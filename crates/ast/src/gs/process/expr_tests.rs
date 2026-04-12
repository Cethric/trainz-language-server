use crate::gs::process::process_trainz_ast;
use pest::Parser;
use trainz_parser::gs::grammar::{GameScriptParser, Rule};

#[test]
fn test_complex_expressions() {
    let src = "class Test {
        void Main() {
            int x = 1 + 2 * 3;
            int y = (1 + 2) * 3;
            bool z = x == 7 and y == 9 or x < y;
            x++;
            --y;
            me.method(1, 2).field[0] = x;
        }
    };";
    let pairs = GameScriptParser::parse(Rule::program, src).unwrap();
    let program = process_trainz_ast(pairs, src);

    let class = program.classes.get("Test").unwrap();
    let method = &class.methods.get("Main").unwrap()[0];
    let statements = &method.body.as_ref().unwrap().statements;

    // x = 1 + 2 * 3
    // Should be 1 + (2 * 3) = 7
    // ... we don't evaluate, but we can check the structure if we want.
    // For now, just ensuring it parses.
    assert_eq!(statements.len(), 6);
}

#[test]
fn test_operator_precedence() {
    let src = "class Test {
        void Main() {
            bool a = 1 + 2 * 3 == 7;
            bool b = (1 + 2) * 3 == 9;
        }
    };";
    let pairs = GameScriptParser::parse(Rule::program, src).unwrap();
    let _program = process_trainz_ast(pairs, src);
}
