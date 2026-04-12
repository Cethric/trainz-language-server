use crate::gs::trainz_diagnostics;
use tower_lsp_server::ls_types::Diagnostic;
use trainz_ast::gs::process::process_trainz_ast;
use trainz_parser::gs::parse;

fn get_diagnostics(src: &str) -> Vec<Diagnostic> {
    let pairs = parse(src).expect("Should parse");
    let program = process_trainz_ast(pairs, src);
    trainz_diagnostics(&program, &program)
}

#[test]
fn test_chained_calls_valid() {
    let src = r#"
class Test {
    void Method() {}
    void Main() {
        me.Method();
    }
};
"#;
    let diagnostics = get_diagnostics(src);
    assert!(
        diagnostics.is_empty(),
        "Expected no diagnostics, got: {:?}",
        diagnostics
    );
}

#[test]
fn test_primitive_deref_error() {
    let src = r#"
class Test {
    void Main() {
        int i = 0;
        i.Method();
    }
};
"#;
    let diagnostics = get_diagnostics(src);
    assert!(
        !diagnostics.is_empty(),
        "Expected error for calling method on primitive 'int'"
    );
    assert!(diagnostics.iter().any(|d| {
        d.message
            .contains("Cannot dereference primitive type 'int'")
    }));
}

#[test]
fn test_member_not_found_error() {
    let src = r#"
class Test {
    void Main() {
        me.UnknownMethod();
    }
};
"#;
    let diagnostics = get_diagnostics(src);
    assert!(!diagnostics.is_empty(), "Expected error for unknown method");
    assert!(diagnostics.iter().any(|d| {
        d.message
            .contains("Member 'UnknownMethod' not found in class 'Test'")
    }));
}

#[test]
fn test_array_indexing() {
    let src = r#"
class Test {
    void Main() {
        int[] arr = new int[4];
        int val = arr[0];
        int fail = arr[5];
    }
};
"#;
    let diagnostics = get_diagnostics(src);
    assert!(
        !diagnostics.is_empty(),
        "Expected error for out of bounds index"
    );
    assert!(
        diagnostics
            .iter()
            .any(|d| d.message.contains("Array index out of range: 5 >= 4"))
    );
}

#[test]
fn test_string_array_behavior() {
    let src = r#"
class Test {
    void Main() {
        string s = "hello";
        string c = s[0];
        string sub = s[0, 2];
    }
};
"#;
    let diagnostics = get_diagnostics(src);
    assert!(
        diagnostics.is_empty(),
        "Expected no diagnostics for string indexing/ranging, got: {:?}",
        diagnostics
    );
}

#[test]
fn test_chained_methods() {
    let src = r#"
class Logger {
    Logger Log(string msg) { return me; }
};

class Test {
    void Main() {
        Logger l = new Logger();
        l.Log("a").Log("b").Log("c");
    }
};
"#;
    let diagnostics = get_diagnostics(src);
    assert!(
        diagnostics.is_empty(),
        "Expected no diagnostics for valid chained calls, got: {:?}",
        diagnostics
    );
}

#[test]
fn test_inheritance_lookup() {
    let src = r#"
class Base {
    void BaseMethod() {}
};

class Derived isclass Base {
    void Main() {
        me.BaseMethod();
    }
};
"#;
    let diagnostics = get_diagnostics(src);
    assert!(
        diagnostics.is_empty(),
        "Expected no diagnostics for inherited method call, got: {:?}",
        diagnostics
    );
}

#[test]
fn test_isclass_method() {
    let src = r#"
class Test {
    void Main() {
        bool b = me.isclass(Test);
    }
};
"#;
    let diagnostics = get_diagnostics(src);
    assert!(
        diagnostics.is_empty(),
        "Expected no diagnostics for isclass method, got: {:?}",
        diagnostics
    );
}

#[test]
fn test_return_type_mismatch() {
    let src = r#"
class Test {
    int GetInt() {
        return "not an int";
    }
    void NoReturn() {
        return 1;
    }
    string MissingReturn() {
        return;
    }
};
"#;
    let diagnostics = get_diagnostics(src);
    assert!(
        diagnostics.len() >= 3,
        "Expected at least 3 diagnostics for return mismatches, got: {:?}",
        diagnostics
    );
    assert!(diagnostics.iter().any(|d| {
        d.message
            .contains("Return type mismatch: expected 'int', got 'string'")
    }));
    assert!(diagnostics.iter().any(|d| {
        d.message
            .contains("Return type mismatch: expected void, got an expression")
    }));
    assert!(diagnostics.iter().any(|d| {
        d.message
            .contains("Return type mismatch: expected 'string', got nothing")
    }));
}

#[test]
fn test_assignment_type_mismatch() {
    let src = r#"
class Test {
    void Main() {
        int i = "not an int";
        string s = 1.0;
        bool b = 0;
    }
};
"#;
    let diagnostics = get_diagnostics(src);
    assert!(diagnostics.len() >= 3);
    assert!(diagnostics.iter().any(|d| {
        d.message
            .contains("Assignment type mismatch: cannot assign 'string' to 'int'")
    }));
    assert!(diagnostics.iter().any(|d| {
        d.message
            .contains("Assignment type mismatch: cannot assign 'float' to 'string'")
    }));
    assert!(diagnostics.iter().any(|d| {
        d.message
            .contains("Assignment type mismatch: cannot assign 'int' to 'bool'")
    }));
}

#[test]
fn test_method_call_validation() {
    let src = r#"
class Test {
    void TakeInt(int i) {}
    void Main() {
        TakeInt("not an int");
        TakeInt(1, 2);
    }
};
"#;
    let diagnostics = get_diagnostics(src);
    assert!(diagnostics.len() >= 2);
    assert!(diagnostics.iter().any(|d| {
        d.message
            .contains("Argument type mismatch: expected 'int', got 'string'")
    }));
    assert!(diagnostics.iter().any(|d| {
        d.message
            .contains("No overload of method takes 2 arguments")
    }));
}

#[test]
fn test_inheritance_assignment() {
    let src = r#"
class Base {};
class Derived isclass Base {};
class Other {};

class Test {
    void Main() {
        Base b = new Derived();
        Derived d = new Base();
        Base b2 = new Other();
    }
};
"#;
    let diagnostics = get_diagnostics(src);
    // Base b = new Derived(); should be OK
    // Derived d = new Base(); should be ERROR
    // Base b2 = new Other(); should be ERROR
    assert!(diagnostics.len() >= 2);
    assert!(diagnostics.iter().any(|d| {
        d.message
            .contains("Assignment type mismatch: cannot assign 'Base' to 'Derived'")
    }));
    assert!(diagnostics.iter().any(|d| {
        d.message
            .contains("Assignment type mismatch: cannot assign 'Other' to 'Base'")
    }));
}

#[test]
fn test_array_built_in_methods() {
    let src = r#"
class Test {
    int[] GetArr() { return new int[4]; }
    void Main() {
        int[] arr = new int[4];
        int s = arr.size();
        int[] c = arr.copy();
        int s2 = GetArr().size();
        int[] c2 = GetArr().copy();
    }
};
"#;
    let diagnostics = get_diagnostics(src);
    assert!(
        diagnostics.is_empty(),
        "Expected no diagnostics for array size() and copy(), got: {:?}",
        diagnostics
    );
}

#[test]
fn test_string_no_built_in_methods() {
    let src = r#"
class Test {
    void Main() {
        string s = "hello";
        int sz = s.size();
    }
};
"#;
    let diagnostics = get_diagnostics(src);
    assert!(!diagnostics.is_empty(), "Expected error for string.size()");
}
