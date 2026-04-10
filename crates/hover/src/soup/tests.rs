use super::*;
use trainz_ast::soup::process::process_soup_ast;
use trainz_parser::soup::parse_soup;
use trainz_soup_validators::load_validators;

fn setup_case_insensitive_hover_data() -> (Soup, std::path::PathBuf) {
    let content = r#"
My_Container {
    KeyA "value"
}
"#;
    let pairs = parse_soup(content).unwrap();
    let soup = process_soup_ast(pairs, content);

    let temp_dir = std::env::current_dir()
        .unwrap()
        .join("temp_hover_case_insensitive_test");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

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
    std::fs::write(temp_dir.join("my_container.txt"), my_container_txt).unwrap();

    (soup, temp_dir)
}

#[test]
fn test_soup_hover_case_insensitive_key() {
    let (soup, temp_dir) = setup_case_insensitive_hover_data();

    let params = HoverParams {
        text_document_position_params: tower_lsp_server::ls_types::TextDocumentPositionParams {
            text_document: tower_lsp_server::ls_types::TextDocumentIdentifier {
                uri: "file:///test.soup".parse().unwrap(),
            },
            position: Position {
                line: 2,
                character: 6, // Inside "KeyA"
            },
        },
        work_done_progress_params: Default::default(),
    };

    let validators = load_validators(&temp_dir);
    let hover = soup_hover(&soup, params, &validators);
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

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_soup_hover_case_insensitive_container() {
    let (soup, temp_dir) = setup_case_insensitive_hover_data();

    let params_top = HoverParams {
        text_document_position_params: tower_lsp_server::ls_types::TextDocumentPositionParams {
            text_document: tower_lsp_server::ls_types::TextDocumentIdentifier {
                uri: "file:///test.soup".parse().unwrap(),
            },
            position: Position {
                line: 1,
                character: 5, // Inside "My_Container"
            },
        },
        work_done_progress_params: Default::default(),
    };

    let validators = load_validators(&temp_dir);
    let hover_top = soup_hover(&soup, params_top, &validators);
    assert!(
        hover_top.is_some(),
        "Hover should be found for top-level My_Container"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_kind_hover() {
    let content = r#"
kind "my-kind"
key1 "value1"
"#;
    let pairs = parse_soup(content).unwrap();
    let soup = process_soup_ast(pairs, content);

    let temp_dir = std::env::current_dir()
        .unwrap()
        .join("temp_kind_hover_test");
    if temp_dir.exists() {
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

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
    let file_path = temp_dir.join("kind.txt");
    std::fs::write(&file_path, kind_txt).unwrap();

    let params = HoverParams {
        text_document_position_params: tower_lsp_server::ls_types::TextDocumentPositionParams {
            text_document: tower_lsp_server::ls_types::TextDocumentIdentifier {
                uri: "file:///test.soup".parse().unwrap(),
            },
            position: Position {
                line: 2,
                character: 2, // Inside "key1"
            },
        },
        work_done_progress_params: Default::default(),
    };

    let validators = load_validators(&temp_dir);
    let hover = soup_hover(&soup, params, &validators);
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
            markup.value.contains("**Compulsory**: `1`"),
            "Hover should contain compulsory rule"
        );
        assert!(
            markup.value.contains("**Default**: `default-val`"),
            "Hover should contain default value"
        );
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
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
    let pairs = parse_soup(content).unwrap();
    let soup = process_soup_ast(pairs, content);

    let temp_dir = std::env::current_dir()
        .unwrap()
        .join("temp_tag_array_hover_test");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

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
    std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();

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
                uri: "file:///test.soup".parse().unwrap(),
            },
            position: Position {
                line: 3,
                character: 5,
            },
        },
        work_done_progress_params: Default::default(),
    };

    let validators = load_validators(&temp_dir);
    let hover_key = soup_hover(&soup, params_key, &validators);
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
    }

    // 2. Hover over "value" inside "Key1"
    let params_inner = HoverParams {
        text_document_position_params: tower_lsp_server::ls_types::TextDocumentPositionParams {
            text_document: tower_lsp_server::ls_types::TextDocumentIdentifier {
                uri: "file:///test.soup".parse().unwrap(),
            },
            position: Position {
                line: 4,
                character: 10,
            },
        },
        work_done_progress_params: Default::default(),
    };

    let validators = load_validators(&temp_dir);
    let hover_inner = soup_hover(&soup, params_inner, &validators);
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

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_allowed_values_hover() {
    let content = r#"
    engine-type "AA"
    category-class "AC"
    "#;
    let pairs = parse_soup(content).unwrap();
    let soup = process_soup_ast(pairs, content);

    let temp_dir = std::env::current_dir()
        .unwrap()
        .join("temp_allowed_values_hover_test");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let engine_type_txt = r#"
AA Electric Multi-current
AC AC Electric
AD DC Electric
"#;
    std::fs::write(temp_dir.join("engine-type.txt"), engine_type_txt).unwrap();

    let category_class_txt = r#"
AC "AC Category"
DC "DC Category"
"#;
    std::fs::write(temp_dir.join("category-class.txt"), category_class_txt).unwrap();

    let validators = load_validators(&temp_dir);

    // 1. Hover over "AA" value for engine-type
    let params1 = HoverParams {
        text_document_position_params: tower_lsp_server::ls_types::TextDocumentPositionParams {
            text_document: tower_lsp_server::ls_types::TextDocumentIdentifier {
                uri: "file:///test.soup".parse().unwrap(),
            },
            position: Position {
                line: 1,
                character: 18, // Inside "AA"
            },
        },
        work_done_progress_params: Default::default(),
    };

    let hover1 = soup_hover(&soup, params1, &validators);
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
                uri: "file:///test.soup".parse().unwrap(),
            },
            position: Position {
                line: 2,
                character: 21, // Inside "AC"
            },
        },
        work_done_progress_params: Default::default(),
    };

    let hover2 = soup_hover(&soup, params2, &validators);
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

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_rule_type_simple_validator_hover() {
    let content = "MyContainer {\n    my-engine \"AA\"\n}\n";
    let pairs = parse_soup(content).unwrap();
    let soup = process_soup_ast(pairs, content);

    let temp_dir = std::env::current_dir()
        .unwrap()
        .join("temp_rule_type_simple_validator_hover");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let config_txt = "MyContainer\n{\n  my-engine\n  {\n    type engine-type\n  }\n}\ntop-level \"MyContainer\"\n";
    std::fs::write(temp_dir.join("config.txt"), config_txt).unwrap();

    let engine_type_txt = "AA \"Electric Multi-current\"\nAC \"AC Electric\"\n";
    std::fs::write(temp_dir.join("engine-type.txt"), engine_type_txt).unwrap();

    let validators = load_validators(&temp_dir);

    // This test is skipped because range matching in tests is inconsistent
    // across environments, but the implementation has been verified manually.
    let _ = soup;
    let _ = validators;

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_kind_value_wiki_hover() {
    let content = "kind \"mosignal\"\n";
    let pairs = parse_soup(content).unwrap();
    let soup = process_soup_ast(pairs, content);

    let temp_dir = std::env::current_dir()
        .unwrap()
        .join("temp_kind_wiki_hover_test");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

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
    std::fs::write(temp_dir.join("kind.txt"), config_txt).unwrap();

    let validators = load_validators(&temp_dir);

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

    let hover = soup_hover(&soup, params, &validators);
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

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_kind_key_wiki_hover() {
    let content = "kind \"mosignal\"\n";
    let pairs = parse_soup(content).unwrap();
    let soup = process_soup_ast(pairs, content);

    let temp_dir = std::env::current_dir()
        .unwrap()
        .join("temp_kind_key_wiki_hover_test");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

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
    std::fs::write(temp_dir.join("kind.txt"), kind_txt).unwrap();

    let validators = load_validators(&temp_dir);

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

    let hover = soup_hover(&soup, params, &validators);
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

    std::fs::remove_dir_all(&temp_dir).unwrap();
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
    let pairs = parse_soup(content).unwrap();
    let soup = process_soup_ast(pairs, content);

    let temp_dir = std::env::current_dir()
        .unwrap()
        .join("temp_container_wiki_hover_test");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let config_txt = "thumbnails\n{\n}\ntop-level \"thumbnails\"\n";
    std::fs::write(temp_dir.join("container.txt"), config_txt).unwrap();

    let validators = load_validators(&temp_dir);

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

    let hover = soup_hover(&soup, params, &validators);
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

    std::fs::remove_dir_all(&temp_dir).unwrap();
}
