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

#[test]
fn test_error_recovery() {
    let src = "class Test {
        void Main() {
            int a = 1;
            !!! garbage !!!
            int b = 2;
        }
    };";
    let pairs = GameScriptParser::parse(Rule::program, src).unwrap();
    let program = process_trainz_ast(pairs, src);

    // We should still have both 'a' and 'b' in the AST!
    let class = program.classes.get("Test").unwrap();
    let method = &class.methods.get("Main").unwrap()[0];
    let statements = &method.body.as_ref().unwrap().statements;

    assert_eq!(statements.len(), 2);
}

#[test]
fn test_on_statement_identifier() {
    let src = "
        class Test {
            void Main() {
                on \"event\", \"target\", msg: {
                    int a = 1;
                }
            }
        };
    ";
    let pairs = GameScriptParser::parse(Rule::program, src).unwrap();
    let program = process_trainz_ast(pairs, src);

    let class = program.classes.get("Test").unwrap();
    let method = &class.methods.get("Main").unwrap()[0];
    let on_stmt = match &method.body.as_ref().unwrap().statements[0] {
        crate::gs::Stmt::On(on) => on,
        _ => panic!("Expected ON statement"),
    };

    assert_eq!(on_stmt.identifier.as_ref().unwrap().name, "msg");
}

#[test]
fn test_error_recovery_top_level() {
    let src = "
        include \"a.gs\";
        @#$%
        class A {};
        @#$%
        include \"b.gs\";
    ";
    let pairs = GameScriptParser::parse(Rule::program, src).unwrap();
    let program = process_trainz_ast(pairs, src);

    assert_eq!(program.includes.len(), 2);
    assert!(program.classes.contains_key("A"));
}

#[test]
fn test_error_recovery_switch() {
    let src = "
        class A {
            void Main() {
                switch (x) {
                    case 1:
                        int a = 1;
                    @#$%
                    case 2:
                        int b = 2;
                    @#$%
                    default:
                        int c = 3;
                }
            }
        };
    ";
    let pairs = GameScriptParser::parse(Rule::program, src).unwrap();
    let program = process_trainz_ast(pairs, src);

    let class = program.classes.get("A").unwrap();
    let method = &class.methods.get("Main").unwrap()[0];
    let switch_stmt = match &method.body.as_ref().unwrap().statements[0] {
        crate::gs::Stmt::Switch(switch) => switch,
        _ => panic!("Expected switch statement"),
    };

    // It should have 2 cases and 1 default
    assert_eq!(switch_stmt.cases.len(), 2);
    assert!(switch_stmt.default.is_some());

    // Bodies should not be empty
    assert_eq!(switch_stmt.cases[0].body.statements.len(), 1);
    assert_eq!(switch_stmt.cases[1].body.statements.len(), 1);
    assert_eq!(switch_stmt.default.as_ref().unwrap().statements.len(), 1);

    if let crate::gs::Expr::Literal(crate::gs::Literal::Int(val, _)) = switch_stmt.cases[0].value {
        assert_eq!(val, 1);
    } else {
        panic!("Expected literal 1");
    }

    if let crate::gs::Expr::Literal(crate::gs::Literal::Int(val, _)) = switch_stmt.cases[1].value {
        assert_eq!(val, 2);
    } else {
        panic!("Expected literal 2");
    }
}

#[test]
fn test_error_recovery_class_body() {
    let src = "
        class A {
            int a;
            @#$%
            void Method() {}
            @#$%
            string s;
        };
    ";
    let pairs = GameScriptParser::parse(Rule::program, src).unwrap();
    let program = process_trainz_ast(pairs, src);

    let class = program.classes.get("A").unwrap();
    assert!(class.fields.contains_key("a"));
    assert!(class.fields.contains_key("s"));
    assert!(class.methods.contains_key("Method"));
}

#[test]
fn test_error_recovery_statements() {
    let src = "
        class A {
            void Main() {
                int a = 1;
                @#$%
                if (true) {
                    @#$%
                    int b = 2;
                }
                @#$%
                return;
            }
        };
    ";
    let pairs = GameScriptParser::parse(Rule::program, src).unwrap();
    let program = process_trainz_ast(pairs, src);

    let class = program.classes.get("A").unwrap();
    let method = &class.methods.get("Main").unwrap()[0];
    let stmts = &method.body.as_ref().unwrap().statements;

    // a = 1, IF, return
    assert_eq!(stmts.len(), 3);
    if let crate::gs::Stmt::If(if_stmt) = &stmts[1] {
        // b = 2
        assert_eq!(if_stmt.then_block.statements.len(), 1);
    } else {
        panic!("Expected IF statement");
    }
}

#[test]
fn test_signal_gs_parse() {
    // Actually, I can just use the content from attached files.
    // Since I cannot include_str a file that is not in the crate, I'll just paste a representative part of it.
    let src = "
include \"gs.gs\"
game class Signal isclass Trackside
{
  public define int GREEN = 2;
  public native bool SetSignalOwner(SecurityToken token, TrainzGameObject owner);
  public obsolete void SetSignalState(int state, string reason)
  {
    SetSignalState(null, state, reason);
  }
  public float GetOverlap(void)
  {
    return overlap;
  }
  public Soup DetermineUpdatedState(void)
  {
    GSTrackSearch myGST = me.BeginTrackSearch(true);
    if (cast<Vehicle>(nextObject)) {
        return null;
    }
    return null;
  }
};
";
    let pairs = GameScriptParser::parse(Rule::program, src).unwrap();
    let program = process_trainz_ast(pairs, src);
    assert!(program.classes.contains_key("Signal"));

    let class = program.classes.get("Signal").unwrap();
    let method = &class.methods.get("DetermineUpdatedState").unwrap()[0];
    let stmts = &method.body.as_ref().unwrap().statements;

    // GSTrackSearch myGST = me.BeginTrackSearch(true);
    if let crate::gs::Stmt::Decl(decl) = &stmts[0]
        && let Some(crate::gs::Expr::Postfix { expr: base, .. }) = decl.values.first()
    {
        if let crate::gs::Expr::Identifier(id) = &**base {
            assert_eq!(id.name, "me");
        } else {
            panic!("Expected 'me' as base of postfix, got {:?}", base);
        }
    } else {
        panic!("Expected declaration as first statement");
    }

    // if (cast<Vehicle>(nextObject))
    if let crate::gs::Stmt::If(if_stmt) = &stmts[1] {
        match &if_stmt.cond {
            crate::gs::Expr::Cast { ty, .. } => {
                assert_eq!(format!("{}", ty), "Vehicle");
            }
            _ => panic!(
                "Expected cast expression in condition, got {:?}",
                if_stmt.cond
            ),
        }
    }
}

#[test]
fn test_if_else_single_statement_parsing() {
    let src = "class Test {
        void Main(Message msg) {
            GameObject srcObj = cast<GameObject>(msg.src);
            if (msg.minor == \"Failure\")
              srcObj.PostMessage(me, \"AsyncQueryHelper_Internal\", \"SynchronouslyWaitForResults_Failure\", 0.f);
            else
              srcObj.PostMessage(me, \"AsyncQueryHelper_Internal\", \"SynchronouslyWaitForResults_AsyncResult\", 0.f);
        }
    };";
    let pairs = GameScriptParser::parse(Rule::program, src).unwrap();
    let program = process_trainz_ast(pairs, src);

    let class = program.classes.get("Test").unwrap();
    let method = &class.methods.get("Main").unwrap()[0];
    let stmts = &method.body.as_ref().unwrap().statements;

    // GameObject srcObj = ...; IF
    assert_eq!(stmts.len(), 2);

    if let crate::gs::Stmt::If(if_stmt) = &stmts[1] {
        // Condition: msg.minor == "Failure"
        // Binary(Eq, msg.minor, "Failure")
        assert_eq!(
            if_stmt.then_block.statements.len(),
            1,
            "Then block should have 1 statement"
        );
        match &if_stmt.then_block.statements[0] {
            crate::gs::Stmt::Expr(_) => {}
            _ => panic!(
                "Expected expression statement in then block, got {:?}",
                if_stmt.then_block.statements[0]
            ),
        }

        // Else block should have 1 statement
        assert!(if_stmt.else_block.is_some(), "Else block should be present");
        let else_block = if_stmt.else_block.as_ref().unwrap();
        assert_eq!(
            else_block.statements.len(),
            1,
            "Else block should have 1 statement"
        );
        match &else_block.statements[0] {
            crate::gs::Stmt::Expr(_) => {}
            _ => panic!(
                "Expected expression statement in else block, got {:?}",
                else_block.statements[0]
            ),
        }
    } else {
        panic!("Expected IF statement, got {:?}", stmts[1]);
    }
}

#[test]
fn test_if_single_statement_break_parsing() {
    let src = "class Test {
        void Main() {
            while (true) {
                if (true)
                    break;
            }
        }
    };";
    let pairs = GameScriptParser::parse(Rule::program, src).unwrap();
    let program = process_trainz_ast(pairs, src);

    let class = program.classes.get("Test").unwrap();
    let method = &class.methods.get("Main").unwrap()[0];
    let while_stmt = match &method.body.as_ref().unwrap().statements[0] {
        crate::gs::Stmt::While(w) => w,
        _ => panic!("Expected WHILE"),
    };

    let block = match &while_stmt.body {
        crate::gs::LoopBody::Block(b) => b,
        _ => panic!("Expected block"),
    };

    if let crate::gs::Stmt::If(if_stmt) = &block.statements[0] {
        assert_eq!(
            if_stmt.then_block.statements.len(),
            1,
            "Then block should have 1 statement (break)"
        );
        match &if_stmt.then_block.statements[0] {
            crate::gs::Stmt::Break(_, _) => {}
            _ => panic!(
                "Expected break statement, got {:?}",
                if_stmt.then_block.statements[0]
            ),
        }
    } else {
        panic!("Expected IF statement");
    }
}
