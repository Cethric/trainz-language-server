use super::*;
use tower_lsp_server::ls_types::{Position, TextDocumentIdentifier, TextDocumentPositionParams};
use trainz_ast::soup::process::process_soup_ast;
use trainz_parser::soup::parse_soup;

#[test]
fn test_multivalue_completions() {
    let temp_dir = std::env::temp_dir().join("trainz-lsp-test-multi-comp");
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

    let era1980s = completions.iter().find(|c| c.label == "1980s era");
    assert!(era1980s.is_some(), "Should suggest '1980s era'");

    let era2010s = completions.iter().find(|c| c.label == "2010s era");
    assert!(era2010s.is_some(), "Should suggest '2010s era'");

    // It shouldn't suggest 2000s if we already have it, or at least it should be there if we haven't implemented filtering
    let era2000s = completions.iter().find(|c| c.label == "2000s era");
    assert!(
        era2000s.is_none(),
        "Should NOT suggest '2000s era' as it is already selected"
    );
}

#[test]
fn test_category_class_completions() {
    let temp_dir = std::env::temp_dir().join("trainz-lsp-test-cat-comp");
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

    let scenery_comp = completions.iter().find(|c| c.label == "Scenery objects");
    assert!(scenery_comp.is_some(), "Should suggest 'Scenery objects'");
    assert_eq!(scenery_comp.unwrap().detail, Some("Scenery".to_string()));
    assert_eq!(
        scenery_comp.unwrap().insert_text,
        Some("Scenery".to_string())
    );

    let track_comp = completions.iter().find(|c| c.label == "Track objects");
    assert!(track_comp.is_some(), "Should suggest 'Track objects'");
    assert_eq!(track_comp.unwrap().detail, Some("Track".to_string()));
    assert_eq!(track_comp.unwrap().insert_text, Some("Track".to_string()));

    // Test category-region completions
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
            }, // Inside the ""
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

    let fra_comp = completions_region.iter().find(|c| c.label == "France");
    assert!(fra_comp.is_some(), "Should suggest 'France'");
    assert_eq!(fra_comp.unwrap().detail, Some("FRA".to_string()));
    assert_eq!(fra_comp.unwrap().insert_text, Some("FRA".to_string()));

    let usa_comp = completions_region
        .iter()
        .find(|c| c.label == "United States");
    assert!(usa_comp.is_some(), "Should suggest 'United States'");
    assert_eq!(usa_comp.unwrap().detail, Some("USA".to_string()));
    assert_eq!(usa_comp.unwrap().insert_text, Some("USA".to_string()));

    // Test category-era completions
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
            }, // Inside the ""
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

    let s2000_comp = completions_era.iter().find(|c| c.label == "2000s era");
    assert!(s2000_comp.is_some(), "Should suggest '2000s era'");
    assert_eq!(s2000_comp.unwrap().detail, Some("2000s".to_string()));
    assert_eq!(s2000_comp.unwrap().insert_text, Some("2000s".to_string()));

    let s2010_comp = completions_era.iter().find(|c| c.label == "2010s era");
    assert!(s2010_comp.is_some(), "Should suggest '2010s era'");
    assert_eq!(s2010_comp.unwrap().detail, Some("2010s".to_string()));
    assert_eq!(s2010_comp.unwrap().insert_text, Some("2010s".to_string()));

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_kind_lib_correction_completion() {
    let temp_dir = std::env::temp_dir().join("trainz-lsp-test-kind-lib");
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
        !completions.iter().any(|c| c.label == "lib"),
        "Should NOT suggest 'lib' as it is invalid"
    );
    // SHOULD suggest "library"
    assert!(
        completions.iter().any(|c| c.label == "library"),
        "Should suggest 'library' as a correction for 'lib'"
    );

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_completion_after_key() {
    let temp_dir = std::env::temp_dir().join("trainz-lsp-test-after-key");
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
        completions.iter().any(|c| c.label == "Desc"),
        "Should suggest 'Desc'"
    );
    assert!(
        completions
            .iter()
            .any(|c| c.insert_text == Some("Scenery".to_string())),
        "Should suggest 'Scenery' as insert text"
    );

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_completion_at_end_of_key() {
    let temp_dir = std::env::temp_dir().join("trainz-lsp-test-end-of-key");
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
        completions.iter().any(|c| c.label == "Desc"),
        "Should suggest 'Desc'"
    );
    assert!(
        completions
            .iter()
            .any(|c| c.insert_text == Some("Scenery".to_string())),
        "Should suggest 'Scenery' as insert text"
    );

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_kind_completions() {
    let temp_dir = std::env::temp_dir().join("trainz-lsp-test-kind-completions");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    // Create a kind.txt with some top-level containers
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
    std::fs::write(temp_dir.join("kind.txt"), kind_content).unwrap();

    let validators = trainz_soup_validators::load_validators(&temp_dir);

    // Test 1: Typing value of "kind" at top level
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
                character: 6, // Inside the quotes of kind ""
            },
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: None,
    };

    let completions = soup_completions(&soup, params.clone(), &validators);
    let labels: Vec<String> = completions.iter().map(|c| c.label.clone()).collect();

    assert!(labels.contains(&"library".to_string()));
    assert!(labels.contains(&"scenery".to_string()));
    assert!(labels.contains(&"track".to_string()));
    assert!(!labels.contains(&"not-top-level".to_string()));

    // Test 2: Typing "kind " (no value yet)
    let soup_content2 = "kind ";
    let pairs2 = parse_soup(soup_content2).unwrap();
    let soup2 = process_soup_ast(pairs2, soup_content2);
    let mut params2 = params.clone();
    params2.text_document_position.position.character = 5;

    let completions2 = soup_completions(&soup2, params2, &validators);
    let labels2: Vec<String> = completions2.iter().map(|c| c.label.clone()).collect();

    assert!(labels2.contains(&"library".to_string()));
    assert!(labels2.contains(&"scenery".to_string()));
    assert!(labels2.contains(&"track".to_string()));

    // Test 3: kind inside a container
    let soup_content3 = "my_container {\n    kind \"\"\n}";
    let container_content3 = r#"
my_container {
    kind {
        type string
    }
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_content3).unwrap();
    let validators3 = trainz_soup_validators::load_validators(&temp_dir);

    let pairs3 = parse_soup(soup_content3).unwrap();
    let soup3 = process_soup_ast(pairs3, soup_content3);
    let mut params3 = params.clone();
    params3.text_document_position.position.line = 1;
    params3.text_document_position.position.character = 10; // Inside kind ""

    let completions3 = soup_completions(&soup3, params3, &validators3);
    let labels3: Vec<String> = completions3.iter().map(|c| c.label.clone()).collect();

    assert!(
        labels3.contains(&"library".to_string()),
        "Nested kind should suggest 'library', got {:?}",
        labels3
    );
    assert!(labels3.contains(&"scenery".to_string()));

    std::fs::remove_dir_all(&temp_dir).unwrap();
}
