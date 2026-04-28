use crate::gs::definitions::*;
use dashmap::DashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use tower_lsp_server::ls_types::{GotoDefinitionResponse, Location, Range, Uri};
use trainz_ast::Position;
use trainz_ast::gs::Program;
use trainz_parser::gs::parse;

#[test]
fn test_gs_goto_definition() {
    let _ = trainz_common::logging::tracing_subscriber::fmt()
        .with_test_writer()
        .try_init();
    let source = "class MyClass { void MyMethod() { MyMethod(); } };";
    let pairs = parse(source).unwrap();
    let program = Arc::new(trainz_ast::gs::process::process_trainz_ast(pairs, source));
    let uri = Uri::from_str("file:///test.gs").unwrap();
    let position = Position {
        line: 0,
        character: 35,
    };
    let parsed_files = DashMap::new();
    parsed_files.insert(uri.to_string(), program.clone());

    let result = gs_goto_definition(program, position, uri, &parsed_files);
    assert!(result.is_some());
    if let Some(GotoDefinitionResponse::Array(locations)) = result {
        assert_eq!(locations.len(), 1);
        assert_eq!(locations[0].range.start.character, 21);
    }
}

#[test]
fn test_gs_goto_definition_superclass() {
    let _ = trainz_common::logging::tracing_subscriber::fmt()
        .with_test_writer()
        .try_init();
    let super_uri = "file:///super.gs";
    let sub_uri = "file:///sub.gs";

    let super_source = "class SuperClass { void MyMethod() {} };";
    let super_pairs = parse(super_source).unwrap();
    let super_program = Arc::new(trainz_ast::gs::process::process_trainz_ast(
        super_pairs,
        super_source,
    ));

    let sub_source = "include \"super.gs\"\nclass SubClass isclass SuperClass { void AnotherMethod() { MyMethod(); } };";
    let sub_pairs = parse(sub_source).unwrap();
    let mut sub_program = trainz_ast::gs::process::process_trainz_ast(sub_pairs, sub_source);
    sub_program.includes[0].path = Some(super_uri.into());
    let sub_program = Arc::new(sub_program);

    let parsed_files = DashMap::new();
    parsed_files.insert(super_uri.to_string(), super_program);
    parsed_files.insert(sub_uri.to_string(), sub_program.clone());

    let position = Position {
        line: 1,
        character: 60,
    }; // Middle of "MyMethod"

    let result = gs_goto_definition(
        sub_program,
        position,
        Uri::from_str(sub_uri).unwrap(),
        &parsed_files,
    );
    assert!(result.is_some(), "Should find definition in superclass");
    if let Some(GotoDefinitionResponse::Link(links)) = result {
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target_uri.to_string(), super_uri);
    } else {
        panic!("Expected Link, got {:?}", result);
    }
}

#[test]
fn test_gs_goto_definition_superclass_chain() {
    let _ = trainz_common::logging::tracing_subscriber::fmt()
        .with_test_writer()
        .try_init();
    let gp_uri = "file:///gp.gs";
    let p_uri = "file:///p.gs";
    let c_uri = "file:///c.gs";

    let gp_source = "class GrandParent { void GPMethod() {} };";
    let gp_pairs = parse(gp_source).unwrap();
    let gp_program = Arc::new(trainz_ast::gs::process::process_trainz_ast(
        gp_pairs, gp_source,
    ));

    let p_source = "include \"gp.gs\"\nclass Parent isclass GrandParent { };";
    let p_pairs = parse(p_source).unwrap();
    let mut p_program = trainz_ast::gs::process::process_trainz_ast(p_pairs, p_source);
    p_program.includes[0].path = Some(gp_uri.into());
    let p_program = Arc::new(p_program);

    let c_source =
        "include \"p.gs\"\nclass Child isclass Parent { void ChildMethod() { GPMethod(); } };";
    let c_pairs = parse(c_source).unwrap();
    let mut c_program = trainz_ast::gs::process::process_trainz_ast(c_pairs, c_source);
    c_program.includes[0].path = Some(p_uri.into());
    let c_program = Arc::new(c_program);

    let parsed_files = DashMap::new();
    parsed_files.insert(gp_uri.to_string(), gp_program);
    parsed_files.insert(p_uri.to_string(), p_program);
    parsed_files.insert(c_uri.to_string(), c_program.clone());

    let position = Position {
        line: 1,
        character: 52,
    }; // Middle of "GPMethod"

    let result = gs_goto_definition(
        c_program,
        position,
        Uri::from_str(c_uri).unwrap(),
        &parsed_files,
    );
    assert!(result.is_some(), "Should find definition in GrandParent");
    if let Some(GotoDefinitionResponse::Link(links)) = result {
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target_uri.to_string(), gp_uri);
    } else {
        panic!("Expected Link, got {:?}", result);
    }
}

#[test]
fn test_gs_goto_definition_superclass_member_chain() {
    let _ = trainz_common::logging::tracing_subscriber::fmt()
        .with_test_writer()
        .try_init();
    let gp_uri = "file:///gp.gs";
    let p_uri = "file:///p.gs";
    let c_uri = "file:///c.gs";

    let gp_source = "class GrandParent { int gp_member; };";
    let gp_pairs = parse(gp_source).unwrap();
    let gp_program = Arc::new(trainz_ast::gs::process::process_trainz_ast(
        gp_pairs, gp_source,
    ));

    let p_source = "include \"gp.gs\"\nclass Parent isclass GrandParent { };";
    let p_pairs = parse(p_source).unwrap();
    let mut p_program = trainz_ast::gs::process::process_trainz_ast(p_pairs, p_source);
    p_program.includes[0].path = Some(gp_uri.into());
    let p_program = Arc::new(p_program);

    let c_source =
        "include \"p.gs\"\nclass Child isclass Parent { void ChildMethod() { gp_member = 1; } };";
    let c_pairs = parse(c_source).unwrap();
    let mut c_program = trainz_ast::gs::process::process_trainz_ast(c_pairs, c_source);
    c_program.includes[0].path = Some(p_uri.into());
    let c_program = Arc::new(c_program);

    let parsed_files = DashMap::new();
    parsed_files.insert(gp_uri.to_string(), gp_program);
    parsed_files.insert(p_uri.to_string(), p_program);
    parsed_files.insert(c_uri.to_string(), c_program.clone());

    let position = Position {
        line: 1,
        character: 52,
    }; // Middle of "gp_member"

    let result = gs_goto_definition(
        c_program,
        position,
        Uri::from_str(c_uri).unwrap(),
        &parsed_files,
    );
    assert!(result.is_some(), "Should find gp_member in GrandParent");
    if let Some(GotoDefinitionResponse::Link(links)) = result {
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target_uri.to_string(), gp_uri);
    } else {
        panic!("Expected Link, got {:?}", result);
    }
}

#[test]
fn test_gs_goto_definition_superclass_name() {
    let _ = trainz_common::logging::tracing_subscriber::fmt()
        .with_test_writer()
        .try_init();
    let source = "class Base { }; class Derived isclass Base { };";
    let pairs = parse(source).unwrap();
    let program = Arc::new(trainz_ast::gs::process::process_trainz_ast(pairs, source));
    let uri = Uri::from_str("file:///test.gs").unwrap();
    let position = Position {
        line: 0,
        character: 40,
    }; // On "Base" in "isclass Base"
    let parsed_files = DashMap::new();
    parsed_files.insert(uri.to_string(), program.clone());

    let result = gs_goto_definition(program, position, uri, &parsed_files);
    assert!(result.is_some());
    if let Some(GotoDefinitionResponse::Array(locations)) = result {
        assert_eq!(locations.len(), 1);
        assert_eq!(locations[0].range.start.character, 6);
    }
}

#[test]
fn test_gs_goto_definition_inherited() {
    let _ = trainz_common::logging::tracing_subscriber::fmt()
        .with_test_writer()
        .try_init();
    let super_uri = "file:///super.gs";
    let sub_uri = "file:///sub.gs";

    let super_source = "class SuperClass { void MyMethod() {} };";
    let super_pairs = parse(super_source).unwrap();
    let super_program = Arc::new(trainz_ast::gs::process::process_trainz_ast(
        super_pairs,
        super_source,
    ));

    let sub_source = "class SubClass isclass SuperClass { void MyMethod() { inherited(); } };";
    // 01234567890123456789012345678901234567890123456789012345678901234567890
    //                                                      ^ 54
    let sub_pairs = parse(sub_source).unwrap();
    let sub_program = Arc::new(trainz_ast::gs::process::process_trainz_ast(
        sub_pairs, sub_source,
    ));

    let sub_program_with_include = Arc::new(Program {
        includes: vec![trainz_ast::gs::Include {
            path: Some(std::path::PathBuf::from("/super.gs")),
            path_range: None,
            name: "super.gs".to_string(),
            range: Range::default(),
            keyword_include_range: Range::default(),
        }],
        ..(*sub_program).clone()
    });

    let parsed_files = DashMap::new();
    parsed_files.insert(super_uri.to_string(), super_program);
    parsed_files.insert(sub_uri.to_string(), sub_program_with_include.clone());

    let position = Position {
        line: 0,
        character: 54,
    }; // Middle of "inherited"

    let result = gs_goto_definition(
        sub_program_with_include,
        position,
        Uri::from_str(sub_uri).unwrap(),
        &parsed_files,
    );
    assert!(
        result.is_some(),
        "Should find parent MyMethod definition via inherited()"
    );
    if let Some(GotoDefinitionResponse::Link(links)) = result {
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target_uri.to_string(), super_uri);
        assert_eq!(links[0].target_selection_range.start.character, 24); // SuperClass::MyMethod
    } else {
        panic!("Expected Link, got {:?}", result);
    }
}

#[test]
fn test_gs_goto_definition_inherited_deep() {
    let _ = trainz_common::logging::tracing_subscriber::fmt()
        .with_test_writer()
        .try_init();
    let gp_uri = "file:///gp.gs";
    let p_uri = "file:///p.gs";
    let c_uri = "file:///c.gs";

    let gp_source = "class GrandParent { void SharedMethod() {} };";
    let gp_pairs = parse(gp_source).unwrap();
    let gp_program = Arc::new(trainz_ast::gs::process::process_trainz_ast(
        gp_pairs, gp_source,
    ));

    let p_source = "class Parent isclass GrandParent { };";
    let p_pairs = parse(p_source).unwrap();
    let p_program = Arc::new(trainz_ast::gs::process::process_trainz_ast(
        p_pairs, p_source,
    ));

    let c_source = "class Child isclass Parent { void SharedMethod() { inherited(); } };";
    // 012345678901234567890123456789012345678901234567890123
    //                                                   ^ 51
    let c_pairs = parse(c_source).unwrap();
    let c_program = Arc::new(trainz_ast::gs::process::process_trainz_ast(
        c_pairs, c_source,
    ));

    // Setup includes
    let c_program_with_include = Arc::new(Program {
        includes: vec![trainz_ast::gs::Include {
            path: Some(std::path::PathBuf::from("/p.gs")),
            path_range: None,
            name: "p.gs".to_string(),
            range: Range::default(),
            keyword_include_range: Range::default(),
        }],
        ..(*c_program).clone()
    });

    let p_program_with_include = Arc::new(Program {
        includes: vec![trainz_ast::gs::Include {
            path: Some(std::path::PathBuf::from("/gp.gs")),
            path_range: None,
            name: "gp.gs".to_string(),
            range: Range::default(),
            keyword_include_range: Range::default(),
        }],
        ..(*p_program).clone()
    });

    let parsed_files = DashMap::new();
    parsed_files.insert(gp_uri.to_string(), gp_program);
    parsed_files.insert(p_uri.to_string(), p_program_with_include);
    parsed_files.insert(c_uri.to_string(), c_program_with_include.clone());

    let position = Position {
        line: 0,
        character: 51,
    }; // Middle of "inherited"

    let res = gs_goto_definition(
        c_program_with_include,
        position,
        Uri::from_str(c_uri).unwrap(),
        &parsed_files,
    );
    assert!(
        res.is_some(),
        "Should find GrandParent SharedMethod via inherited() from Child"
    );
    if let Some(GotoDefinitionResponse::Link(links)) = res {
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target_uri.to_string(), gp_uri);
    } else {
        panic!("Expected Link, got {:?}", res);
    }
}

#[test]
fn test_gs_goto_definition_inherited_not_current() {
    let _ = trainz_common::logging::tracing_subscriber::fmt()
        .with_test_writer()
        .try_init();
    let uri = "file:///test.gs";
    let source = "class MyClass isclass MyClass { void SharedMethod() { inherited(); } };";
    // 0123456789012345678901234567890123456789012345678901234567890
    //                                                   ^ 54
    let pairs = parse(source).unwrap();
    let program = Arc::new(trainz_ast::gs::process::process_trainz_ast(pairs, source));
    let parsed_files = DashMap::new();
    parsed_files.insert(uri.to_string(), program.clone());

    let position = Position {
        line: 0,
        character: 54,
    };

    let res = gs_goto_definition(
        program,
        position,
        Uri::from_str(uri).unwrap(),
        &parsed_files,
    );

    // It should NOT find MyClass::SharedMethod as inherited, even though MyClass is its own superclass.
    assert!(
        res.is_none(),
        "Should NOT return current method for inherited() call"
    );
}

#[test]
fn test_gs_goto_definition_include() {
    let _ = trainz_common::logging::tracing_subscriber::fmt()
        .with_test_writer()
        .try_init();
    let include_path = "/path/to/Bar.gs";
    let source = format!("include \"{}\"\nclass Foo {{ }};", include_path);
    let pairs = parse(&source).unwrap();
    let mut program = trainz_ast::gs::process::process_trainz_ast(pairs, &source);
    println!("Includes count: {}", program.includes.len());
    if !program.includes.is_empty() {
        println!("Include 0 range: {:?}", program.includes[0].range);
        println!("Include 0 path_range: {:?}", program.includes[0].path_range);
    }
    let path_range = program.includes[0].path_range;
    program.includes[0].path = Some(PathBuf::from(include_path));
    let program = Arc::new(program);
    let uri = Uri::from_str("file:///test.gs").unwrap();
    let position = path_range.unwrap().start; // within the include path string
    println!("Testing position: {:?}", position);
    let parsed_files = DashMap::new();
    parsed_files.insert(uri.to_string(), program.clone());

    let result = gs_goto_definition(program.clone(), position, uri.clone(), &parsed_files);

    assert!(result.is_some());
    if let Some(GotoDefinitionResponse::Link(links)) = result {
        assert_eq!(links.len(), 1);
        assert_eq!(
            links[0].target_uri.to_file_path().unwrap(),
            PathBuf::from(include_path)
        );
    } else {
        panic!("Expected Link response");
    }
}

#[test]
fn test_gs_goto_definition_field_method_call() {
    let _ = trainz_common::logging::tracing_subscriber::fmt()
        .with_test_writer()
        .try_init();
    let source = r#"
            class AcsText {
                void CountTags() { }
            };
            class Test {
                AcsText acs_text;
                void Run() {
                    acs_text.CountTags();
                }
            };
        "#;
    let pairs = parse(source).unwrap();
    let program = Arc::new(trainz_ast::gs::process::process_trainz_ast(pairs, source));
    let uri = Uri::from_str("file:///test.gs").unwrap();
    let position = Position {
        line: 7,
        character: 32,
    };
    let parsed_files = DashMap::new();

    let result = gs_goto_definition(program, position, uri, &parsed_files);

    assert!(result.is_some());
    let locs = match result.unwrap() {
        GotoDefinitionResponse::Array(locs) => locs,
        GotoDefinitionResponse::Link(links) => links
            .into_iter()
            .map(|l| Location {
                uri: l.target_uri,
                range: l.target_selection_range,
            })
            .collect(),
        _ => panic!("Expected array or link"),
    };
    assert_eq!(locs.len(), 1);
    let loc = &locs[0];
    assert_eq!(loc.range.start.line, 2);
    assert_eq!(loc.range.start.character, 21);
    assert_eq!(loc.range.end.line, 2);
    assert_eq!(loc.range.end.character, 30);
}

#[test]
fn test_gs_goto_definition_method_call() {
    let _ = trainz_common::logging::tracing_subscriber::fmt()
        .with_test_writer()
        .try_init();
    let source = r#"
            class Str {
                void Tokens(int pid, string s) { }
            };
            class Test {
                void Run() {
                    Str.Tokens(1, "_");
                }
            };
        "#;
    let pairs = trainz_ast::gs::process::process_trainz_ast(
        trainz_parser::gs::parse(source).unwrap(),
        source,
    );
    let program = Arc::new(pairs);
    let uri = Uri::from_str("file:///test.gs").unwrap();
    let parsed_files = DashMap::new();

    // Position on "Tokens" in `Str.Tokens`
    let position = Position {
        line: 6,
        character: 25,
    };

    let result = gs_goto_definition(program.clone(), position, uri.clone(), &parsed_files);
    assert!(result.is_some(), "Expected a definition to be found");
    let locs = match result.unwrap() {
        GotoDefinitionResponse::Array(locs) => locs,
        GotoDefinitionResponse::Link(links) => links
            .into_iter()
            .map(|l| Location {
                uri: l.target_uri,
                range: l.target_selection_range,
            })
            .collect(),
        _ => panic!("Expected array or link"),
    };
    assert_eq!(locs.len(), 1);
    assert_eq!(locs[0].range.start.line, 2);
}

#[test]
fn test_gs_goto_definition_chained_method_call() {
    let _ = trainz_common::logging::tracing_subscriber::fmt()
        .with_test_writer()
        .try_init();
    let source = r#"
            class Asset {
                void FindAsset(string name) { }
            };
            class System {
                Asset GetAsset() { return new Asset(); }
            };
            class Test {
                void Run() {
                    System sys;
                    sys.GetAsset().FindAsset("lamp-lib");
                }
            };
        "#;
    let pairs = trainz_ast::gs::process::process_trainz_ast(
        trainz_parser::gs::parse(source).unwrap(),
        source,
    );
    let program = Arc::new(pairs);
    let uri = Uri::from_str("file:///test.gs").unwrap();
    let parsed_files = DashMap::new();

    // Position on "FindAsset" in `sys.GetAsset().FindAsset("lamp-lib");`
    let position = Position {
        line: 10,
        character: 35,
    };

    let result = gs_goto_definition(program.clone(), position, uri.clone(), &parsed_files);
    assert!(
        result.is_some(),
        "Expected a definition to be found for chained method call"
    );
    let locs = match result.unwrap() {
        GotoDefinitionResponse::Array(locs) => locs,
        GotoDefinitionResponse::Link(links) => links
            .into_iter()
            .map(|l| Location {
                uri: l.target_uri,
                range: l.target_selection_range,
            })
            .collect(),
        _ => panic!("Expected array or link"),
    };
    assert_eq!(locs.len(), 1);
    assert_eq!(locs[0].range.start.line, 2);
}

#[test]
fn test_gs_goto_definition_chained_method_inheritance() {
    let _ = trainz_common::logging::tracing_subscriber::fmt()
        .with_test_writer()
        .try_init();
    let source = r#"
            class Base {
                AcsText GetConfigAcsText() { return null; }
            };
            class Asset isclass Base {
            };
            class AcsText {
                AcsText GetNamedAcsText(string name) { return null; }
            };
            class Test {
                Asset GetAsset() { return null; }
                void Run() {
                    GetAsset().GetConfigAcsText().GetNamedAcsText("mesh-table");
                }
            };
        "#;
    let pairs = parse(source).unwrap();
    let program = Arc::new(trainz_ast::gs::process::process_trainz_ast(pairs, source));
    let uri = Uri::from_str("file:///test.gs").unwrap();
    // "GetAsset().GetConfigAcsText().GetNamedAcsText("mesh-table");"
    let position = Position {
        line: 12,
        character: 53,
    }; // middle of GetNamedAcsText
    let parsed_files = DashMap::new();
    parsed_files.insert(uri.to_string(), program.clone());

    let result = gs_goto_definition(program, position, uri.clone(), &parsed_files);
    assert!(result.is_some(), "Expected to find definition");
    let locs = match result.unwrap() {
        GotoDefinitionResponse::Array(locs) => locs,
        GotoDefinitionResponse::Link(links) => links
            .into_iter()
            .map(|l| Location {
                uri: l.target_uri,
                range: l.target_selection_range,
            })
            .collect(),
        _ => panic!("Expected array or link"),
    };
    assert_eq!(locs.len(), 1);
    assert_eq!(locs[0].range.start.line, 7);
}

#[test]
fn test_gs_goto_definition_get_named_acs_text_inheritance() {
    let _ = trainz_common::logging::tracing_subscriber::fmt()
        .with_test_writer()
        .try_init();
    let source = r#"
            class BaseAcsText {
                AcsText GetNamedAcsText(string name) { return null; }
            };
            class AcsText isclass BaseAcsText {
            };
            class Base {
                AcsText GetConfigAcsText() { return null; }
            };
            class Asset isclass Base {
            };
            class Test {
                Asset GetAsset() { return null; }
                void Run() {
                    GetAsset().GetConfigAcsText().GetNamedAcsText("mesh-table");
                }
            };
        "#;
    let pairs = parse(source).unwrap();
    let program = Arc::new(trainz_ast::gs::process::process_trainz_ast(pairs, source));
    let uri = Uri::from_str("file:///test.gs").unwrap();
    // "GetAsset().GetConfigAcsText().GetNamedAcsText("mesh-table");"
    let position = Position {
        line: 14,
        character: 53,
    }; // middle of GetNamedAcsText
    let parsed_files = DashMap::new();
    parsed_files.insert(uri.to_string(), program.clone());

    let result = gs_goto_definition(program, position, uri.clone(), &parsed_files);
    assert!(result.is_some(), "Expected to find definition");
    let locs = match result.unwrap() {
        GotoDefinitionResponse::Array(locs) => locs,
        GotoDefinitionResponse::Link(links) => links
            .into_iter()
            .map(|l| Location {
                uri: l.target_uri,
                range: l.target_selection_range,
            })
            .collect(),
        _ => panic!("Expected array or link"),
    };
    assert_eq!(locs.len(), 1);
    assert_eq!(locs[0].range.start.line, 2);
}

#[test]
fn test_gs_goto_definition_local_var_method_call() {
    let _ = trainz_common::logging::tracing_subscriber::fmt()
        .with_test_writer()
        .try_init();
    let source = r#"
class AcsText {
    public void GetIndexedTagName(int i) {}
};

class SignalNSW {
    public void UnSetLamps() {
        AcsText meshtable = new AcsText();
        meshtable.GetIndexedTagName(0);
    }
};
        "#;
    let pairs = trainz_ast::gs::process::process_trainz_ast(
        trainz_parser::gs::parse(source).unwrap(),
        source,
    );
    let program = Arc::new(pairs);
    let parsed_files = DashMap::new();
    parsed_files.insert("test://file".to_string(), program.clone());
    let uri = Uri::from_str("test://file").unwrap();

    let position = Position {
        line: 8,
        character: 25,
    }; // inside GetIndexedTagName
    let result = gs_goto_definition(program, position, uri.clone(), &parsed_files);
    assert!(result.is_some(), "Definition not found");

    let locs = match result.unwrap() {
        GotoDefinitionResponse::Array(locs) => locs,
        GotoDefinitionResponse::Link(links) => links
            .into_iter()
            .map(|l| Location {
                uri: l.target_uri,
                range: l.target_selection_range,
            })
            .collect(),
        _ => panic!("Expected array or link"),
    };
    assert_eq!(locs.len(), 1);
    assert_eq!(locs[0].range.start.line, 2);
}

#[test]
fn test_gs_goto_definition_meshtable_resolution() {
    let _ = trainz_common::logging::tracing_subscriber::fmt()
        .with_test_writer()
        .try_init();
    let source = r#"
class AcsText {
    public void GetIndexedTagName(int i) {}
};

class SignalNSW {
    AcsText meshtable;
    public void UnSetLamps() {
        int i = 0;
        meshtable.GetIndexedTagName(i);
    }
};
        "#;
    let pairs = trainz_ast::gs::process::process_trainz_ast(
        trainz_parser::gs::parse(source).unwrap(),
        source,
    );
    let program = Arc::new(pairs);
    let parsed_files = DashMap::new();
    let uri = Uri::from_str("test://file").unwrap();
    parsed_files.insert(uri.to_string(), program.clone());

    let position = Position {
        line: 9,
        character: 25,
    }; // inside GetIndexedTagName
    let result = gs_goto_definition(program, position, uri.clone(), &parsed_files);
    assert!(result.is_some(), "Definition not found");

    let locs = match result.unwrap() {
        GotoDefinitionResponse::Array(locs) => locs,
        GotoDefinitionResponse::Link(links) => links
            .into_iter()
            .map(|l| Location {
                uri: l.target_uri,
                range: l.target_selection_range,
            })
            .collect(),
        _ => panic!("Expected array or link"),
    };
    assert_eq!(locs.len(), 1);
    assert_eq!(locs[0].range.start.line, 2);
}

#[test]
fn test_gs_goto_definition_multilevel_inheritance() {
    let _ = trainz_common::logging::tracing_subscriber::fmt()
        .with_test_writer()
        .try_init();
    let source = r#"
class GrandParent {
    public void GrandMethod() {}
};
class Parent isclass GrandParent {
    public void ParentMethod() {}
};
class Child isclass Parent {
    public void ChildMethod() {}
};

class Test {
    public void Run() {
        Child c = new Child();
        c.GrandMethod();
    }
};
        "#;
    let pairs = trainz_ast::gs::process::process_trainz_ast(
        trainz_parser::gs::parse(source).unwrap(),
        source,
    );
    let program = Arc::new(pairs);
    let parsed_files = DashMap::new();
    let uri = Uri::from_str("test://file").unwrap();
    parsed_files.insert(uri.to_string(), program.clone());

    let position = Position {
        line: 14,
        character: 15,
    }; // inside GrandMethod
    let result = gs_goto_definition(program, position, uri.clone(), &parsed_files);
    assert!(result.is_some(), "Definition not found");

    let locs = match result.unwrap() {
        GotoDefinitionResponse::Array(locs) => locs,
        GotoDefinitionResponse::Link(links) => links
            .into_iter()
            .map(|l| Location {
                uri: l.target_uri,
                range: l.target_selection_range,
            })
            .collect(),
        _ => panic!("Expected array or link"),
    };
    assert_eq!(locs.len(), 1);
    assert_eq!(locs[0].range.start.line, 2);
}

#[test]
fn test_gs_goto_definition_inherited_field_method_call() {
    let _ = trainz_common::logging::tracing_subscriber::fmt()
        .with_test_writer()
        .try_init();
    let source = r#"
class AcsText {
    public void GetIndexedTagName(int i) {}
};

class BaseClass {
    AcsText meshtable;
};

class SignalNSW isclass BaseClass {
    public void UnSetLamps() {
        int i = 0;
        meshtable.GetIndexedTagName(i);
    }
};
        "#;
    let pairs = trainz_ast::gs::process::process_trainz_ast(
        trainz_parser::gs::parse(source).unwrap(),
        source,
    );
    let program = Arc::new(pairs);
    let parsed_files = DashMap::new();
    let uri = Uri::from_str("test://file").unwrap();
    parsed_files.insert(uri.to_string(), program.clone());

    let position = Position {
        line: 12,
        character: 25,
    }; // inside GetIndexedTagName
    let result = gs_goto_definition(program, position, uri.clone(), &parsed_files);
    assert!(result.is_some(), "Definition not found");

    let locs = match result.unwrap() {
        GotoDefinitionResponse::Array(locs) => locs,
        GotoDefinitionResponse::Link(links) => links
            .into_iter()
            .map(|l| Location {
                uri: l.target_uri,
                range: l.target_selection_range,
            })
            .collect(),
        _ => panic!("Expected array or link"),
    };
    assert_eq!(locs.len(), 1);
    assert_eq!(locs[0].range.start.line, 2);
}

#[test]
fn test_gs_goto_definition_static_method_call() {
    let _ = trainz_common::logging::tracing_subscriber::fmt()
        .with_test_writer()
        .try_init();
    let source = "class Router {\n    static GameObject GetCurrentThreadGameObject() { return null; }\n};\nclass Test {\n    void Run() {\n        GameObject g = Router.GetCurrentThreadGameObject();\n    }\n};";
    let pairs = trainz_ast::gs::process::process_trainz_ast(
        trainz_parser::gs::parse(source).unwrap(),
        source,
    );
    let program = Arc::new(pairs);
    let uri = Uri::from_str("file:///test.gs").unwrap();
    let parsed_files = DashMap::new();
    parsed_files.insert(uri.to_string(), program.clone());

    // Position on "GetCurrentThreadGameObject" in `Router.GetCurrentThreadGameObject();`
    // 0: class Router {
    // 1:     static GameObject GetCurrentThreadGameObject() { return null; }
    // 2: };
    // 3: class Test {
    // 4:     void Run() {
    // 5:         GameObject g = Router.GetCurrentThreadGameObject();
    let position = Position {
        line: 5,
        character: 35,
    };

    let result = gs_goto_definition(program.clone(), position, uri.clone(), &parsed_files);
    assert!(
        result.is_some(),
        "Expected a definition to be found for static method call"
    );
    let locs = match result.unwrap() {
        GotoDefinitionResponse::Array(locs) => locs,
        GotoDefinitionResponse::Link(links) => links
            .into_iter()
            .map(|l| Location {
                uri: l.target_uri,
                range: l.target_selection_range,
            })
            .collect(),
        _ => panic!("Expected array or link"),
    };
    assert_eq!(locs.len(), 1);
    assert_eq!(locs[0].range.start.line, 1);
}

#[test]
fn test_gs_goto_definition_overload_inheritance() {
    let _ = trainz_common::logging::tracing_subscriber::fmt()
        .with_test_writer()
        .try_init();
    let source = r#"
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
    let pairs = trainz_ast::gs::process::process_trainz_ast(
        trainz_parser::gs::parse(source).unwrap(),
        source,
    );
    let program = Arc::new(pairs);
    let parsed_files = DashMap::new();
    let uri = Uri::from_str("test://file").unwrap();
    parsed_files.insert(uri.to_string(), program.clone());

    let position = Position {
        line: 9,
        character: 11,
    }; // on "foo" in me.foo();
    let result = gs_goto_definition(program, position, uri.clone(), &parsed_files);
    assert!(result.is_some(), "Definition not found");

    let locs = match result.unwrap() {
        GotoDefinitionResponse::Array(locs) => locs,
        GotoDefinitionResponse::Link(links) => links
            .into_iter()
            .map(|l| Location {
                uri: l.target_uri,
                range: l.target_selection_range,
            })
            .collect(),
        _ => panic!("Expected array or link"),
    };
    // It should find both foo() in A and foo(int) in B
    assert_eq!(
        locs.len(),
        2,
        "Should find both overloads, but found: {:?}",
        locs
    );
}

#[test]
fn test_gs_goto_definition_scoped_search() {
    let _ = trainz_common::logging::tracing_subscriber::fmt()
        .with_test_writer()
        .try_init();
    let source_a = "class A {};";
    let source_b = "class B {};";

    let program_a = Arc::new(trainz_ast::gs::process::process_trainz_ast(
        trainz_parser::gs::parse(source_a).unwrap(),
        source_a,
    ));
    let program_b = Arc::new(trainz_ast::gs::process::process_trainz_ast(
        trainz_parser::gs::parse(source_b).unwrap(),
        source_b,
    ));

    let parsed_files = DashMap::new();
    let uri_a = Uri::from_str("file:///a.gs").unwrap();
    let uri_b = Uri::from_str("file:///b.gs").unwrap();
    parsed_files.insert(uri_a.to_string(), program_a.clone());
    parsed_files.insert(uri_b.to_string(), program_b.clone());

    // Searching for "B" in a.gs, but a.gs does NOT include b.gs
    // We simulate this by passing "B" as the target manually in our head,
    // but gs_goto_definition finds the ID at position.

    // Let's make a.gs use B
    let source_a_with_b = "class A { B b; };";
    let program_a_with_b = Arc::new(trainz_ast::gs::process::process_trainz_ast(
        trainz_parser::gs::parse(source_a_with_b).unwrap(),
        source_a_with_b,
    ));
    parsed_files.insert(uri_a.to_string(), program_a_with_b.clone());

    let position = Position {
        line: 0,
        character: 10,
    }; // on "B" in "B b;"

    let result = gs_goto_definition(program_a_with_b, position, uri_a.clone(), &parsed_files);

    // Should NOT find B because it's not included
    assert!(
        result.is_none(),
        "Should not find B because it is not included"
    );

    // Now include b.gs in a.gs
    let source_a_with_include = "include \"b.gs\"\nclass A { B b; };";
    let mut program_a_inc = trainz_ast::gs::process::process_trainz_ast(
        trainz_parser::gs::parse(source_a_with_include).unwrap(),
        source_a_with_include,
    );
    program_a_inc.includes[0].path = Some("file:///b.gs".into());
    let program_a_inc = Arc::new(program_a_inc);
    parsed_files.insert(uri_a.to_string(), program_a_inc.clone());

    let position = Position {
        line: 1,
        character: 10,
    }; // on "B" in "B b;"

    let result = gs_goto_definition(program_a_inc, position, uri_a, &parsed_files);
    assert!(result.is_some(), "Should find B because it is now included");
}
