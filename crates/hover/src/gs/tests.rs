use crate::gs::trainz_hover;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tower_lsp_server::ls_types::{HoverContents, Position};
use trainz_ast::gs::dependency_graph::ProgramResolver;
use trainz_ast::gs::process::process_trainz_ast;
use trainz_ast::gs::program::Program;
use trainz_ast::gs::type_eval::ClassResolver;
use trainz_parser::gs::parse;

#[test]
fn test_trainz_hover_array_methods() {
    let code = "class Test {
    void Main() {
        int[] arr = new int[4];
        arr.size();
        arr.copy();
    }
};";
    let pairs = parse(code).unwrap();
    let program = process_trainz_ast(pairs, code);

    // Hover over 'size' in 'arr.size()'
    let pos_size = Position {
        line: 3,
        character: 12,
    };
    let hover_size = trainz_hover::trainz_hover(&program, &program, &program, pos_size)
        .expect("Should find hover for size()");
    if let HoverContents::Markup(markup) = hover_size.contents {
        assert_eq!(markup.value, "int size()");
    }

    // Hover over 'copy' in 'arr.copy()'
    let pos_copy = Position {
        line: 4,
        character: 12,
    };
    let hover_copy = trainz_hover::trainz_hover(&program, &program, &program, pos_copy)
        .expect("Should find hover for copy()");
    if let HoverContents::Markup(markup) = hover_copy.contents {
        assert_eq!(markup.value, "int[] copy()");
    }
}

#[test]
fn test_trainz_hover_nested_array_methods() {
    let code = "class Test {
    void Main() {
        int[][] nested = new int[4][2];
        nested.copy();
        nested[0].copy();
    }
};";
    let pairs = parse(code).unwrap();
    let program = process_trainz_ast(pairs, code);

    // Hover over 'copy' in 'nested.copy()'
    let pos_copy1 = Position {
        line: 3,
        character: 15,
    };
    let hover_copy1 = trainz_hover::trainz_hover(&program, &program, &program, pos_copy1)
        .expect("Should find hover for nested.copy()");
    if let HoverContents::Markup(markup) = hover_copy1.contents {
        assert_eq!(markup.value, "int[][] copy()");
    }

    // Hover over 'copy' in 'nested[0].copy()'
    let pos_copy2 = Position {
        line: 4,
        character: 18,
    };
    let hover_copy2 = trainz_hover::trainz_hover(&program, &program, &program, pos_copy2)
        .expect("Should find hover for nested[0].copy()");
    if let HoverContents::Markup(markup) = hover_copy2.contents {
        assert_eq!(markup.value, "int[] copy()");
    }
}

#[test]
fn test_trainz_hover_string_methods() {
    let code = "class Test {
    void Main() {
        string s = \"test\";
        s.size();
    }
};";
    let pairs = parse(code).unwrap();
    let program = process_trainz_ast(pairs, code);

    // Hover over 'size' in 's.size()'
    let pos_size = Position {
        line: 3,
        character: 12,
    };
    let hover_size = trainz_hover::trainz_hover(&program, &program, &program, pos_size)
        .expect("Should find hover for string.size()");
    if let HoverContents::Markup(markup) = hover_size.contents {
        assert_eq!(markup.value, "int size()");
    }
}

#[test]
fn test_trainz_hover_array_indexing() {
    let code = "class Test {
    void Main() {
        int[] arr = new int[4];
        int x = arr[0];
    }
};";
    let pairs = parse(code).unwrap();
    let program = process_trainz_ast(pairs, code);

    // Hover over 'arr' in 'arr[0]'
    let pos_arr = Position {
        line: 3,
        character: 16,
    };
    let hover_arr = trainz_hover::trainz_hover(&program, &program, &program, pos_arr)
        .expect("Should find hover for arr in arr[0]");
    if let HoverContents::Markup(markup) = hover_arr.contents {
        assert_eq!(markup.value, "int[] arr");
    }
}

#[test]
fn test_trainz_hover_isclass() {
    let code = "class Foo {};
class Test {
    void Main() {
        Foo f = new Foo();
        f.isclass(null);
    }
};";
    let pairs = parse(code).unwrap();
    let program = process_trainz_ast(pairs, code);

    // Hover over 'isclass'
    let pos = Position {
        line: 4,
        character: 12,
    };
    let hover = trainz_hover::trainz_hover(&program, &program, &program, pos)
        .expect("Should find hover for isclass()");
    if let HoverContents::Markup(markup) = hover.contents {
        assert_eq!(markup.value, "bool isclass(object cls)");
    }
}

#[test]
fn test_trainz_hover_declaration() {
    let code = "class Test {
  public bool AlreadyThereStr(string[] strArray, string searchStr)
  {
    int i;
    for (i = 0; i < 10; i++)
      if (searchStr == strArray[i])
        return true;

    return false;
  }
};";
    let pairs = parse(code).unwrap();
    let program = process_trainz_ast(pairs, code);

    // Hover over 'i' in 'i = 0'
    let pos_i = Position {
        line: 4,
        character: 9,
    };
    let hover_i = trainz_hover::trainz_hover(&program, &program, &program, pos_i).unwrap();
    if let HoverContents::Markup(markup) = hover_i.contents {
        assert_eq!(markup.value, "int i");
    }

    // Hover over 'searchStr' in 'searchStr =='
    let pos_search = Position {
        line: 5,
        character: 10,
    };
    let hover_search =
        trainz_hover::trainz_hover(&program, &program, &program, pos_search).unwrap();
    if let HoverContents::Markup(markup) = hover_search.contents {
        assert_eq!(markup.value, "parameter: string searchStr");
    }
}

#[test]
fn test_trainz_hover_fields() {
    let code = "class Test {
  int m_queryResult = 0;
  define int ERROR_INVALID_STATE = 2;

  public int GetQueryErrorCode() { return m_queryResult; }
  public int GetError() { return ERROR_INVALID_STATE; }
};";
    let pairs = parse(code).unwrap();
    let program = process_trainz_ast(pairs, code);

    // Hover over 'm_queryResult' in 'GetQueryErrorCode'
    let pos_field = Position {
        line: 4,
        character: 42,
    };
    let hover_field = trainz_hover::trainz_hover(&program, &program, &program, pos_field)
        .expect("Should find hover for m_queryResult");
    if let HoverContents::Markup(markup) = hover_field.contents {
        assert_eq!(markup.value, "field: int Test::m_queryResult");
    }

    // Hover over 'ERROR_INVALID_STATE' in 'GetError'
    let pos_define = Position {
        line: 5,
        character: 33,
    };
    let hover_define = trainz_hover::trainz_hover(&program, &program, &program, pos_define)
        .expect("Should find hover for ERROR_INVALID_STATE");
    if let HoverContents::Markup(markup) = hover_define.contents {
        assert_eq!(markup.value, "int Test::ERROR_INVALID_STATE = 2");
    }
}

#[test]
fn test_trainz_hover_method() {
    let code = "class Test {
  public void CountTags(int pid, string s) { }
  void Run() {
    CountTags(1, \"test\");
  }
};";
    let pairs = parse(code).unwrap();
    let program = process_trainz_ast(pairs, code);

    // Hover over 'CountTags' in 'Run'
    let pos_call = Position {
        line: 3,
        character: 4,
    };
    let hover_call = trainz_hover::trainz_hover(&program, &program, &program, pos_call)
        .expect("Should find hover for method call");
    if let HoverContents::Markup(markup) = hover_call.contents {
        assert_eq!(
            markup.value,
            "public void Test::CountTags(int pid, string s)"
        );
    }

    let pos_def = Position {
        line: 1,
        character: 14,
    };
    let hover_def = trainz_hover::trainz_hover(&program, &program, &program, pos_def)
        .expect("Should find hover for method definition");
    if let HoverContents::Markup(markup) = hover_def.contents {
        assert_eq!(
            markup.value,
            "public void Test::CountTags(int pid, string s)"
        );
    }
}

#[test]
fn test_trainz_hover_static_method() {
    let code = "class Router {
    static GameObject GetCurrentThreadGameObject() { return null; }
};

class Test {
    void Main() {
        GameObject g = Router.GetCurrentThreadGameObject();
    }
};";
    let pairs = parse(code).unwrap();
    let program = process_trainz_ast(pairs, code);

    // Hover over 'GetCurrentThreadGameObject' in 'Main'
    let pos_call = Position {
        line: 6,
        character: 35,
    };
    let hover_call = trainz_hover::trainz_hover(&program, &program, &program, pos_call)
        .expect("Should find hover for static method call");
    if let HoverContents::Markup(markup) = hover_call.contents {
        assert_eq!(
            markup.value,
            "static GameObject Router::GetCurrentThreadGameObject()"
        );
    }
}

#[test]
fn test_trainz_hover_inheritance() {
    let code = "class Base {
    public int baseField = 0;
    public void BaseMethod(int a) {}
};
class Derived isclass Base {
    void Main() {
        BaseMethod(1);
        int x = baseField;
    }
};";
    let pairs = parse(code).unwrap();
    let program = process_trainz_ast(pairs, code);

    // Hover over 'BaseMethod' call in 'Derived::Main'
    let pos_method = Position {
        line: 6,
        character: 8,
    };
    let hover_method = trainz_hover::trainz_hover(&program, &program, &program, pos_method)
        .expect("Should find hover for inherited method");
    if let HoverContents::Markup(markup) = hover_method.contents {
        assert_eq!(markup.value, "public void Base::BaseMethod(int a)");
    }

    // Hover over 'baseField' in 'Derived::Main'
    let pos_field = Position {
        line: 7,
        character: 16,
    };
    let hover_field = trainz_hover::trainz_hover(&program, &program, &program, pos_field)
        .expect("Should find hover for inherited field");
    if let HoverContents::Markup(markup) = hover_field.contents {
        assert_eq!(markup.value, "field: int Base::baseField");
    }
}

#[test]
fn test_trainz_hover_chained_calls() {
    let code = "class Logger {
    public Logger Log(string msg) { return me; }
    public int Count = 0;
};
class Test {
    void Main() {
        Logger l = new Logger();
        l.Log(\"a\").Log(\"b\").Count;
    }
};";
    let pairs = parse(code).unwrap();
    let program = process_trainz_ast(pairs, code);

    // Hover over second 'Log' in 'l.Log(\"a\").Log(\"b\")'
    let pos_log2 = Position {
        line: 7,
        character: 21,
    };
    let hover_log2 = trainz_hover::trainz_hover(&program, &program, &program, pos_log2)
        .expect("Should find hover for second Log()");
    if let HoverContents::Markup(markup) = hover_log2.contents {
        assert_eq!(markup.value, "public Logger Logger::Log(string msg)");
    }

    // Hover over 'Count' at the end of the chain
    let pos_count = Position {
        line: 7,
        character: 29,
    };
    let hover_count = trainz_hover::trainz_hover(&program, &program, &program, pos_count)
        .expect("Should find hover for Count field");
    if let HoverContents::Markup(markup) = hover_count.contents {
        assert_eq!(markup.value, "field: int Logger::Count");
    }
}

#[test]
fn test_trainz_hover_class_name() {
    let code = "class Foo {};
class Test {
    void Main() {
        Foo f = new Foo();
    }
};";
    let pairs = parse(code).unwrap();
    let program = process_trainz_ast(pairs, code);

    // Hover over 'Foo' in 'Foo f'
    let pos_decl = Position {
        line: 3,
        character: 8,
    };
    let hover_decl = trainz_hover::trainz_hover(&program, &program, &program, pos_decl)
        .expect("Should find hover for class name in declaration");
    if let HoverContents::Markup(markup) = hover_decl.contents {
        assert_eq!(markup.value, "class Foo");
    }

    // Hover over 'Foo' in 'new Foo()'
    let pos_new = Position {
        line: 3,
        character: 20,
    };
    let hover_new = trainz_hover::trainz_hover(&program, &program, &program, pos_new)
        .expect("Should find hover for class name in new expression");
    if let HoverContents::Markup(markup) = hover_new.contents {
        assert_eq!(markup.value, "class Foo");
    }

    // Hover over 'Base' in 'class Derived isclass Base'
    let code2 = "class Base {}; class Derived isclass Base {};";
    let pairs2 = parse(code2).unwrap();
    let program2 = process_trainz_ast(pairs2, code2);
    let pos_base = Position {
        line: 0,
        character: 38,
    };
    let hover_base = trainz_hover::trainz_hover(&program2, &program2, &program2, pos_base)
        .expect("Should find hover for superclass name");
    if let HoverContents::Markup(markup) = hover_base.contents {
        assert_eq!(markup.value, "class Base");
    }
}

#[test]
fn test_trainz_hover_isclass_standalone() {
    let code = "class Foo {};
class Test {
    void Main() {
        isclass(Foo);
    }
};";
    let pairs = parse(code).unwrap();
    let program = process_trainz_ast(pairs, code);

    // Hover over 'isclass'
    let pos = Position {
        line: 3,
        character: 8,
    };
    // This currently fails at find_id_at_position if we don't fix it
    let hover = trainz_hover::trainz_hover(&program, &program, &program, pos);
    assert!(
        hover.is_some(),
        "Should find hover for standalone isclass()"
    );
    if let Some(h) = hover
        && let HoverContents::Markup(markup) = h.contents
    {
        assert_eq!(markup.value, "bool isclass(object cls)");
    }
}

#[test]
fn test_trainz_hover_array_type_declaration() {
    let code = "class Foo {};
class Test {
    void Main() {
        Foo[] arr = new Foo[10];
    }
};";
    let pairs = parse(code).unwrap();
    let program = process_trainz_ast(pairs, code);

    // Hover over 'Foo' in 'Foo[] arr'
    let pos_decl = Position {
        line: 3,
        character: 8,
    };
    let hover_decl = trainz_hover::trainz_hover(&program, &program, &program, pos_decl);
    assert!(
        hover_decl.is_some(),
        "Should find hover for class name in array declaration"
    );
    if let Some(h) = hover_decl
        && let HoverContents::Markup(markup) = h.contents
    {
        assert_eq!(markup.value, "class Foo");
    }

    // Hover over 'Foo' in 'new Foo[10]'
    let pos_new = Position {
        line: 3,
        character: 24,
    };
    let hover_new = trainz_hover::trainz_hover(&program, &program, &program, pos_new);
    assert!(
        hover_new.is_some(),
        "Should find hover for class name in new array expression"
    );
    if let Some(h) = hover_new
        && let HoverContents::Markup(markup) = h.contents
    {
        assert_eq!(markup.value, "class Foo");
    }
}

#[test]
fn test_trainz_hover_field_array_type() {
    let code = "class Foo {};
class Test {
    Foo[] arr;
};";
    let pairs = parse(code).unwrap();
    let program = process_trainz_ast(pairs, code);

    // Hover over 'Foo' in 'Foo[] arr'
    let pos = Position {
        line: 2,
        character: 5,
    };
    let hover = trainz_hover::trainz_hover(&program, &program, &program, pos);
    assert!(
        hover.is_some(),
        "Should find hover for class name in field array declaration"
    );
    if let Some(h) = hover
        && let HoverContents::Markup(markup) = h.contents
    {
        assert_eq!(markup.value, "class Foo");
    }
}

#[test]
fn test_trainz_hover_method_types() {
    let code = "class Foo {};
class Test {
    Foo Method(Foo param) { return new Foo(); }
};";
    let pairs = parse(code).unwrap();
    let program = process_trainz_ast(pairs, code);

    // Hover over 'Foo' in return type
    let pos_ret = Position {
        line: 2,
        character: 4,
    };
    let hover_ret = trainz_hover::trainz_hover(&program, &program, &program, pos_ret);
    assert!(
        hover_ret.is_some(),
        "Should find hover for class name in return type"
    );
    if let Some(h) = hover_ret
        && let HoverContents::Markup(markup) = h.contents
    {
        assert_eq!(markup.value, "class Foo");
    }

    // Hover over 'Foo' in parameter type
    let pos_param = Position {
        line: 2,
        character: 15,
    };
    let hover_param = trainz_hover::trainz_hover(&program, &program, &program, pos_param);
    assert!(
        hover_param.is_some(),
        "Should find hover for class name in parameter type"
    );
    if let Some(h) = hover_param
        && let HoverContents::Markup(markup) = h.contents
    {
        assert_eq!(markup.value, "class Foo");
    }
}

#[test]
fn test_trainz_hover_me_keyword() {
    let code = "class Test {
    void Main() {
        me.Main();
    }
};";
    let pairs = parse(code).unwrap();
    let program = process_trainz_ast(pairs, code);

    // Hover over 'me'
    let pos = Position {
        line: 2,
        character: 8,
    };
    let hover = trainz_hover::trainz_hover(&program, &program, &program, pos)
        .expect("Should find hover for me keyword");
    if let HoverContents::Markup(markup) = hover.contents {
        assert_eq!(markup.value, "class Test");
    }
}

#[test]
fn test_trainz_hover_void_parameter() {
    let code = "class Test {
    void FiremanWave(void) { }
};";
    let pairs = parse(code).unwrap();
    let program = process_trainz_ast(pairs, code);

    // Hover over 'FiremanWave'
    let pos = Position {
        line: 1,
        character: 14,
    };
    let hover = trainz_hover::trainz_hover(&program, &program, &program, pos)
        .expect("Should find hover for FiremanWave");
    if let HoverContents::Markup(markup) = hover.contents {
        assert_eq!(markup.value, "void Test::FiremanWave(void)");
    }
}

#[test]
fn test_trainz_hover_include() {
    let code_a = "class A { void MethodA() {} }; include \"b.gs\"";
    let code_b = "class B { void MethodB() {} };";
    let code_main = "include \"a.gs\"";

    // Setup mock resolver
    struct MockResolver {
        programs: HashMap<String, Arc<Program>>,
    }
    impl ClassResolver for MockResolver {
        fn find_class(&self, name: &str) -> Option<trainz_ast::gs::ClassDef> {
            for p in self.programs.values() {
                if let Some(cls) = p.classes.get(name) {
                    return Some(cls.clone());
                }
            }
            None
        }
    }
    impl ProgramResolver for MockResolver {
        fn resolve_program(&self, path: &str) -> Option<Arc<Program>> {
            self.programs.get(path).cloned()
        }
    }

    let pairs_a = parse(code_a).unwrap();
    let mut program_a_val = process_trainz_ast(pairs_a, code_a);
    program_a_val.includes[0].path = Some(PathBuf::from("b.gs"));
    let program_a = Arc::new(program_a_val);

    let pairs_b = parse(code_b).unwrap();
    let program_b = Arc::new(process_trainz_ast(pairs_b, code_b));

    let pairs_main = parse(code_main).unwrap();
    let mut program_main = process_trainz_ast(pairs_main, code_main);
    program_main.includes[0].path = Some(PathBuf::from("a.gs"));

    let mut programs = HashMap::new();
    programs.insert("a.gs".to_string(), program_a);
    programs.insert("b.gs".to_string(), program_b);

    let resolver = MockResolver { programs };

    let pos = Position {
        line: 0,
        character: 5,
    }; // Over "include"
    let hover = trainz_hover::trainz_hover(&program_main, &resolver, &resolver, pos)
        .expect("Should find hover for include");

    if let HoverContents::Markup(markup) = hover.contents {
        assert!(markup.value.contains("Includes classes:"));
        assert!(markup.value.contains("- A"));
        assert!(markup.value.contains("- B"));
    }
}
