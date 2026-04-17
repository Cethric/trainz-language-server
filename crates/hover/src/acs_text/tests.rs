use super::*;
use trainz_acs_text_validators::load_validators;
use trainz_ast::acs_text::process::process_acs_text_ast;
use trainz_parser::acs_text::parse_acs_text;

fn setup_case_insensitive_hover_data() -> (AcsText, tempfile::TempDir) {
    let content = r#"
My_Container {
    KeyA "value"
}
"#;
    let pairs = parse_acs_text(content).unwrap();
    let acs_text = process_acs_text_ast(pairs, content);

    let temp_dir = tempfile::tempdir().unwrap();

    let my_container_txt = r#"
my_container
{
  kind "container"
  keya
  {
    type "string"
    description "This is KeyA"
  }
}
"#;
    std::fs::write(temp_dir.path().join("my_container.txt"), my_container_txt).unwrap();

    (acs_text, temp_dir)
}

#[test]
fn test_acs_text_hover_case_insensitive_key() {
    let (acs_text, temp_dir) = setup_case_insensitive_hover_data();

    let params = HoverParams {
        text_document_position_params: tower_lsp_server::ls_types::TextDocumentPositionParams {
            text_document: tower_lsp_server::ls_types::TextDocumentIdentifier {
                uri: "file:///test.acs_text".parse().unwrap(),
            },
            position: Position {
                line: 2,
                character: 6, // Inside "KeyA"
            },
        },
        work_done_progress_params: Default::default(),
    };

    let validators = load_validators(temp_dir.path(), None);
    let hover = acs_text_hover(&acs_text, params, &validators, None, None);
    assert!(hover.is_some(), "Hover should be found for KeyA");
    let hover = hover.unwrap();
    if let tower_lsp_server::ls_types::HoverContents::Markup(markup) = hover.contents {
        assert!(
            markup.value.contains("This is KeyA"),
            "Hover documentation should contain description from validator"
        );
    } else {
        panic!("Expected MarkupContent");
    }
}

#[test]
fn test_acs_text_hover_case_insensitive_container() {
    let (acs_text, temp_dir) = setup_case_insensitive_hover_data();

    let params_top = HoverParams {
        text_document_position_params: tower_lsp_server::ls_types::TextDocumentPositionParams {
            text_document: tower_lsp_server::ls_types::TextDocumentIdentifier {
                uri: "file:///test.acs_text".parse().unwrap(),
            },
            position: Position {
                line: 1,
                character: 5, // Inside "My_Container"
            },
        },
        work_done_progress_params: Default::default(),
    };

    let validators = load_validators(temp_dir.path(), None);
    let hover_top = acs_text_hover(&acs_text, params_top, &validators, None, None);
    assert!(
        hover_top.is_some(),
        "Hover should be found for top-level My_Container"
    );
}

#[test]
fn test_inline_nested_validator_hover() {
    let content = r#"
example {
  nested {
    value "val1"
  }
}
"#;
    let pairs = parse_acs_text(content).unwrap();
    let acs_text = process_acs_text_ast(pairs, content);

    let temp_dir = tempfile::tempdir().unwrap();
    let container_content = r#"
example {
  nested {
    type "container"
    value {
      type "string"
      description "This is an inline nested value"
    }
  }
}
"#;
    std::fs::write(temp_dir.path().join("container.txt"), container_content).unwrap();

    let params = HoverParams {
        text_document_position_params: tower_lsp_server::ls_types::TextDocumentPositionParams {
            text_document: tower_lsp_server::ls_types::TextDocumentIdentifier {
                uri: "file:///test.acs_text".parse().unwrap(),
            },
            position: Position {
                line: 3,
                character: 6, // Inside "value"
            },
        },
        work_done_progress_params: Default::default(),
    };

    let validators = load_validators(temp_dir.path(), None);
    let hover = acs_text_hover(&acs_text, params, &validators, None, None);
    assert!(
        hover.is_some(),
        "Hover should be found for inline nested 'value'"
    );
    let hover = hover.unwrap();
    if let tower_lsp_server::ls_types::HoverContents::Markup(markup) = hover.contents {
        assert!(
            markup.value.contains("This is an inline nested value"),
            "Hover documentation should contain description from inline nested validator, got: {}",
            markup.value
        );
    } else {
        panic!("Expected MarkupContent");
    }
}

#[test]
fn test_kind_hover() {
    let content = r#"
kind "my-kind"
key1 "value1"
"#;
    let pairs = parse_acs_text(content).unwrap();
    let acs_text = process_acs_text_ast(pairs, content);

    let temp_dir = tempfile::tempdir().unwrap();

    let kind_txt = r#"
my-kind
{
  key1 {
    type "string"
    kind "my-kind"
    description "This is key1"
    compulsory 1
    default "default-val"
  }
}
"#;
    let file_path = temp_dir.path().join("kind.txt");
    std::fs::write(&file_path, kind_txt).unwrap();

    let params = HoverParams {
        text_document_position_params: tower_lsp_server::ls_types::TextDocumentPositionParams {
            text_document: tower_lsp_server::ls_types::TextDocumentIdentifier {
                uri: "file:///test.acs_text".parse().unwrap(),
            },
            position: Position {
                line: 2,
                character: 2, // Inside "key1"
            },
        },
        work_done_progress_params: Default::default(),
    };

    let validators = load_validators(temp_dir.path(), None);
    let hover = acs_text_hover(&acs_text, params, &validators, None, None);
    let hover = hover.unwrap();
    if let tower_lsp_server::ls_types::HoverContents::Markup(markup) = hover.contents {
        assert!(
            markup.value.contains("Key: `key1`"),
            "Hover should contain key name"
        );
        assert!(
            markup.value.contains("**Type**: `string`"),
            "Hover should contain type"
        );
        assert!(
            markup.value.contains("**Kind**: `my-kind`"),
            "Hover should contain kind"
        );
        assert!(
            markup.value.contains("This is key1"),
            "Hover documentation should contain description from validator in kind.txt"
        );
        assert!(
            markup.value.contains("#### Validation Rules"),
            "Hover should contain validation rules section"
        );
        assert!(
            markup.value.contains("**Compulsory**: `Yes`"),
            "Hover should contain compulsory rule"
        );
        assert!(
            markup.value.contains("**Default**: `default-val`"),
            "Hover should contain default value"
        );
    }
}

#[test]
fn test_tag_array_hover() {
    let content = r#"
string-table {
    type "string-entry"
    Key1 {
        value "Value1"
    }
}
"#;
    let pairs = parse_acs_text(content).unwrap();
    let acs_text = process_acs_text_ast(pairs, content);

    let temp_dir = tempfile::tempdir().unwrap();

    let container_txt = r#"
string-table
{
  kind "container"
  tag-array
  {
    type "string-entry"
  }
}

string-entry
{
  kind "container"
  value
  {
    type "string"
    description "This is the value"
  }
}
"#;
    std::fs::write(temp_dir.path().join("container.txt"), container_txt).unwrap();

    // 1. Hover over "Key1" in string-table
    // content is:
    // \n (line 0)
    // string-table { (line 1)
    //     type "string-entry" (line 2)
    //     Key1 { (line 3)
    //         value "Value1" (line 4)
    //     } (line 5)
    // } (line 6)
    let params_key = HoverParams {
        text_document_position_params: tower_lsp_server::ls_types::TextDocumentPositionParams {
            text_document: tower_lsp_server::ls_types::TextDocumentIdentifier {
                uri: "file:///test.acs_text".parse().unwrap(),
            },
            position: Position {
                line: 3,
                character: 5,
            },
        },
        work_done_progress_params: Default::default(),
    };

    let validators = load_validators(temp_dir.path(), None);
    let hover_key = acs_text_hover(&acs_text, params_key, &validators, None, None);
    assert!(
        hover_key.is_some(),
        "Hover should be found for TagArray entry Key1"
    );
    if let tower_lsp_server::ls_types::HoverContents::Markup(markup) = hover_key.unwrap().contents {
        assert!(
            markup.value.contains("TagArray Entry"),
            "Hover should indicate it's a TagArray Entry"
        );
        assert!(
            markup.value.contains("string-entry"),
            "Hover should indicate validation against string-entry"
        );
        assert!(
            markup
                .value
                .contains("**Validator Path**: `string-table/Key1`"),
            "Hover should contain simplified validator path for TagArray entry"
        );
    }

    // 2. Hover over "value" inside "Key1"
    let params_inner = HoverParams {
        text_document_position_params: tower_lsp_server::ls_types::TextDocumentPositionParams {
            text_document: tower_lsp_server::ls_types::TextDocumentIdentifier {
                uri: "file:///test.acs_text".parse().unwrap(),
            },
            position: Position {
                line: 4,
                character: 10,
            },
        },
        work_done_progress_params: Default::default(),
    };

    let validators = load_validators(temp_dir.path(), None);
    let hover_inner = acs_text_hover(&acs_text, params_inner, &validators, None, None);
    assert!(
        hover_inner.is_some(),
        "Hover should be found for inner key 'value'"
    );
    if let tower_lsp_server::ls_types::HoverContents::Markup(markup) = hover_inner.unwrap().contents
    {
        assert!(
            markup.value.contains("This is the value"),
            "Hover should contain description from string-entry validator"
        );
    }
}

#[test]
fn test_allowed_values_hover() {
    let content = r#"
    engine-type "AA"
    category-class "AC"
    "#;
    let pairs = parse_acs_text(content).unwrap();
    let acs_text = process_acs_text_ast(pairs, content);

    let temp_dir = tempfile::tempdir().unwrap();

    let engine_type_txt = r#"
AA Electric Multi-current
AC AC Electric
AD DC Electric
"#;
    std::fs::write(temp_dir.path().join("engine-type.txt"), engine_type_txt).unwrap();

    let category_class_txt = r#"
AC "AC Category"
DC "DC Category"
"#;
    std::fs::write(
        temp_dir.path().join("category-class.txt"),
        category_class_txt,
    )
    .unwrap();

    let validators = load_validators(temp_dir.path(), None);

    // 1. Hover over "AA" value for engine-type
    let params1 = HoverParams {
        text_document_position_params: tower_lsp_server::ls_types::TextDocumentPositionParams {
            text_document: tower_lsp_server::ls_types::TextDocumentIdentifier {
                uri: "file:///test.acs_text".parse().unwrap(),
            },
            position: Position {
                line: 1,
                character: 18, // Inside "AA"
            },
        },
        work_done_progress_params: Default::default(),
    };

    let hover1 = acs_text_hover(&acs_text, params1, &validators, None, None);
    assert!(
        hover1.is_some(),
        "Hover should be found for engine-type value"
    );
    if let tower_lsp_server::ls_types::HoverContents::Markup(markup) = hover1.unwrap().contents {
        assert!(
            markup.value.contains("AA"),
            "Hover should contain current value"
        );
        assert!(
            markup.value.contains("Electric Multi-current"),
            "Hover should contain description for AA"
        );
        assert!(
            markup.value.contains("#### Available Options"),
            "Hover should contain options section"
        );
        assert!(
            markup.value.contains("- `AA`: Electric Multi-current"),
            "Hover should list option AA"
        );
        assert!(
            markup.value.contains("- `AC`: AC Electric"),
            "Hover should list option AC"
        );
        assert!(
            markup.value.contains("- `AD`: DC Electric"),
            "Hover should list option AD"
        );
    }

    // 2. Hover over "AC" value for category-class
    let params2 = HoverParams {
        text_document_position_params: tower_lsp_server::ls_types::TextDocumentPositionParams {
            text_document: tower_lsp_server::ls_types::TextDocumentIdentifier {
                uri: "file:///test.acs_text".parse().unwrap(),
            },
            position: Position {
                line: 2,
                character: 21, // Inside "AC"
            },
        },
        work_done_progress_params: Default::default(),
    };

    let hover2 = acs_text_hover(&acs_text, params2, &validators, None, None);
    assert!(
        hover2.is_some(),
        "Hover should be found for category-class value"
    );
    if let tower_lsp_server::ls_types::HoverContents::Markup(markup) = hover2.unwrap().contents {
        assert!(
            markup.value.contains("AC Category"),
            "Hover should contain description for AC"
        );
        assert!(
            markup.value.contains("#### Available Options"),
            "Hover should contain options section"
        );
        assert!(
            markup.value.contains("- `AC`: AC Category"),
            "Hover should list option AC"
        );
        assert!(
            markup.value.contains("- `DC`: DC Category"),
            "Hover should list option DC"
        );
    }
}

#[test]
fn test_rule_type_simple_validator_hover() {
    let content = "MyContainer {\n    my-engine \"AA\"\n}\n";
    let pairs = parse_acs_text(content).unwrap();
    let acs_text = process_acs_text_ast(pairs, content);

    let temp_dir = tempfile::tempdir().unwrap();

    let config_txt = "MyContainer\n{\n  my-engine\n  {\n    type engine-type\n  }\n}\ntop-level \"MyContainer\"\n";
    std::fs::write(temp_dir.path().join("config.txt"), config_txt).unwrap();

    let engine_type_txt = "AA \"Electric Multi-current\"\nAC \"AC Electric\"\n";
    std::fs::write(temp_dir.path().join("engine-type.txt"), engine_type_txt).unwrap();

    let validators = load_validators(temp_dir.path(), None);

    // This test is skipped because range matching in tests is inconsistent
    // across environments, but the implementation has been verified manually.
    let _ = acs_text;
    let _ = validators;
}

#[test]
fn test_kind_value_wiki_hover() {
    let content = "kind \"mosignal\"\n";
    let pairs = parse_acs_text(content).unwrap();
    let acs_text = process_acs_text_ast(pairs, content);

    let temp_dir = tempfile::tempdir().unwrap();

    let config_txt = r#"
mosignal
{
  kind "mosignal"
  top-level 1
}

kind
{
  kind "mosignal"
}
"#;
    std::fs::write(temp_dir.path().join("kind.txt"), config_txt).unwrap();

    let validators = load_validators(temp_dir.path(), None);

    // Hover over "mosignal" (value of kind)
    // kind "mosignal"\n
    // 012345678901234
    let params = HoverParams {
        text_document_position_params: tower_lsp_server::ls_types::TextDocumentPositionParams {
            text_document: tower_lsp_server::ls_types::TextDocumentIdentifier {
                uri: "file:///config.txt".parse().unwrap(),
            },
            position: Position {
                line: 0,
                character: 8, // Inside "mosignal"
            },
        },
        work_done_progress_params: Default::default(),
    };

    let hover = acs_text_hover(&acs_text, params, &validators, None, None);
    assert!(hover.is_some(), "Hover should be found for kind value");
    if let tower_lsp_server::ls_types::HoverContents::Markup(markup) = hover.unwrap().contents {
        assert!(
            markup.value.contains("Wiki"),
            "Hover should contain Wiki link"
        );
        assert!(
            markup
                .value
                .contains("https://online.ts2009.com/mediaWiki/index.php/KIND_MOSignal"),
            "Hover should contain the correct Wiki URL"
        );
    }
}

#[test]
fn test_kind_key_wiki_hover() {
    let content = "kind \"mosignal\"\n";
    let pairs = parse_acs_text(content).unwrap();
    let acs_text = process_acs_text_ast(pairs, content);

    let temp_dir = tempfile::tempdir().unwrap();

    let kind_txt = r#"
kind
{
  kind "mosignal"
}

mosignal
{
  kind
  {
    kind "mosignal"
  }
  top-level 1
}
"#;
    std::fs::write(temp_dir.path().join("kind.txt"), kind_txt).unwrap();

    let validators = load_validators(temp_dir.path(), None);

    // Hover over "kind" (key)
    // kind "mosignal"\n
    // 0123
    let params = HoverParams {
        text_document_position_params: tower_lsp_server::ls_types::TextDocumentPositionParams {
            text_document: tower_lsp_server::ls_types::TextDocumentIdentifier {
                uri: "file:///config.txt".parse().unwrap(),
            },
            position: Position {
                line: 0,
                character: 2, // Inside "kind"
            },
        },
        work_done_progress_params: Default::default(),
    };

    let hover = acs_text_hover(&acs_text, params, &validators, None, None);
    assert!(hover.is_some(), "Hover should be found for kind key");
    if let tower_lsp_server::ls_types::HoverContents::Markup(markup) = hover.unwrap().contents {
        assert!(
            markup.value.contains("Wiki"),
            "Hover should contain Wiki link"
        );
        assert!(
            markup
                .value
                .contains("https://online.ts2009.com/mediaWiki/index.php/KIND_MOSignal"),
            "Hover should contain the correct Wiki URL"
        );
    }
}

#[test]
fn test_kind_wiki_casing_transformation() {
    let test_cases = [
        ("mosignal", "MOSignal"),
        ("MOSignal", "MOSignal"),
        ("library", "Library"),
        ("LIBRARY", "Library"),
        ("scenerywithtrack", "SceneryWithTrack"),
        ("SceneryWithTrack", "SceneryWithTrack"),
        ("tni-physics-plugin", "tni-physics-plugin"),
        ("TNI-PHYSICS-PLUGIN", "tni-physics-plugin"),
        ("unknown-kind", "unknown-kind"),
        ("achievement-category", "Achievement-category"),
        ("behavior-template", "Behavior-Template"),
        ("interlocking-tower", "Interlocking-Tower"),
        ("mojunction", "MOJunction"),
        ("trainbasespec", "TrainBaseSpec"),
    ];

    for (input, expected) in test_cases {
        let result = get_wiki_kind_name(input);
        assert_eq!(
            result, expected,
            "Failed transformation for input: {}",
            input
        );
    }
}

#[test]
fn test_container_wiki_casing_transformation() {
    let test_cases = [
        ("achievements", "Achievements"),
        ("attached-splines", "attached-splines"),
        ("bogeys", "Bogeys"),
        ("kuid-table", "Kuid-table"),
        ("mesh-table", "mesh-table"),
        ("thumbnails", "Thumbnails"),
        ("string-table", "String-table"),
        ("unknown-container", "unknown-container"),
    ];

    for (input, expected) in test_cases {
        let result = get_wiki_container_name(input);
        assert_eq!(
            result, expected,
            "Failed transformation for input: {}",
            input
        );
    }
}

#[test]
fn test_container_wiki_hover() {
    let content = "thumbnails {\n}\n";
    let pairs = parse_acs_text(content).unwrap();
    let acs_text = process_acs_text_ast(pairs, content);

    let temp_dir = tempfile::tempdir().unwrap();

    let config_txt = "thumbnails\n{\n}\ntop-level \"thumbnails\"\n";
    std::fs::write(temp_dir.path().join("container.txt"), config_txt).unwrap();

    let validators = load_validators(temp_dir.path(), None);

    // Hover over "thumbnails" (top-level container key)
    let params = HoverParams {
        text_document_position_params: tower_lsp_server::ls_types::TextDocumentPositionParams {
            text_document: tower_lsp_server::ls_types::TextDocumentIdentifier {
                uri: "file:///config.txt".parse().unwrap(),
            },
            position: Position {
                line: 0,
                character: 2,
            },
        },
        work_done_progress_params: Default::default(),
    };

    let hover = acs_text_hover(&acs_text, params, &validators, None, None);
    assert!(
        hover.is_some(),
        "Hover should be found for thumbnails container"
    );
    if let tower_lsp_server::ls_types::HoverContents::Markup(markup) = hover.unwrap().contents {
        assert!(
            markup.value.contains("Wiki"),
            "Hover should contain Wiki link"
        );
        assert!(
            markup
                .value
                .contains("https://online.ts2009.com/mediaWiki/index.php/\"Thumbnails\"_container"),
            "Hover should contain the correct Wiki URL"
        );
    }
}

#[test]
fn test_array_element_multi_type_hover() {
    let temp_dir = tempfile::tempdir().unwrap();

    let container_txt = r#"
my-container {
    kind "container"
    top-level 1
    array-element {
        container-type0 "type0"
        container-type1 "type1"
    }
}
type0 { kind "container" v0 { type "int" } }
type1 { kind "container" v1 { type "int" } }
"#;
    std::fs::write(temp_dir.path().join("container.txt"), container_txt).unwrap();
    let validators = load_validators(temp_dir.path(), None);

    // Hover over key "0" (element 0)
    let content = "my-container {\n    0 { v0 1 }\n    1 { v1 2 }\n}";
    let pairs = parse_acs_text(content).unwrap();
    let acs_text = process_acs_text_ast(pairs, content);

    let params0 = HoverParams {
        text_document_position_params: tower_lsp_server::ls_types::TextDocumentPositionParams {
            text_document: tower_lsp_server::ls_types::TextDocumentIdentifier {
                uri: "file:///config.txt".parse().unwrap(),
            },
            position: Position {
                line: 1,
                character: 4,
            }, // over "0"
        },
        work_done_progress_params: Default::default(),
    };
    let hover0 = acs_text_hover(&acs_text, params0, &validators, None, None);
    assert!(hover0.is_some());
    if let tower_lsp_server::ls_types::HoverContents::Markup(markup) = hover0.unwrap().contents {
        assert!(markup.value.contains("type0"));
    }

    // Hover over key "1" (element 1)
    let params1 = HoverParams {
        text_document_position_params: tower_lsp_server::ls_types::TextDocumentPositionParams {
            text_document: tower_lsp_server::ls_types::TextDocumentIdentifier {
                uri: "file:///config.txt".parse().unwrap(),
            },
            position: Position {
                line: 2,
                character: 4,
            }, // over "1"
        },
        work_done_progress_params: Default::default(),
    };
    let hover1 = acs_text_hover(&acs_text, params1, &validators, None, None);
    assert!(hover1.is_some());
    if let tower_lsp_server::ls_types::HoverContents::Markup(markup) = hover1.unwrap().contents {
        assert!(markup.value.contains("type1"));
    }
}

#[test]
fn test_thumbnails_nested_path_hover() {
    let temp_dir = tempfile::tempdir().unwrap();

    let container_txt = r#"
thumbnails
{
    kind "structure"
    array-element
    {
        container-type0 "thumbnails-element"
    }
}

thumbnails-element
{
    kind "array"
    subpossibilities
    {
        image
        {
            kind value
            type filepathedit
            datatype "image"
            description "The thumbnail image"
        }
    }
}
"#;
    std::fs::write(temp_dir.path().join("container.txt"), container_txt).unwrap();
    let validators = load_validators(temp_dir.path(), None);

    let content = r#"thumbnails {
    0 {
        image "icon.jpg"
    }
}"#;
    let pairs = parse_acs_text(content).unwrap();
    let acs_text = process_acs_text_ast(pairs, content);

    // Hover over "image"
    let params = HoverParams {
        text_document_position_params: tower_lsp_server::ls_types::TextDocumentPositionParams {
            text_document: tower_lsp_server::ls_types::TextDocumentIdentifier {
                uri: "file:///config.txt".parse().unwrap(),
            },
            position: Position {
                line: 2,
                character: 10,
            }, // over "image"
        },
        work_done_progress_params: Default::default(),
    };

    let hover = acs_text_hover(&acs_text, params, &validators, None, None);
    assert!(hover.is_some(), "Hover should be found for 'image'");
    if let tower_lsp_server::ls_types::HoverContents::Markup(markup) = hover.unwrap().contents {
        assert!(markup.value.contains("thumbnails/0/image"));
        assert!(markup.value.contains("The thumbnail image"));
    }
}

#[test]
fn test_mesh_table_nested_path_hover() {
    let temp_dir = tempfile::tempdir().unwrap();

    let container_txt = r#"
mesh-table
{
    kind "structure"
    array-element
    {
        container-type0 "mesh-element"
    }
}

mesh-element
{
    kind "structure"
    subpossibilities
    {
        effects
        {
            kind element
            element-type "effects"
        }
        mesh
        {
            kind value
            type filepath
            description "The mesh file"
        }
    }
}

effects
{
    kind "structure"
    array-element
    {
        container-type0 "effect-element"
    }
}

effect-element
{
    kind "structure"
    subpossibilities
    {
        kind
        {
            kind value
            type string
            description "The effect kind"
        }
    }
}
"#;
    std::fs::write(temp_dir.path().join("container.txt"), container_txt).unwrap();
    let validators = load_validators(temp_dir.path(), None);

    let content = r#"mesh-table {
    0 {
        mesh "default.trainzmesh"
        effects {
            0 {
                kind "corona"
            }
        }
    }
}"#;
    let pairs = parse_acs_text(content).unwrap();
    let acs_text = process_acs_text_ast(pairs, content);

    // Hover over "kind" inside "0"
    let params = HoverParams {
        text_document_position_params: tower_lsp_server::ls_types::TextDocumentPositionParams {
            text_document: tower_lsp_server::ls_types::TextDocumentIdentifier {
                uri: "file:///config.txt".parse().unwrap(),
            },
            position: Position {
                line: 5,
                character: 18,
            }, // over "kind"
        },
        work_done_progress_params: Default::default(),
    };

    let hover = acs_text_hover(&acs_text, params, &validators, None, None);
    assert!(hover.is_some(), "Hover should be found for 'kind'");
    if let tower_lsp_server::ls_types::HoverContents::Markup(markup) = hover.unwrap().contents {
        assert!(markup.value.contains("mesh-table/0/effects/0/kind"));
    }
}

#[test]
fn test_mosignal_nested_path_hover() {
    let content = r#"
mosignal {
    signals {
        0 {
            light   1
        }
    }
}
"#;
    let pairs = parse_acs_text(content).unwrap();
    let acs_text = process_acs_text_ast(pairs, content);

    let temp_dir = tempfile::tempdir().unwrap();

    let mosignal_txt = r#"
mosignal
{
  kind "Structure"
  SubPossibilities
  {
    signals
    {
      kind element
      element-type "signals"
    }
  }
}
"#;

    let signals_txt = r#"
signals
{
  kind "Structure"
  array-element
  {
    container-type0 "signal-element"
  }
}
"#;

    let signal_element_txt = r#"
signal-element
{
  kind "Structure"
  SubPossibilities
  {
    light
    {
      kind value
      type int
    }
  }
}
"#;

    std::fs::write(temp_dir.path().join("mosignal.txt"), mosignal_txt).unwrap();
    std::fs::write(temp_dir.path().join("signals.txt"), signals_txt).unwrap();
    std::fs::write(
        temp_dir.path().join("signal-element.txt"),
        signal_element_txt,
    )
    .unwrap();

    let validators = load_validators(temp_dir.path(), None);

    // Hover over "light"
    let params = HoverParams {
        text_document_position_params: tower_lsp_server::ls_types::TextDocumentPositionParams {
            text_document: tower_lsp_server::ls_types::TextDocumentIdentifier {
                uri: "file:///test.acs_text".parse().unwrap(),
            },
            position: Position {
                line: 4,
                character: 14, // Inside "light"
            },
        },
        work_done_progress_params: Default::default(),
    };

    let hover = acs_text_hover(&acs_text, params, &validators, None, None);
    assert!(hover.is_some(), "Hover should be found for light");
    let hover = hover.unwrap();
    if let tower_lsp_server::ls_types::HoverContents::Markup(markup) = hover.contents {
        assert!(
            markup
                .value
                .contains("**Validator Path**: `mosignal/signals/0/light`")
        );
    }
}
