use crate::find::HasRange;
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

#[test]
fn test_include_range() {
    let src = "include \"test.gs\"\n\n\nclass Foo {};";
    let pairs = GameScriptParser::parse(Rule::program, src).unwrap();
    let program = process_trainz_ast(pairs, src);

    assert_eq!(program.includes.len(), 1);
    let include = &program.includes[0];

    // Check that the range doesn't span multiple lines
    assert_eq!(include.range.start.line, 0);
    assert_eq!(include.range.end.line, 0);

    let src_with_semicolon = "include \"test.gs\";\n\n\nclass Foo {};";
    let pairs = GameScriptParser::parse(Rule::program, src_with_semicolon).unwrap();
    let program = process_trainz_ast(pairs, src_with_semicolon);

    assert_eq!(program.includes.len(), 1);
    let include = &program.includes[0];
    assert_eq!(include.range.start.line, 0);
    assert_eq!(include.range.end.line, 0);
}

#[test]
fn test_label_range() {
    let src = "class Test {\n    void Main() {\n        label:\n\n\n        return;\n    }\n};";
    let pairs = GameScriptParser::parse(Rule::program, src).unwrap();
    let program = process_trainz_ast(pairs, src);

    let method = &program
        .classes
        .values()
        .next()
        .unwrap()
        .methods
        .values()
        .next()
        .unwrap()[0];
    let label_stmt = &method.body.as_ref().unwrap().statements[0];
    if let crate::gs::Stmt::Label(_, _, _) = label_stmt {
        let range = label_stmt.range();
        assert_eq!(range.start.line, 2);
        assert_eq!(range.end.line, 2);
    } else {
        panic!("Expected label statement");
    }
}

#[test]
fn test_return_range() {
    let src = "class Test {\n    void Main() {\n        return\n\n\n        ;\n    }\n};";
    let pairs = GameScriptParser::parse(Rule::program, src).unwrap();
    let program = process_trainz_ast(pairs, src);

    let method = &program
        .classes
        .values()
        .next()
        .unwrap()
        .methods
        .values()
        .next()
        .unwrap()[0];
    let return_stmt = &method.body.as_ref().unwrap().statements[0];
    if let crate::gs::Stmt::Return(_, _, _) = return_stmt {
        let range = return_stmt.range();
        assert_eq!(range.start.line, 2);
        assert_eq!(range.end.line, 2);
    } else {
        panic!("Expected return statement");
    }
}

#[test]
fn test_wait_range() {
    let src = "class Test {\n    void Main() {\n        wait() { }\n\n\n        return;\n    }\n};";
    let pairs = GameScriptParser::parse(Rule::program, src).unwrap();
    let program = process_trainz_ast(pairs, src);

    let method = &program
        .classes
        .values()
        .next()
        .unwrap()
        .methods
        .values()
        .next()
        .unwrap()[0];
    let wait_stmt = &method.body.as_ref().unwrap().statements[0];
    if let crate::gs::Stmt::Wait(_) = wait_stmt {
        let range = wait_stmt.range();
        assert_eq!(range.start.line, 2);
        assert_eq!(range.end.line, 2);
    } else {
        panic!("Expected wait statement");
    }
}
