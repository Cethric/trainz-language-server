use crate::gs::process::process_trainz_ast;
use pest::Parser;
use trainz_parser::gs::grammar::{GameScriptParser, Rule};

fn parse_gs(src: &str) {
    let pairs = GameScriptParser::parse(Rule::program, src)
        .unwrap_or_else(|e| panic!("Parse failed: {}", e));
    process_trainz_ast(pairs, src);
}

#[test]
fn test_on_statement_variants() {
    let src = "class Test {
        void Main() {
            on \"event\", \"target\" { }
            on \"event\", \"target\", id { }
            on \"event\", \"target\" // comment
            { }
            on \"event\", \"target\", id // comment
            { }
        }
    };";
    parse_gs(src);
}

#[test]
fn test_if_else_comments() {
    let src = "class Test {
        void Main() {
            if (true) // comment
            { }
            else // comment
            { }

            if (true) { } else if (false) { }
        }
    };";
    parse_gs(src);
}

#[test]
fn test_params_comments() {
    let src = "class Test {
        void Method(int // comment
            a, string /* comment */ b) { }
    };";
    parse_gs(src);
}

#[test]
fn test_binary_resilience() {
    let src = "class Test {
        void Main() {
            int a = (1 + 2);
            int b = [1, 2][0];
            // Test resilience to extra tokens that might be captured by the greedy process_binary
            // although the parser should ideally not give them to it.
            int c = 1 + 2;
        }
    };";
    parse_gs(src);
}

#[test]
fn test_float_suffixes() {
    let src = "class Test {
        float f1 = 1.0f;
        float f2 = 1.0F;
        float f3 = 1.0;
    };";
    parse_gs(src);
}

#[test]
fn test_bitwise_and_math_operators() {
    let src = "class Test {
        void Main() {
            int a = 1 << 2;
            int b = 4 >> 1;
            int c = 1 & 1;
            int d = 1 | 2;
            int e = 1 ^ 3;
            int f = 10 % 3;
        }
    };";
    parse_gs(src);
}
