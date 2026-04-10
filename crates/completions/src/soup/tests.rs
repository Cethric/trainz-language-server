use super::*;
use rayon::prelude::*;
use tower_lsp_server::ls_types::{Position, TextDocumentIdentifier, TextDocumentPositionParams};
use trainz_ast::soup::process::process_soup_ast;
use trainz_parser::soup::parse_soup;

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

    let soup_content = r#"kind "library"
category-era "2000s; "
"#;
    let pairs = parse_soup(soup_content).unwrap();
    let soup = process_soup_ast(pairs, soup_content);

    let params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: "file:///test.soup".parse().unwrap(),
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

    let validators = trainz_soup_validators::load_validators(&temp_dir);
    let completions = soup_completions(&soup, params, &validators);

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

    let soup_content = r#"
my_container {
    category-class "S"
}
"#;
    let pairs = parse_soup(soup_content).unwrap();
    let soup = process_soup_ast(pairs, soup_content);

    let params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: "file:///test.soup".parse().unwrap(),
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

    let validators = trainz_soup_validators::load_validators(&temp_dir);
    let completions = soup_completions(&soup, params, &validators);

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

    let soup_content_region = r#"
region_container {
    category-region ""
}
"#;
    let pairs_region = parse_soup(soup_content_region).unwrap();
    let soup_region = process_soup_ast(pairs_region, soup_content_region);

    let params_region = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: "file:///test_region.soup".parse().unwrap(),
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

    let validators_region = trainz_soup_validators::load_validators(&temp_dir);
    let completions_region = soup_completions(&soup_region, params_region, &validators_region);

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

    let soup_content_era = r#"
era_container {
    category-era ""
}
"#;
    let pairs_era = parse_soup(soup_content_era).unwrap();
    let soup_era = process_soup_ast(pairs_era, soup_content_era);

    let params_era = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: "file:///test_era.soup".parse().unwrap(),
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

    let validators_era = trainz_soup_validators::load_validators(&temp_dir);
    let completions_era = soup_completions(&soup_era, params_era, &validators_era);

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

    let soup_content = "kind \"lib\"";
    // Cursor at end of "lib" (kind "lib"|)
    // line 0, char 0-3 is "kind"
    // line 0, char 4 is " "
    // line 0, char 5 is "\""
    // line 0, char 6-8 is "lib"
    // line 0, char 9 is "\""
    // Cursor should be at char 8 or 9. Let's try 8.

    let pairs = parse_soup(soup_content).unwrap();
    let soup = process_soup_ast(pairs, soup_content);

    let params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: "file:///test.soup".parse().unwrap(),
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

    let validators = trainz_soup_validators::load_validators(&temp_dir);
    let completions = soup_completions(&soup, params, &validators);

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

    let soup_content = "my_container {\n    kind \"my_container\"\n    category-class \n}";
    // Line 1, char 19 is after "category-class "

    let pairs = parse_soup(soup_content).unwrap();
    let soup = process_soup_ast(pairs, soup_content);

    let params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: "file:///test.soup".parse().unwrap(),
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

    let validators = trainz_soup_validators::load_validators(&temp_dir);
    let completions = soup_completions(&soup, params, &validators);
    eprintln!("Completions: {:?}", completions);
    eprintln!("Soup KeyValuePairs: {:?}", soup.key_value_pairs);

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

    let soup_content = "my_container {\n    category-class\n}";

    let pairs = parse_soup(soup_content).unwrap();
    let soup = process_soup_ast(pairs, soup_content);

    let params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: "file:///test.soup".parse().unwrap(),
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

    let validators = trainz_soup_validators::load_validators(&temp_dir);
    let completions = soup_completions(&soup, params, &validators);

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

    let soup_content = "kind \"\"";
    let pairs = parse_soup(soup_content).unwrap();
    let soup = process_soup_ast(pairs, soup_content);

    let params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: "file:///test.soup".parse().unwrap(),
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

    let validators = trainz_soup_validators::load_validators(&temp_dir);
    let completions = soup_completions(&soup, params, &validators);
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

    let soup_content = "kind ";
    let pairs = parse_soup(soup_content).unwrap();
    let soup = process_soup_ast(pairs, soup_content);

    let params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: "file:///test.soup".parse().unwrap(),
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

    let validators = trainz_soup_validators::load_validators(&temp_dir);
    let completions = soup_completions(&soup, params, &validators);
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

    let soup_content = "my_container {\n    kind \"\"\n}";
    let pairs = parse_soup(soup_content).unwrap();
    let soup = process_soup_ast(pairs, soup_content);

    let params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: "file:///test.soup".parse().unwrap(),
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

    let validators = trainz_soup_validators::load_validators(&temp_dir);
    let completions = soup_completions(&soup, params, &validators);
    let labels: Vec<String> = completions.par_iter().map(|c| c.label.clone()).collect();

    assert!(
        labels.contains(&"library".to_string()),
        "Nested kind should suggest 'library', got {:?}",
        labels
    );
    assert!(labels.contains(&"scenery".to_string()));

    std::fs::remove_dir_all(&temp_dir).unwrap();
}
