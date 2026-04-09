use tower_lsp_server::ls_types::{
    Documentation, ParameterInformation, SignatureHelp, SignatureHelpParams, SignatureInformation,
};
use trainz_ast::soup::value::Value;
use trainz_ast::soup::{KeyValuePair, Soup};
use trainz_soup_validators::{ArrayElementType, ContainerValidator, Validators};

pub fn soup_signature_help(
    soup: &Soup,
    params: SignatureHelpParams,
    validators: &Validators,
) -> Option<SignatureHelp> {
    let position = params.text_document_position_params.position;

    // Find kind-based validator for the whole soup if 'kind' key exists
    let kind_validator = soup
        .key_value_pairs
        .iter()
        .find(|kv| kv.key.eq_ignore_ascii_case("kind"))
        .and_then(|kv| {
            if let Some(Value::String(kind_name, _)) = &kv.value {
                validators
                    .containers
                    .iter()
                    .find(|v| v.container_name.eq_ignore_ascii_case(kind_name))
            } else if let Some(Value::Variable(kind_name, _)) = &kv.value {
                validators
                    .containers
                    .iter()
                    .find(|v| v.container_name.eq_ignore_ascii_case(kind_name))
            } else {
                None
            }
        });

    find_signature_recursive(&soup.key_value_pairs, position, validators, kind_validator)
}

fn find_signature_recursive(
    kvs: &[KeyValuePair],
    position: tower_lsp_server::ls_types::Position,
    validators: &Validators,
    current_validator: Option<&ContainerValidator>,
) -> Option<SignatureHelp> {
    for kv in kvs {
        if let Some(value) = &kv.value {
            let value_range = match value {
                Value::Array(_, r) => *r,
                Value::Numeric(_, r) => *r,
                Value::String(_, r) => *r,
                Value::Variable(_, r) => *r,
                Value::Kuid(_, r) => *r,
                Value::Container(_, r) => *r,
            };

            if is_in_range(position, &value_range) {
                if let Value::Container(inner_kvs, _) = value {
                    // Determine the validator for this container.
                    let next_validator = if let Some(cv) = current_validator {
                        if let Some(array_element_type) = &cv.array_element {
                            let type_name = match array_element_type {
                                ArrayElementType::Array(s) => Some(s),
                                ArrayElementType::Tuple(types) => {
                                    kv.key.parse::<usize>().ok().and_then(|idx| types.get(idx))
                                }
                            };
                            type_name.and_then(|tn| {
                                validators
                                    .containers
                                    .iter()
                                    .find(|v| v.container_name.eq_ignore_ascii_case(tn))
                            })
                        } else {
                            let rule = cv
                                .rules
                                .iter()
                                .find(|r| r.key.eq_ignore_ascii_case(&kv.key))
                                .or_else(|| {
                                    cv.sub_possibilities
                                        .iter()
                                        .find(|r| r.key.eq_ignore_ascii_case(&kv.key))
                                });
                            if let Some(rule) = rule {
                                let target_name = rule.kind.as_ref().or(rule.type_name.as_ref());
                                if let Some(name) = target_name {
                                    validators
                                        .containers
                                        .iter()
                                        .find(|v| v.container_name.eq_ignore_ascii_case(name))
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        }
                    } else {
                        // Top level, maybe kv.key is a container name
                        validators
                            .containers
                            .iter()
                            .find(|v| v.container_name.eq_ignore_ascii_case(&kv.key))
                    };

                    if let Some(sig) =
                        find_signature_recursive(inner_kvs, position, validators, next_validator)
                    {
                        return Some(sig);
                    }

                    // If we are inside this container but no inner KV matches, show signature for this container
                    if let Some(validator) = next_validator {
                        return Some(create_signature_help(validator, inner_kvs, position));
                    }
                }
            }
        }
    }

    // If we are at the top level and no KV matches, show signature for top level if we have a validator
    if let Some(validator) = current_validator {
        return Some(create_signature_help(validator, kvs, position));
    }

    None
}

fn create_signature_help(
    validator: &ContainerValidator,
    kvs: &[KeyValuePair],
    position: tower_lsp_server::ls_types::Position,
) -> SignatureHelp {
    let mut parameters = Vec::new();
    let mut label = format!("{}(", validator.container_name);
    let mut active_parameter = 0;

    // Combine rules and subpossibilities
    let mut all_rules = validator.rules.clone();
    for sub in &validator.sub_possibilities {
        if !all_rules.iter().any(|r| r.key == sub.key) {
            all_rules.push(sub.clone());
        }
    }

    for (i, rule) in all_rules.iter().enumerate() {
        if i > 0 {
            label.push_str(", ");
        }
        let start = label.len();
        label.push_str(&rule.key);
        let end = label.len();

        let mut doc = String::new();
        if let Some(t) = &rule.type_name {
            doc.push_str(&format!("Type: {}\n", t));
        }
        if let Some(desc) = &rule.description {
            doc.push_str(desc);
        }

        parameters.push(ParameterInformation {
            label: tower_lsp_server::ls_types::ParameterLabel::LabelOffsets([
                start as u32,
                end as u32,
            ]),
            documentation: if doc.is_empty() {
                None
            } else {
                Some(Documentation::String(doc))
            },
        });

        // Determine active parameter based on cursor position relative to existing KVs
        // This is a bit simplified: we check if the cursor is at or after this KV
        let kv = kvs.iter().find(|kv| kv.key.eq_ignore_ascii_case(&rule.key));
        if let Some(kv) = kv {
            let kv_range = get_kv_range(kv);
            if position.line > kv_range.end.line
                || (position.line == kv_range.end.line
                    && position.character >= kv_range.end.character)
            {
                active_parameter = (i + 1) as u32;
            }
        }
    }
    label.push(')');

    // Ensure active_parameter is within bounds
    if active_parameter >= parameters.len() as u32 {
        active_parameter = if parameters.is_empty() {
            0
        } else {
            (parameters.len() - 1) as u32
        };
    }

    SignatureHelp {
        signatures: vec![SignatureInformation {
            label,
            documentation: None,
            parameters: Some(parameters),
            active_parameter: Some(active_parameter),
        }],
        active_signature: Some(0),
        active_parameter: Some(active_parameter),
    }
}

fn is_in_range(position: tower_lsp_server::ls_types::Position, range: &trainz_ast::Range) -> bool {
    let pos_line = position.line;
    let pos_char = position.character;

    if pos_line < range.start.line || pos_line > range.end.line {
        return false;
    }

    if pos_line == range.start.line && pos_char < range.start.character {
        return false;
    }

    if pos_line == range.end.line && pos_char > range.end.character {
        return false;
    }

    true
}

fn get_kv_range(kv: &KeyValuePair) -> trainz_ast::Range {
    let mut range = kv.key_range;
    if let Some(value) = &kv.value {
        let value_range = match value {
            Value::Array(_, r) => *r,
            Value::Numeric(_, r) => *r,
            Value::String(_, r) => *r,
            Value::Variable(_, r) => *r,
            Value::Kuid(_, r) => *r,
            Value::Container(_, r) => *r,
        };
        range.end = value_range.end;
    }
    range
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower_lsp_server::ls_types::{
        Position, SignatureHelpParams, TextDocumentIdentifier, TextDocumentPositionParams, Uri,
    };
    use trainz_ast::soup::value::Value;
    use trainz_ast::soup::KeyValuePair;
    use trainz_ast::Range;
    use trainz_soup_validators::{ContainerRule, ContainerValidator, Validators};

    #[test]
    fn test_soup_signature_help_basic() {
        let soup = Soup {
            key_value_pairs: vec![
                KeyValuePair {
                    key: "kind".to_string(),
                    key_range: Range {
                        start: Position::new(0, 0),
                        end: Position::new(0, 4),
                    },
                    value: Some(Value::String(
                        "MyContainer".to_string(),
                        Range {
                            start: Position::new(0, 5),
                            end: Position::new(0, 16),
                        },
                    )),
                    range: Range {
                        start: Position::new(0, 0),
                        end: Position::new(0, 16),
                    },
                },
                KeyValuePair {
                    key: "name".to_string(),
                    key_range: Range {
                        start: Position::new(1, 0),
                        end: Position::new(1, 4),
                    },
                    value: Some(Value::String(
                        "test".to_string(),
                        Range {
                            start: Position::new(1, 5),
                            end: Position::new(1, 11),
                        },
                    )),
                    range: Range {
                        start: Position::new(1, 0),
                        end: Position::new(1, 11),
                    },
                },
            ],
            range: Range {
                start: Position::new(0, 0),
                end: Position::new(2, 0),
            },
            src: String::new(),
        };

        let mut validators = Validators::default();
        validators.containers.push(ContainerValidator {
            container_name: "MyContainer".to_string(),
            top_level: true,
            rules: vec![
                ContainerRule {
                    key: "kind".to_string(),
                    description: Some("The kind of container".to_string()),
                    ..Default::default()
                },
                ContainerRule {
                    key: "name".to_string(),
                    description: Some("The name of the object".to_string()),
                    ..Default::default()
                },
                ContainerRule {
                    key: "description".to_string(),
                    description: Some("Detailed description".to_string()),
                    ..Default::default()
                },
            ],
            ..Default::default()
        });

        // Cursor at the end of "name test"
        let params = SignatureHelpParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: Uri::from_file_path("/test.soup").unwrap(),
                },
                position: Position::new(1, 12),
            },
            work_done_progress_params: Default::default(),
            context: None,
        };

        let help = soup_signature_help(&soup, params, &validators).unwrap();
        assert_eq!(help.signatures.len(), 1);
        let sig = &help.signatures[0];
        assert_eq!(sig.label, "MyContainer(kind, name, description)");
        // Since we are at line 1, char 12, which is after "name" KV (line 1, char 0-11)
        // active_parameter should be 2 (the index of "description")
        assert_eq!(sig.active_parameter, Some(2));
    }

    #[test]
    fn test_soup_signature_help_nested() {
        let soup = Soup {
            key_value_pairs: vec![
                KeyValuePair {
                    key: "kind".to_string(),
                    key_range: Range {
                        start: Position::new(0, 0),
                        end: Position::new(0, 4),
                    },
                    value: Some(Value::String(
                        "Parent".to_string(),
                        Range {
                            start: Position::new(0, 5),
                            end: Position::new(0, 11),
                        },
                    )),
                    range: Range {
                        start: Position::new(0, 0),
                        end: Position::new(0, 11),
                    },
                },
                KeyValuePair {
                    key: "child".to_string(),
                    key_range: Range {
                        start: Position::new(1, 0),
                        end: Position::new(1, 5),
                    },
                    value: Some(Value::Container(
                        vec![KeyValuePair {
                            key: "inner".to_string(),
                            key_range: Range {
                                start: Position::new(2, 2),
                                end: Position::new(2, 7),
                            },
                            value: Some(Value::Numeric(
                                trainz_ast::soup::value::NumericValue::Int(10),
                                Range {
                                    start: Position::new(2, 8),
                                    end: Position::new(2, 10),
                                },
                            )),
                            range: Range {
                                start: Position::new(2, 2),
                                end: Position::new(2, 10),
                            },
                        }],
                        Range {
                            start: Position::new(1, 6),
                            end: Position::new(3, 1),
                        },
                    )),
                    range: Range {
                        start: Position::new(1, 0),
                        end: Position::new(3, 1),
                    },
                },
            ],
            range: Range {
                start: Position::new(0, 0),
                end: Position::new(4, 0),
            },
            src: String::new(),
        };

        let mut validators = Validators::default();
        validators.containers.push(ContainerValidator {
            container_name: "Parent".to_string(),
            top_level: true,
            rules: vec![
                ContainerRule {
                    key: "kind".to_string(),
                    ..Default::default()
                },
                ContainerRule {
                    key: "child".to_string(),
                    type_name: Some("Child".to_string()),
                    ..Default::default()
                },
            ],
            sub_possibilities: vec![],
            ..Default::default()
        });
        validators.containers.push(ContainerValidator {
            container_name: "Child".to_string(),
            rules: vec![
                ContainerRule {
                    key: "inner".to_string(),
                    description: Some("Inner value".to_string()),
                    ..Default::default()
                },
                ContainerRule {
                    key: "other".to_string(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        });

        // Cursor inside "child" container, after "inner 10"
        let params = SignatureHelpParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: Uri::from_file_path("/test.soup").unwrap(),
                },
                position: Position::new(2, 11),
            },
            work_done_progress_params: Default::default(),
            context: None,
        };

        let help = soup_signature_help(&soup, params, &validators).unwrap();
        assert_eq!(help.signatures[0].label, "Child(inner, other)");
        assert_eq!(help.signatures[0].active_parameter, Some(1));
    }
}
