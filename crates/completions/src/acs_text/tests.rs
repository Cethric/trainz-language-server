use super::*;
use rayon::prelude::*;
use tower_lsp_server::ls_types::{Position, TextDocumentIdentifier, TextDocumentPositionParams};
use trainz_acs_text_validators::load_validators;
use trainz_ast::acs_text::process::process_acs_text_ast;
use trainz_parser::acs_text::parse_acs_text;

#[test]
fn test_multivalue_completions() {
    let temp_dir = std::env::temp_dir().join("language-server-test-multi-comp");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let cat_era_content = r#"
1980s "1980s era"
2000s "2000s era"
2010s "2010s era"
"#;
    std::fs::write(temp_dir.join("category-era.txt"), cat_era_content).unwrap();

    let container_content = r#"
library {
    category-era {
        type string
        validation IsValidCategoryEra
    }
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_content).unwrap();

    let acs_text_content = r#"kind "library"
category-era "2000s; "
"#;
    let pairs = parse_acs_text(acs_text_content).unwrap();
    let acs_text = process_acs_text_ast(pairs, acs_text_content);

    let params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: "file:///test.acs_text".parse().unwrap(),
            },
            position: Position {
                line: 1,
                character: 21,
            }, // After "2000s; " (line 0 is kind, line 1 is category-era)
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: None,
    };

    let validators = load_validators(&temp_dir, None);
    let completions = acs_text_completions(&acs_text, params, &validators, None);

    println!("Completions for '2000s; ': {:?}", completions);

    let era1980s = completions.par_iter().find_first(|c| c.label == "1980s");
    assert!(era1980s.is_some(), "Should suggest '1980s'");

    let era2010s = completions.par_iter().find_first(|c| c.label == "2010s");
    assert!(era2010s.is_some(), "Should suggest '2010s'");

    // It shouldn't suggest 2000s if we already have it, or at least it should be there if we haven't implemented filtering
    let era2000s = completions.par_iter().find_first(|c| c.label == "2000s");
    assert!(
        era2000s.is_none(),
        "Should NOT suggest '2000s era' as it is already selected"
    );
}

#[test]
fn test_category_class_completions() {
    let temp_dir = std::env::temp_dir().join("language-server-test-cat-comp");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let cat_class_content = r#"
Scenery "Scenery objects"
Track "Track objects"
"#;
    std::fs::write(temp_dir.join("category-class.txt"), cat_class_content).unwrap();

    let container_content = r#"
my_container {
    category-class {
        type string
        validation IsValidCategoryClass
    }
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_content).unwrap();

    let acs_text_content = r#"
my_container {
    category-class "S"
}
"#;
    let pairs = parse_acs_text(acs_text_content).unwrap();
    let acs_text = process_acs_text_ast(pairs, acs_text_content);

    let params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: "file:///test.acs_text".parse().unwrap(),
            },
            position: Position {
                line: 2,
                character: 20,
            }, // Inside the "S"
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: None,
    };

    let validators = load_validators(&temp_dir, None);
    let completions = acs_text_completions(&acs_text, params, &validators, None);

    assert!(!completions.is_empty(), "Completions should not be empty");

    println!("Completions: {:?}", completions);

    let scenery_comp = completions
        .par_iter()
        .find_first(|c| c.detail == Some(String::from("Scenery objects")));
    assert!(scenery_comp.is_some(), "Should suggest 'Scenery objects'");
    assert_eq!(scenery_comp.unwrap().label, "Scenery");
    assert_eq!(
        scenery_comp.unwrap().insert_text,
        Some("Scenery".to_string())
    );

    let track_comp = completions
        .par_iter()
        .find_first(|c| c.detail == Some(String::from("Track objects")));
    assert!(track_comp.is_some(), "Should suggest 'Track objects'");
    assert_eq!(track_comp.unwrap().label, "Track");
    assert_eq!(track_comp.unwrap().insert_text, Some("Track".to_string()));

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_category_region_completions() {
    let temp_dir = std::env::temp_dir().join("language-server-test-cat-region");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let cat_region_content = r#"
FRA "France"
USA "United States"
"#;
    std::fs::write(temp_dir.join("category-region.txt"), cat_region_content).unwrap();

    let container_content_region = r#"
region_container {
    category-region {
        type string
        validation IsValidCategoryRegion
    }
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_content_region).unwrap();

    let acs_text_content_region = r#"
region_container {
    category-region ""
}
"#;
    let pairs_region = parse_acs_text(acs_text_content_region).unwrap();
    let acs_text_region = process_acs_text_ast(pairs_region, acs_text_content_region);

    let params_region = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: "file:///test_region.acs_text".parse().unwrap(),
            },
            position: Position {
                line: 2,
                character: 20,
            },
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: None,
    };

    let validators = load_validators(&temp_dir, None);
    let completions_region =
        acs_text_completions(&acs_text_region, params_region, &validators, None);

    assert!(
        !completions_region.is_empty(),
        "Region completions should not be empty"
    );

    let fra_comp = completions_region
        .par_iter()
        .find_first(|c| c.label == "FRA");
    assert!(fra_comp.is_some(), "Should suggest 'France'");
    assert_eq!(fra_comp.unwrap().detail, Some("France".to_string()));
    assert_eq!(fra_comp.unwrap().insert_text, Some("FRA".to_string()));

    let usa_comp = completions_region
        .par_iter()
        .find_first(|c| c.label == "USA");
    assert!(usa_comp.is_some(), "Should suggest 'United States'");
    assert_eq!(usa_comp.unwrap().detail, Some("United States".to_string()));
    assert_eq!(usa_comp.unwrap().insert_text, Some("USA".to_string()));

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_category_era_completions() {
    let temp_dir = std::env::temp_dir().join("language-server-test-cat-era");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let cat_era_content = r#"
2000s "2000s era"
2010s "2010s era"
"#;
    std::fs::write(temp_dir.join("category-era.txt"), cat_era_content).unwrap();

    let container_content_era = r#"
era_container {
    category-era {
        type string
        validation IsValidCategoryEra
    }
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_content_era).unwrap();

    let acs_text_content_era = r#"
era_container {
    category-era ""
}
"#;
    let pairs_era = parse_acs_text(acs_text_content_era).unwrap();
    let acs_text_era = process_acs_text_ast(pairs_era, acs_text_content_era);

    let params_era = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: "file:///test_era.acs_text".parse().unwrap(),
            },
            position: Position {
                line: 2,
                character: 18,
            },
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: None,
    };

    let validators = load_validators(&temp_dir, None);
    let completions_era = acs_text_completions(&acs_text_era, params_era, &validators, None);

    assert!(
        !completions_era.is_empty(),
        "Era completions should not be empty"
    );

    let s2000_comp = completions_era
        .par_iter()
        .find_first(|c| c.label == "2000s");
    assert!(s2000_comp.is_some(), "Should suggest '2000s era'");
    assert_eq!(s2000_comp.unwrap().detail, Some("2000s era".to_string()));
    assert_eq!(s2000_comp.unwrap().insert_text, Some("2000s".to_string()));

    let s2010_comp = completions_era
        .par_iter()
        .find_first(|c| c.label == "2010s");
    assert!(s2010_comp.is_some(), "Should suggest '2010s era'");
    assert_eq!(s2010_comp.unwrap().detail, Some("2010s era".to_string()));
    assert_eq!(s2010_comp.unwrap().insert_text, Some("2010s".to_string()));

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_kind_lib_correction_completion() {
    let temp_dir = std::env::temp_dir().join("language-server-test-kind-lib");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let container_content = r#"
library {
    top-level true
}
"#;
    std::fs::write(temp_dir.join("kind.txt"), container_content).unwrap();

    let acs_text_content = "kind \"lib\"";
    // Cursor at end of "lib" (kind "lib"|)
    // line 0, char 0-3 is "kind"
    // line 0, char 4 is " "
    // line 0, char 5 is "\""
    // line 0, char 6-8 is "lib"
    // line 0, char 9 is "\""
    // Cursor should be at char 8 or 9. Let's try 8.

    let pairs = parse_acs_text(acs_text_content).unwrap();
    let acs_text = process_acs_text_ast(pairs, acs_text_content);

    let params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: "file:///test.acs_text".parse().unwrap(),
            },
            position: Position {
                line: 0,
                character: 7,
            },
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: None,
    };

    let validators = load_validators(&temp_dir, None);
    let completions = acs_text_completions(&acs_text, params, &validators, None);

    // Should NOT suggest "lib"
    assert!(
        !completions.par_iter().any(|c| c.label == "lib"),
        "Should NOT suggest 'lib' as it is invalid"
    );
    // SHOULD suggest "library"
    assert!(
        completions.par_iter().any(|c| c.label == "library"),
        "Should suggest 'library' as a correction for 'lib'"
    );

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_completion_after_key() {
    let temp_dir = std::env::temp_dir().join("language-server-test-after-key");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let container_content = r#"
my_container {
    category-class {
        type string
        validation IsValidCategoryClass
    }
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_content).unwrap();
    std::fs::write(temp_dir.join("category-class.txt"), "Scenery \"Desc\"").unwrap();

    let acs_text_content = "my_container {\n    kind \"my_container\"\n    category-class \n}";
    // Line 1, char 19 is after "category-class "

    let pairs = parse_acs_text(acs_text_content).unwrap();
    let acs_text = process_acs_text_ast(pairs, acs_text_content);

    let params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: "file:///test.acs_text".parse().unwrap(),
            },
            position: Position {
                line: 2,
                character: 19,
            },
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: None,
    };

    let validators = load_validators(&temp_dir, None);
    let completions = acs_text_completions(&acs_text, params, &validators, None);
    eprintln!("Completions: {:?}", completions);
    eprintln!("AcsText KeyValuePairs: {:?}", acs_text.key_value_pairs);

    assert!(
        !completions.is_empty(),
        "Completions should not be empty after key"
    );
    assert!(
        completions.par_iter().any(|c| c.label == "Scenery"),
        "Should suggest 'Scenery'"
    );
    assert!(
        completions
            .par_iter()
            .any(|c| c.insert_text == Some("Scenery".to_string())),
        "Should suggest 'Scenery' as insert text"
    );

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_completion_at_end_of_key() {
    let temp_dir = std::env::temp_dir().join("language-server-test-end-of-key");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let container_content = r#"
my_container {
    category-class {
        type string
        validation IsValidCategoryClass
    }
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_content).unwrap();
    std::fs::write(temp_dir.join("category-class.txt"), "Scenery \"Desc\"").unwrap();

    let acs_text_content = "my_container {\n    category-class\n}";

    let pairs = parse_acs_text(acs_text_content).unwrap();
    let acs_text = process_acs_text_ast(pairs, acs_text_content);

    let params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: "file:///test.acs_text".parse().unwrap(),
            },
            position: Position {
                line: 1,
                character: 18, // Exactly at the end of "category-class"
            },
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: None,
    };

    let validators = load_validators(&temp_dir, None);
    let completions = acs_text_completions(&acs_text, params, &validators, None);

    assert!(
        !completions.is_empty(),
        "Completions should not be empty at end of key"
    );
    assert!(
        completions.par_iter().any(|c| c.label == "Scenery"),
        "Should suggest 'Scenery'"
    );
    assert!(
        completions
            .par_iter()
            .any(|c| c.insert_text == Some("Scenery".to_string())),
        "Should suggest 'Scenery' as insert text"
    );

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

fn create_kind_validators(temp_dir: &std::path::Path, kind_content: &str) {
    if temp_dir.exists() {
        std::fs::remove_dir_all(temp_dir).unwrap();
    }
    std::fs::create_dir_all(temp_dir).unwrap();
    std::fs::write(temp_dir.join("kind.txt"), kind_content).unwrap();
}

#[test]
fn test_kind_completions_top_level_value() {
    let temp_dir = std::env::temp_dir().join("language-server-test-kind-top-level");
    let kind_content = r#"
library {
    top-level true
}
scenery {
    top-level true
}
track {
    top-level true
}
not-top-level {
    top-level false
}
"#;
    create_kind_validators(&temp_dir, kind_content);

    let acs_text_content = "kind \"\"";
    let pairs = parse_acs_text(acs_text_content).unwrap();
    let acs_text = process_acs_text_ast(pairs, acs_text_content);

    let params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: "file:///test.acs_text".parse().unwrap(),
            },
            position: Position {
                line: 0,
                character: 6,
            },
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: None,
    };

    let validators = load_validators(&temp_dir, None);
    let completions = acs_text_completions(&acs_text, params, &validators, None);
    let labels: Vec<String> = completions.par_iter().map(|c| c.label.clone()).collect();

    assert!(labels.contains(&"library".to_string()));
    assert!(labels.contains(&"scenery".to_string()));
    assert!(labels.contains(&"track".to_string()));
    assert!(!labels.contains(&"not-top-level".to_string()));

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_kind_completions_no_value() {
    let temp_dir = std::env::temp_dir().join("language-server-test-kind-no-value");
    let kind_content = r#"
library {
    top-level true
}
scenery {
    top-level true
}
track {
    top-level true
}
"#;
    create_kind_validators(&temp_dir, kind_content);

    let acs_text_content = "kind ";
    let pairs = parse_acs_text(acs_text_content).unwrap();
    let acs_text = process_acs_text_ast(pairs, acs_text_content);

    let params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: "file:///test.acs_text".parse().unwrap(),
            },
            position: Position {
                line: 0,
                character: 5,
            },
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: None,
    };

    let validators = load_validators(&temp_dir, None);
    let completions = acs_text_completions(&acs_text, params, &validators, None);
    let labels: Vec<String> = completions.par_iter().map(|c| c.label.clone()).collect();

    assert!(labels.contains(&"library".to_string()));
    assert!(labels.contains(&"scenery".to_string()));
    assert!(labels.contains(&"track".to_string()));

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_kind_completions_nested_container() {
    let temp_dir = std::env::temp_dir().join("language-server-test-kind-nested");
    let kind_content = r#"
library {
    top-level true
}
scenery {
    top-level true
}
"#;
    create_kind_validators(&temp_dir, kind_content);

    let acs_text_content = "my_container {\n    kind \"\"\n}";
    let pairs = parse_acs_text(acs_text_content).unwrap();
    let acs_text = process_acs_text_ast(pairs, acs_text_content);

    let params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: "file:///test.acs_text".parse().unwrap(),
            },
            position: Position {
                line: 1,
                character: 10,
            },
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: None,
    };

    let validators = load_validators(&temp_dir, None);
    let completions = acs_text_completions(&acs_text, params, &validators, None);
    let labels: Vec<String> = completions.par_iter().map(|c| c.label.clone()).collect();

    assert!(
        labels.contains(&"library".to_string()),
        "Nested kind should suggest 'library', got {:?}",
        labels
    );
    assert!(labels.contains(&"scenery".to_string()));

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_array_element_multi_type_completions() {
    let temp_dir = std::env::temp_dir().join(format!(
        "acs_text_array_compl_test_{:?}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&temp_dir).unwrap();

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
    std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();
    let validators = load_validators(&temp_dir, None);

    // Completion inside element 0
    let content0 = "my-container {\n    0 {\n        \n    }\n}";
    let pairs0 = parse_acs_text(content0).unwrap();
    let acs_text0 = process_acs_text_ast(pairs0, content0);
    let params0 = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: "file:///test.acs_text".parse().unwrap(),
            },
            position: Position {
                line: 2,
                character: 8,
            },
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: None,
    };
    let completions0 = acs_text_completions(&acs_text0, params0, &validators, None);
    let labels0: Vec<String> = completions0.iter().map(|c| c.label.clone()).collect();
    assert!(
        labels0.contains(&"v0".to_string()),
        "Expected v0 in element 0 completions, got {:?}",
        labels0
    );
    assert!(!labels0.contains(&"v1".to_string()));

    // Completion inside element 1
    let content1 = "my-container {\n    0 { v0 1 }\n    1 {\n        \n    }\n}";
    let pairs1 = parse_acs_text(content1).unwrap();
    let acs_text1 = process_acs_text_ast(pairs1, content1);
    let params1 = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: "file:///test.acs_text".parse().unwrap(),
            },
            position: Position {
                line: 3,
                character: 8,
            },
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: None,
    };
    let completions1 = acs_text_completions(&acs_text1, params1, &validators, None);
    let labels1: Vec<String> = completions1.iter().map(|c| c.label.clone()).collect();
    assert!(
        labels1.contains(&"v1".to_string()),
        "Expected v1 in element 1 completions, got {:?}",
        labels1
    );
    assert!(!labels1.contains(&"v0".to_string()));

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_tag_array_multi_type_completions() {
    let temp_dir = std::env::temp_dir().join(format!(
        "acs_text_tag_array_compl_test_{:?}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&temp_dir).unwrap();

    let container_txt = r#"
my-container {
    kind "container"
    top-level 1
    tagarray {
        container-type0 "type0"
        container-type1 "type1"
    }
}
type0 { kind "container" v0 { type "int" } }
type1 { kind "container" v1 { type "int" } }
"#;
    std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();
    let validators = load_validators(&temp_dir, None);

    // Completion inside 1st element (order based)
    let content0 = "my-container {\n    foo {\n        \n    }\n}";
    let pairs0 = parse_acs_text(content0).unwrap();
    let acs_text0 = process_acs_text_ast(pairs0, content0);
    let params0 = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: "file:///test.acs_text".parse().unwrap(),
            },
            position: Position {
                line: 2,
                character: 8,
            },
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: None,
    };
    let completions0 = acs_text_completions(&acs_text0, params0, &validators, None);
    let labels0: Vec<String> = completions0.iter().map(|c| c.label.clone()).collect();
    assert!(
        labels0.contains(&"v0".to_string()),
        "Expected v0 in 1st element completions, got {:?}",
        labels0
    );

    // Completion inside 2nd element
    let content1 = "my-container {\n    foo { v0 1 }\n    bar {\n        \n    }\n}";
    let pairs1 = parse_acs_text(content1).unwrap();
    let acs_text1 = process_acs_text_ast(pairs1, content1);
    let params1 = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: "file:///test.acs_text".parse().unwrap(),
            },
            position: Position {
                line: 3,
                character: 8,
            },
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: None,
    };
    let completions1 = acs_text_completions(&acs_text1, params1, &validators, None);
    let labels1: Vec<String> = completions1.iter().map(|c| c.label.clone()).collect();
    assert!(
        labels1.contains(&"v1".to_string()),
        "Expected v1 in 2nd element completions, got {:?}",
        labels1
    );

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_inline_nested_validator_completions() {
    let temp_dir = std::env::temp_dir().join("language-server-test-inline-nested");
    if temp_dir.exists() {
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let container_content = r#"
example {
  nested {
    type "container"
    value {
      type "string"
      validation "IsValidValue"
    }
  }
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_content).unwrap();

    let is_valid_value_content = r#"
val1 "Value 1"
val2 "Value 2"
"#;
    let is_valid_value_path = temp_dir.join("isvalidvalue.txt");
    std::fs::write(is_valid_value_path, is_valid_value_content).unwrap();

    let acs_text_content = "example {\n  nested {\n    value \"\"\n  }\n}";
    let pairs = parse_acs_text(acs_text_content).unwrap();
    let acs_text = process_acs_text_ast(pairs, acs_text_content);

    let params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: "file:///test.acs_text".parse().unwrap(),
            },
            position: Position {
                line: 2,
                character: 11, // Inside value "" of 'value'
            },
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: None,
    };

    let validators = load_validators(&temp_dir, None);
    let completions = acs_text_completions(&acs_text, params, &validators, None);
    let labels: Vec<String> = completions.par_iter().map(|c| c.label.clone()).collect();

    assert!(
        labels.contains(&"val1".to_string()),
        "Should suggest 'val1' from inline nested validator, got {:?}",
        labels
    );
    assert!(labels.contains(&"val2".to_string()));

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_tagarray_nested_validator_completions() {
    let temp_dir = std::env::temp_dir().join("language-server-test-tagarray-nested");
    if temp_dir.exists() {
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let container_content = r#"
extensions {
  kind "container"
  top-level 1
  tagarray {
    foo { type "string" }
  }
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_content).unwrap();

    let acs_text_content = "extensions {\n  my-ext {\n    \n  }\n}";
    let pairs = parse_acs_text(acs_text_content).unwrap();
    let acs_text = process_acs_text_ast(pairs, acs_text_content);

    let params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: "file:///test.acs_text".parse().unwrap(),
            },
            position: Position {
                line: 2,
                character: 4, // Inside 'my-ext' container
            },
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: None,
    };

    let validators = load_validators(&temp_dir, None);
    let completions = acs_text_completions(&acs_text, params, &validators, None);
    let labels: Vec<String> = completions.par_iter().map(|c| c.label.clone()).collect();

    assert!(
        labels.contains(&"foo".to_string()),
        "Should suggest 'foo' from nested tagarray validator, got {:?}",
        labels
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_array_element_nested_validator_completions() {
    let temp_dir = std::env::temp_dir().join("language-server-test-array-element-nested");
    if temp_dir.exists() {
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let container_content = r#"
my-array {
  kind "container"
  top-level 1
  array-element {
    foo { type "string" }
  }
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_content).unwrap();

    let acs_text_content = "my-array {\n  0 {\n    \n  }\n}";
    let pairs = parse_acs_text(acs_text_content).unwrap();
    let acs_text = process_acs_text_ast(pairs, acs_text_content);

    let params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: "file:///test.acs_text".parse().unwrap(),
            },
            position: Position {
                line: 2,
                character: 4, // Inside '0' container
            },
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: None,
    };

    let validators = load_validators(&temp_dir, None);
    let completions = acs_text_completions(&acs_text, params, &validators, None);
    let labels: Vec<String> = completions.par_iter().map(|c| c.label.clone()).collect();

    assert!(
        labels.contains(&"foo".to_string()),
        "Should suggest 'foo' from nested array-element validator, got {:?}",
        labels
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}
