use gs_ast::soup::value::Value;
use gs_ast::soup::{KeyValuePair, Soup};
use gs_diagnostics::soup::{ContainerValidator, Validators, load_validators};
use std::path::PathBuf;
use tower_lsp_server::ls_types::{CompletionItem, CompletionItemKind, CompletionParams, Position};

pub fn soup_completions(
    soup: &Soup,
    params: CompletionParams,
    validation_path: Option<PathBuf>,
) -> Vec<CompletionItem> {
    let position = params.text_document_position.position;

    if let Some(validation_path) = &validation_path {
        let validators = load_validators(validation_path);
        return find_completions_recursive(&soup.key_value_pairs, position, &validators, None);
    }

    vec![]
}

fn find_completions_recursive(
    kvs: &[KeyValuePair],
    position: Position,
    validators: &Validators,
    current_validator: Option<&ContainerValidator>,
) -> Vec<CompletionItem> {
    let mut completions = vec![];

    for kv in kvs {
        if let Some(value) = &kv.value {
            let range = match value {
                Value::Container(_, r) => r,
                _ => continue,
            };

            if is_in_range(position, range) {
                // We are inside this container.
                // Determine the validator for this container.
                let next_validator = if let Some(cv) = current_validator {
                    // We are in a nested container.
                    // If the parent is an array-element container, this might be an element.
                    if let Some(element_type) = &cv.array_element {
                        validators
                            .containers
                            .iter()
                            .find(|v| v.container_name.eq_ignore_ascii_case(element_type))
                    } else {
                        // Otherwise, it might be a explicitly named sub-container.
                        let rule = cv
                            .rules
                            .iter()
                            .find(|r| r.key.eq_ignore_ascii_case(&kv.key));
                        if let Some(rule) = rule {
                            if let Some(type_name) = &rule.type_name {
                                validators
                                    .containers
                                    .iter()
                                    .find(|v| v.container_name.eq_ignore_ascii_case(type_name))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                } else {
                    // Top-level container.
                    validators
                        .containers
                        .iter()
                        .find(|v| v.container_name.eq_ignore_ascii_case(&kv.key))
                };

                if let Value::Container(inner_kvs, _) = value {
                    // Check if we are inside a deeper nested container.
                    let nested_completions =
                        find_completions_recursive(inner_kvs, position, validators, next_validator);
                    if !nested_completions.is_empty() {
                        return nested_completions;
                    }

                    // If not in a deeper one, we are directly in this container.
                    if let Some(validator) = next_validator {
                        let mut in_value = false;
                        for inner_kv in inner_kvs {
                            if let Some(inner_value) = &inner_kv.value {
                                let inner_value_range = get_value_range(inner_value);
                                if is_in_range(position, &inner_value_range) {
                                    in_value = true;
                                    // Completion for value based on validation rule
                                    let rule = validator
                                        .rules
                                        .iter()
                                        .find(|r| r.key.eq_ignore_ascii_case(&inner_kv.key));
                                    if let Some(rule) = rule {
                                        if let Some(validation) = &rule.validation {
                                            if validation
                                                .eq_ignore_ascii_case("IsValidCategoryClass")
                                            {
                                                for (key, name) in &validators.category_classes {
                                                    completions.push(CompletionItem {
                                                        label: key.clone(),
                                                        kind: Some(CompletionItemKind::ENUM_MEMBER),
                                                        detail: Some(name.clone()),
                                                        ..Default::default()
                                                    });
                                                }
                                            } else if validation
                                                .eq_ignore_ascii_case("IsValidCategoryRegion")
                                            {
                                                for (key, name) in &validators.category_regions {
                                                    completions.push(CompletionItem {
                                                        label: key.clone(),
                                                        kind: Some(CompletionItemKind::ENUM_MEMBER),
                                                        detail: Some(name.clone()),
                                                        ..Default::default()
                                                    });
                                                }
                                            } else if validation
                                                .eq_ignore_ascii_case("IsValidCategoryEra")
                                            {
                                                for (key, name) in &validators.category_eras {
                                                    completions.push(CompletionItem {
                                                        label: key.clone(),
                                                        kind: Some(CompletionItemKind::ENUM_MEMBER),
                                                        detail: Some(name.clone()),
                                                        ..Default::default()
                                                    });
                                                }
                                            } else {
                                                for allowed_val in
                                                    validation.split(',').map(|s| s.trim())
                                                {
                                                    completions.push(CompletionItem {
                                                        label: allowed_val.to_string(),
                                                        kind: Some(CompletionItemKind::ENUM_MEMBER),
                                                        ..Default::default()
                                                    });
                                                }
                                            }
                                        }
                                    }
                                    break;
                                }
                            }
                        }

                        if !in_value {
                            // Completion for keys in container
                            for rule in &validator.rules {
                                completions.push(CompletionItem {
                                    label: rule.key.clone(),
                                    kind: Some(CompletionItemKind::FIELD),
                                    detail: rule.type_name.clone(),
                                    documentation: rule.validation.as_ref().map(|v| {
                                        tower_lsp_server::ls_types::Documentation::String(format!(
                                            "Validation: {}",
                                            v
                                        ))
                                    }),
                                    ..Default::default()
                                });
                            }

                            // If it's an array-element container, suggest that it takes any key
                            if let Some(element_type) = &validator.array_element {
                                completions.push(CompletionItem {
                                    label: "element_name".to_string(),
                                    kind: Some(CompletionItemKind::SNIPPET),
                                    detail: Some(format!("Element type: {}", element_type)),
                                    documentation: Some(
                                        tower_lsp_server::ls_types::Documentation::String(format!(
                                            "This container accepts elements of type '{}' with any unique name.",
                                            element_type
                                        )),
                                    ),
                                    insert_text: Some("${1:name} {\n\t$0\n}".to_string()),
                                    insert_text_format: Some(
                                        tower_lsp_server::ls_types::InsertTextFormat::SNIPPET,
                                    ),
                                    ..Default::default()
                                });
                            }
                        }
                    }
                }
                return completions;
            }
        }
    }

    // Simple validators check
    for kv in kvs {
        if let Some(value) = &kv.value {
            let value_range = get_value_range(value);

            if is_in_range(position, &value_range) {
                // We are in a value. Check if we have a validator for the key.
                for validator in &validators.simple {
                    if validator.key_to_check.eq_ignore_ascii_case(&kv.key) {
                        for (allowed_value, description) in &validator.allowed_values {
                            completions.push(CompletionItem {
                                label: allowed_value.clone(),
                                kind: Some(CompletionItemKind::ENUM_MEMBER),
                                detail: if description.is_empty() {
                                    None
                                } else {
                                    Some(description.clone())
                                },
                                ..Default::default()
                            });
                        }
                    }
                }
            }
        }
    }

    completions
}

fn is_in_range(position: tower_lsp_server::ls_types::Position, range: &gs_ast::Range) -> bool {
    position.line >= range.start.line
        && position.line <= range.end.line
        && (position.line > range.start.line || position.character >= range.start.character)
        && (position.line < range.end.line || position.character < range.end.character)
}

fn get_value_range(value: &Value) -> gs_ast::Range {
    match value {
        Value::Array(_, r) => *r,
        Value::Numeric(_, r) => *r,
        Value::String(_, r) => *r,
        Value::Variable(_, r) => *r,
        Value::Kuid(_, r) => *r,
        Value::Container(_, r) => *r,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gs_ast::soup::process::process_soup_ast;
    use gs_parser::soup::parse_soup;
    use tower_lsp_server::ls_types::{TextDocumentIdentifier, TextDocumentPositionParams};

    #[test]
    fn test_category_class_completions() {
        let temp_dir = std::env::temp_dir().join("gs-lsp-test-cat-comp");
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
        let soup = process_soup_ast(
            pairs,
            soup_content,
            &std::path::PathBuf::from("test.soup"),
            &vec![],
            &vec![],
        );

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

        let completions = soup_completions(&soup, params, Some(temp_dir.clone()));

        assert!(!completions.is_empty(), "Completions should not be empty");

        let scenery_comp = completions.iter().find(|c| c.label == "Scenery");
        assert!(scenery_comp.is_some(), "Should suggest 'Scenery'");
        assert_eq!(
            scenery_comp.unwrap().detail,
            Some("Scenery objects".to_string())
        );

        let track_comp = completions.iter().find(|c| c.label == "Track");
        assert!(track_comp.is_some(), "Should suggest 'Track'");
        assert_eq!(
            track_comp.unwrap().detail,
            Some("Track objects".to_string())
        );

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
        let soup_region = process_soup_ast(
            pairs_region,
            soup_content_region,
            &std::path::PathBuf::from("test_region.soup"),
            &vec![],
            &vec![],
        );

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

        let completions_region =
            soup_completions(&soup_region, params_region, Some(temp_dir.clone()));

        assert!(
            !completions_region.is_empty(),
            "Region completions should not be empty"
        );

        let fra_comp = completions_region.iter().find(|c| c.label == "FRA");
        assert!(fra_comp.is_some(), "Should suggest 'FRA'");
        assert_eq!(fra_comp.unwrap().detail, Some("France".to_string()));

        let usa_comp = completions_region.iter().find(|c| c.label == "USA");
        assert!(usa_comp.is_some(), "Should suggest 'USA'");
        assert_eq!(usa_comp.unwrap().detail, Some("United States".to_string()));

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
        let soup_era = process_soup_ast(
            pairs_era,
            soup_content_era,
            &std::path::PathBuf::from("test_era.soup"),
            &vec![],
            &vec![],
        );

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

        let completions_era = soup_completions(&soup_era, params_era, Some(temp_dir.clone()));

        assert!(
            !completions_era.is_empty(),
            "Era completions should not be empty"
        );

        let s2000_comp = completions_era.iter().find(|c| c.label == "2000s");
        assert!(s2000_comp.is_some(), "Should suggest '2000s'");
        assert_eq!(s2000_comp.unwrap().detail, Some("2000s era".to_string()));

        let s2010_comp = completions_era.iter().find(|c| c.label == "2010s");
        assert!(s2010_comp.is_some(), "Should suggest '2010s'");
        assert_eq!(s2010_comp.unwrap().detail, Some("2010s era".to_string()));

        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
}
