use crate::gs::trainz_diagnostics;
use tower_lsp_server::ls_types::Diagnostic;
use trainz_ast::gs::process::process_trainz_ast;
use trainz_parser::gs::parse;

fn get_diagnostics(src: &str) -> Vec<Diagnostic> {
    let pairs = parse(src).expect("Should parse");
    let program = process_trainz_ast(pairs, src);

    struct EmptyProgramResolver;
    impl trainz_ast::gs::dependency_graph::ProgramResolver for EmptyProgramResolver {
        fn resolve_program(&self, _path: &str) -> Option<std::sync::Arc<trainz_ast::gs::Program>> {
            None
        }
    }

    trainz_diagnostics("test.gs", &program, &program, &EmptyProgramResolver)
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
        Test t = 1;
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
            .contains("Assignment type mismatch: cannot assign 'int' to 'Test'")
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
            .contains("No matching overload of method takes 1 arguments")
    }));
    assert!(diagnostics.iter().any(|d| {
        d.message
            .contains("No matching overload of method takes 2 arguments")
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
fn test_string_built_in_methods() {
    let src = r#"
class Test {
    void Main() {
        string s = "hello";
        int sz = s.size();
    }
};
"#;
    let diagnostics = get_diagnostics(src);
    assert!(
        diagnostics.is_empty(),
        "Expected no diagnostics for string.size(), got: {:?}",
        diagnostics
    );
}

#[test]
fn test_me_deep_inheritance() {
    let src = r#"
        class GrandBase {
            void GrandMethod() {}
        };
        class Base isclass GrandBase {
            void BaseMethod() {}
        };
        class Derived isclass Base {
            void Main() {
                me.GrandMethod();
            }
        };
    "#;
    let diagnostics = get_diagnostics(src);
    assert!(
        diagnostics.is_empty(),
        "Expected no diagnostics for deep inherited method call, got: {:?}",
        diagnostics
    );
}

#[test]
fn test_overload_validation() {
    let src = r#"
class Base {
    void Foo(int i) {}
};

class Derived isclass Base {
    void Foo(string s) {}
    
    void Test() {
        Foo(1);          // OK: matches Base::Foo(int)
        Foo("hello");    // OK: matches Derived::Foo(string)
        Foo(1.5);        // OK: matches Base::Foo(int) with warning
    }
};
"#;
    let diagnostics = get_diagnostics(src);
    // Foo(1.5) should have a precision warning
    assert!(
        diagnostics
            .iter()
            .any(|d| d.message.contains("Implicit cast from 'float' to 'int'")),
        "Expected precision warning, got: {:?}",
        diagnostics
    );
}

#[test]
fn test_inherited_validation() {
    let src = r#"
class Base {
    void Test(int i) {}
};

class Derived isclass Base {
    void Test(int i) {
        inherited(i);    // OK: matches Base::Test(int)
        inherited("hi"); // Warning: Base::Test takes int, not string
    }
    
    void Bar() {
        inherited(); // Error: Bar not in any parent
    }
};
"#;
    let diagnostics = get_diagnostics(src);
    assert!(
        diagnostics
            .iter()
            .any(|d| d.message.contains("not defined in any inherited class"))
    );
    assert!(diagnostics.iter().any(|d| {
        d.message
            .contains("Arguments not compatible with 'inherited' method in class 'Base'")
    }));
}

#[test]
fn test_multiple_inheritance_inherited() {
    let src = r#"
        class A { public void Foo(int x) {} };
        class B { public void Foo(string s) {} };
        class Other {};
        class C isclass A, B {
            public void Foo(int x) {
                inherited(x);    // ERROR for B (closest match Foo(string))
                inherited("hi"); // ERROR for A (closest match Foo(int))
                inherited(new Other()); // ERROR for both
            }
            public void Bar() {
                inherited(); // ERROR: Bar not in A or B
            }
        };
    "#;
    let diagnostics = get_diagnostics(src);
    // 1. inherited(x)
    assert!(diagnostics.iter().any(|d| {
        d.message
            .contains("Arguments not compatible with 'inherited'")
            && d.message.contains("class 'B'")
    }));
    // 2. inherited("hi")
    assert!(diagnostics.iter().any(|d| {
        d.message
            .contains("Arguments not compatible with 'inherited'")
            && d.message.contains("class 'A'")
    }));
    // 3. inherited(new Other())
    assert!(diagnostics.iter().any(|d| {
        d.message
            .contains("Arguments not compatible with 'inherited'")
            && d.message.contains("class 'A'")
    }));
    assert!(diagnostics.iter().any(|d| {
        d.message
            .contains("Arguments not compatible with 'inherited'")
            && d.message.contains("class 'B'")
    }));
    // 4. Bar() -> inherited()
    assert!(diagnostics.iter().any(|d| {
        d.message
            .contains("Method 'Bar' not defined in any inherited class")
    }));
}

#[test]
fn test_me_overload_inheritance() {
    let src = r#"
        class A {
            void foo() {}
        };
        class B isclass A {
            void foo(int x) {}
        };
        class C isclass B {
            void Main() {
                me.foo();
            }
        };
    "#;
    let diagnostics = get_diagnostics(src);
    assert!(
        diagnostics.is_empty(),
        "Expected no diagnostics for inherited overload call, got: {:?}",
        diagnostics
    );
}

#[test]
fn test_cyclic_include_warning() {
    use crate::gs::trainz_diagnostics;
    use std::collections::HashMap;
    use std::sync::Arc;
    use trainz_ast::gs::ClassDef;
    use trainz_ast::gs::Program;
    use trainz_ast::gs::dependency_graph::ProgramResolver;
    use trainz_ast::gs::type_eval::ClassResolver;

    let src_a = "include \"b.gs\"\nclass A {};";
    let src_b = "include \"a.gs\"\nclass B {};";

    let program_a = Arc::new(trainz_ast::gs::process::process_trainz_ast(
        trainz_parser::gs::parse(src_a).unwrap(),
        src_a,
    ));
    let program_b = Arc::new(trainz_ast::gs::process::process_trainz_ast(
        trainz_parser::gs::parse(src_b).unwrap(),
        src_b,
    ));

    // Manually set resolved paths
    let mut program_a_val = (*program_a).clone();
    program_a_val.includes[0].path = Some("b.gs".into());
    let program_a = Arc::new(program_a_val);

    let mut program_b_val = (*program_b).clone();
    program_b_val.includes[0].path = Some("a.gs".into());
    let program_b = Arc::new(program_b_val);

    struct TestResolver {
        programs: HashMap<String, Arc<Program>>,
    }
    impl ProgramResolver for TestResolver {
        fn resolve_program(&self, path: &str) -> Option<Arc<Program>> {
            self.programs.get(path).cloned()
        }
    }
    impl ClassResolver for TestResolver {
        fn find_class(&self, _name: &str) -> Option<ClassDef> {
            None
        }
    }

    let mut programs = HashMap::new();
    programs.insert("a.gs".to_string(), program_a.clone());
    programs.insert("b.gs".to_string(), program_b.clone());
    let resolver = TestResolver { programs };

    let diagnostics = trainz_diagnostics("a.gs", &program_a, &resolver, &resolver);

    assert!(
        diagnostics
            .iter()
            .any(|d| d.message.contains("Cyclic include")),
        "Expected cyclic include warning, got: {:?}",
        diagnostics
    );
}

#[test]
fn test_gs_type_system_rules() {
    let src = r#"
class Test {
    void Main() {
        object o = me; // everything inherits from object
        object o2 = 1; // int inherits from object
        int i; // implicitly assigned to null
        float f = 1; // int can be implicitly cast to float
        int i2 = 1.5; // float can be implicitly cast to int (warning)
        
        if (i2) { // int can be implicitly cast to bool
        }
        
        bool b = i2 & i; // & result is bool
        bool b2 = i2 | i; // | result is bool
        bool b3 = ~i; // ~ result is bool
        bool b4 = !i; // ! result is bool
    }
};"#;
    let diagnostics = get_diagnostics(src);

    // Check for float to int warning
    assert!(
        diagnostics
            .iter()
            .any(|d| d.message.contains("Implicit cast from 'float' to 'int'")),
        "Expected warning for float to int cast, got: {:?}",
        diagnostics
    );

    // Ensure no errors for other rules
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Some(tower_lsp_server::ls_types::DiagnosticSeverity::ERROR))
        .collect();
    assert!(
        errors.is_empty(),
        "Expected no errors, but found: {:?}",
        errors
    );
}

#[test]
fn test_null_assignment_to_all_types() {
    let src = r#"
class Test {
    void Main() {
        int i = null;
        float f = null;
        bool b = null;
        string s = null;
        object o = null;
        Test t = null;
        int[] arr = null;
    }
};"#;
    let diagnostics = get_diagnostics(src);
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Some(tower_lsp_server::ls_types::DiagnosticSeverity::ERROR))
        .collect();
    assert!(
        errors.is_empty(),
        "Expected no errors for null assignment, but found: {:?}",
        errors
    );
}

#[test]
fn test_object_as_condition() {
    let src = r#"
class Test {
    void Main() {
        Test t = null;
        if (t) { }
        object o = null;
        if (o) { }
        string s = null;
        if (s) { }
        int[] arr = null;
        if (arr) { }
    }
};"#;
    let diagnostics = get_diagnostics(src);
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Some(tower_lsp_server::ls_types::DiagnosticSeverity::ERROR))
        .collect();
    assert!(
        errors.is_empty(),
        "Expected no errors for object as condition, but found: {:?}",
        errors
    );
}

#[test]
fn test_null_comprehensive() {
    let src = r#"
class Test {
    define int MAX = null;
    int i;
    string s;
    Test t;

    void SetValues(int i, string s, Test t) {
        me.i = i;
        me.s = s;
        me.t = t;
    }

    Test GetTest() {
        return null;
    }

    void Main() {
        SetValues(null, null, null);
        Test other = GetTest();
        if (other == null) { }
        if (null == other) { }
        bool b = (null == null);
        
        int[] arr;
        arr = null;
        int[] arr2 = new int[4];
        arr2[0] = null;
        
        bool b2 = null.isclass(Test);
    }
};"#;
    let diagnostics = get_diagnostics(src);
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Some(tower_lsp_server::ls_types::DiagnosticSeverity::ERROR))
        .collect();
    assert!(
        errors.is_empty(),
        "Expected no errors for comprehensive null usage, but found: {:?}",
        errors
    );
}

#[test]
fn test_null_invalid_usage() {
    let src = r#"
class Test {
    void Main() {
        null.UnknownMethod();
        int i = null[0];
    }
};"#;
    let diagnostics = get_diagnostics(src);
    assert!(diagnostics.len() >= 2);
    assert!(
        diagnostics
            .iter()
            .any(|d| { d.message.contains("Cannot dereference generic 'object'") })
    );
    assert!(
        diagnostics
            .iter()
            .any(|d| { d.message.contains("Expression is not an array or string") })
    );
}

#[test]
fn test_bitwise_operators() {
    let src = r#"
class Test {
    void Main() {
        int a = 1;
        int b = 2;
        int c = a | b;
        int d = a & b;
        int e = a ^ b;
        int f = a << 1;
        int g = b >> 1;
        int h = ~a;
        
        float fl = 1.0;
        int i = a | fl;
        int j = fl & a;
        int k = ~fl;
        int l = a << fl;
        int m = fl >> b;
    }
};"#;
    let diagnostics = get_diagnostics(src);
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Some(tower_lsp_server::ls_types::DiagnosticSeverity::ERROR))
        .collect();

    assert_eq!(errors.len(), 5);
    assert!(errors.iter().any(|d| {
        d.message
            .contains("Bitwise operator requires 'int' operand, got 'float'")
    }));
    assert!(errors.iter().any(|d| {
        d.message
            .contains("Bit shift operator requires 'int' operand, got 'float'")
    }));
    assert!(errors.iter().any(|d| {
        d.message
            .contains("Bitwise NOT operator requires 'int' operand, got 'float'")
    }));
}

#[test]
fn test_bit_shift_operators() {
    let src = r#"
class Test {
    void Main() {
        int a = 1;
        int b = a << 2;
        int c = a >> 1;
        int d = 1 << a;
        int e = 100 >> 2;
        
        float f = 1.0;
        string s = "test";
        bool bl = true;
        object o = null;
        
        int err1 = a << f;
        int err2 = f >> a;
        int err3 = s << 1;
        int err4 = 1 >> s;
        int err5 = bl << 1;
        int err6 = 1 >> bl;
        int err7 = o << 1;
        int err8 = 1 >> o;
    }
};"#;
    let diagnostics = get_diagnostics(src);
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Some(tower_lsp_server::ls_types::DiagnosticSeverity::ERROR))
        .collect();

    assert_eq!(errors.len(), 8);
    for error in errors {
        assert!(
            error
                .message
                .contains("Bit shift operator requires 'int' operand")
        );
    }
}
