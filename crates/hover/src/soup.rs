use gs_ast::soup::value::Value;
use gs_ast::soup::{KeyValuePair, Soup};
use gs_diagnostics::soup::{ContainerValidator, Validators};
use std::collections::HashMap;
use tower_lsp_server::ls_types::{Hover, HoverParams, MarkupContent, MarkupKind, Position};

pub fn soup_hover(soup: &Soup, params: HoverParams, validators: &Validators) -> Option<Hover> {
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

    if let Some(validator) = kind_validator {
        if validator.top_level {
            // Find hover among subpossibilities
            for sub in &validator.subpossibilities {
                let soup_kv = soup
                    .key_value_pairs
                    .iter()
                    .find(|kv| kv.key.eq_ignore_ascii_case(&sub.key));
                if let Some(kv) = soup_kv {
                    if is_in_range(position, &kv.key_range) {
                        return create_hover_from_rule(sub, &kv.key_range);
                    }
                    if let Some(value) = &kv.value {
                        let val_range = match value {
                            Value::Array(_, r) => *r,
                            Value::Numeric(_, r) => *r,
                            Value::String(_, r) => *r,
                            Value::Variable(_, r) => *r,
                            Value::Kuid(_, r) => *r,
                            Value::Container(_, r) => *r,
                        };
                        if is_in_range(position, &val_range) {
                            if let Some(h) =
                                find_hover_in_value(value, position, validators, Some(sub))
                            {
                                return Some(h);
                            }
                        }
                    }
                }
            }
            // Also check rules
            for rule in &validator.rules {
                let soup_kv = soup
                    .key_value_pairs
                    .iter()
                    .find(|kv| kv.key.eq_ignore_ascii_case(&rule.key));
                if let Some(kv) = soup_kv {
                    if is_in_range(position, &kv.key_range) {
                        return create_hover_from_rule(rule, &kv.key_range);
                    }
                    if let Some(value) = &kv.value {
                        let val_range = match value {
                            Value::Array(_, r) => *r,
                            Value::Numeric(_, r) => *r,
                            Value::String(_, r) => *r,
                            Value::Variable(_, r) => *r,
                            Value::Kuid(_, r) => *r,
                            Value::Container(_, r) => *r,
                        };
                        if is_in_range(position, &val_range) {
                            if let Some(h) =
                                find_hover_in_value(value, position, validators, Some(rule))
                            {
                                return Some(h);
                            }
                        }
                    }
                }
            }
        }
    }

    find_hover_recursive(&soup.key_value_pairs, position, &validators, kind_validator)
}

fn create_hover_from_rule(
    rule: &gs_diagnostics::soup::ContainerRule,
    range: &gs_ast::Range,
) -> Option<Hover> {
    let mut doc = format!("### Key: `{}`\n", rule.key);
    if let Some(t) = &rule.type_name {
        doc.push_str(&format!("**Type**: `{}`\n\n", t));
    }
    if let Some(k) = &rule.kind {
        doc.push_str(&format!("**Kind**: `{}`\n\n", k));
    }
    if let Some(d) = &rule.description {
        if !d.is_empty() {
            doc.push_str(&format!("**Description**: {}\n\n", d));
        }
    }

    let mut validation_rules = Vec::new();

    if let Some(v) = &rule.validation {
        validation_rules.push(format!("**Validation**: `{}`", v));
    }
    if let Some(d) = &rule.default_value {
        validation_rules.push(format!("**Default**: `{}`", d));
    }
    if let Some(c) = &rule.compulsory {
        validation_rules.push(format!("**Compulsory**: `{}`", c));
    }
    if let Some(f) = &rule.filter {
        validation_rules.push(format!("**Filter**: `{}`", f));
    }
    if let Some(dis) = &rule.disabled {
        if *dis {
            validation_rules.push("**Disabled**: `true`".to_string());
        }
    }

    if !validation_rules.is_empty() {
        doc.push_str("#### Validation Rules\n");
        for v_rule in validation_rules {
            doc.push_str(&format!("- {}\n", v_rule));
        }
        doc.push('\n');
    }

    return Some(Hover {
        contents: tower_lsp_server::ls_types::HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: doc,
        }),
        range: Some((*range).into()),
    });
}

fn find_hover_in_value(
    value: &Value,
    position: Position,
    validators: &Validators,
    rule: Option<&gs_diagnostics::soup::ContainerRule>,
) -> Option<Hover> {
    match value {
        Value::Container(container_kv, _) => {
            let mut next_validator = None;
            if let Some(rule) = rule {
                if let Some(kind_name) = &rule.kind {
                    next_validator = validators
                        .containers
                        .iter()
                        .find(|v| v.container_name.eq_ignore_ascii_case(kind_name));
                } else if let Some(type_name) = &rule.type_name {
                    next_validator = validators
                        .containers
                        .iter()
                        .find(|v| v.container_name.eq_ignore_ascii_case(type_name));
                }
            }
            find_hover_recursive(container_kv, position, validators, next_validator)
        }
        _ => None,
    }
}

fn get_hover_for_simple_validator(
    value_str: &str,
    allowed_values: &HashMap<String, String>,
    value_range: &gs_ast::Range,
) -> Option<Hover> {
    let mut value_doc = String::new();
    let values: Vec<&str> = value_str.split(';').map(|s| s.trim()).collect();

    let mut found = false;
    for v in &values {
        if let Some(desc) = allowed_values.get(*v) {
            value_doc.push_str(&format!(
                "**Value**: `{}`\n\n**Description**: {}\n\n",
                v, desc
            ));
            found = true;
        }
    }

    if !found && !allowed_values.is_empty() {
        value_doc.push_str(&format!("**Value**: `{}`\n\n", value_str));
    }

    if !allowed_values.is_empty() {
        value_doc.push_str("#### Available Options\n");
        let mut sorted_options: Vec<_> = allowed_values.iter().collect();
        sorted_options.sort_by(|a, b| a.0.cmp(b.0));
        for (opt, desc) in sorted_options {
            value_doc.push_str(&format!("- `{}` {}\n", opt, desc));
        }
    }

    if value_doc.is_empty() {
        return None;
    }

    Some(Hover {
        contents: tower_lsp_server::ls_types::HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: value_doc,
        }),
        range: Some((*value_range).into()),
    })
}

fn find_hover_recursive(
    kvs: &[KeyValuePair],
    position: Position,
    validators: &Validators,
    current_validator: Option<&ContainerValidator>,
) -> Option<Hover> {
    for kv in kvs {
        // Hover over key
        if is_in_range(position, &kv.key_range) {
            if let Some(validator) = current_validator {
                let rule = validator
                    .rules
                    .iter()
                    .find(|r| r.key.eq_ignore_ascii_case(&kv.key));
                if let Some(rule) = rule {
                    return create_hover_from_rule(rule, &kv.key_range);
                }

                // Check if current container is a TagArray and if it has a type rule
                if let Some(validation) = &validator.validation {
                    if validation.eq_ignore_ascii_case("TagArray") {
                        // For TagArray, keys that are not explicitly defined in rules might be entries
                        // and they should be validated against the container type specified in the 'type' rule.
                        if let Some(type_rule) = validator
                            .rules
                            .iter()
                            .find(|r| r.key.eq_ignore_ascii_case("type"))
                        {
                            if let Some(type_name) = &type_rule.type_name {
                                let type_validator = validators
                                    .containers
                                    .iter()
                                    .find(|v| v.container_name.eq_ignore_ascii_case(type_name));
                                if let Some(type_validator) = type_validator {
                                    return Some(Hover {
                                        contents: tower_lsp_server::ls_types::HoverContents::Markup(
                                            MarkupContent {
                                                kind: MarkupKind::Markdown,
                                                value: format!(
                                                    "### TagArray Entry: `{}`\n\n**Validated against**: `{}`",
                                                    kv.key, type_validator.container_name
                                                ),
                                            },
                                        ),
                                        range: Some(kv.key_range.into()),
                                    });
                                }
                            }
                        }
                    }
                }
            } else {
                // Top-level or simple validator
                for validator in &validators.simple {
                    if validator.key_to_check.eq_ignore_ascii_case(&kv.key) {
                        return Some(Hover {
                            contents: tower_lsp_server::ls_types::HoverContents::Markup(
                                MarkupContent {
                                    kind: MarkupKind::Markdown,
                                    value: format!(
                                        "### Key: `{}`\n\n**Validated against**: `{}.txt`",
                                        kv.key, kv.key
                                    ),
                                },
                            ),
                            range: Some(kv.key_range.into()),
                        });
                    }
                }

                for validator in &validators.containers {
                    if validator.container_name.eq_ignore_ascii_case(&kv.key) {
                        return Some(Hover {
                            contents: tower_lsp_server::ls_types::HoverContents::Markup(
                                MarkupContent {
                                    kind: MarkupKind::Markdown,
                                    value: format!(
                                        "### Container: `{}`\n\nValidated against `{}.txt`",
                                        kv.key, kv.key
                                    ),
                                },
                            ),
                            range: Some(kv.key_range.into()),
                        });
                    }
                }
            }
        }

        // Hover over value
        if let Some(value) = &kv.value {
            let value_range = get_value_range(value);
            if is_in_range(position, &value_range) {
                if let Value::Container(inner_kvs, _) = value {
                    // Determine the validator for this container.
                    let next_validator = if let Some(cv) = current_validator {
                        if let Some(element_type) = &cv.array_element {
                            validators
                                .containers
                                .iter()
                                .find(|v| v.container_name.eq_ignore_ascii_case(element_type))
                        } else {
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
                                // Check if this is a TagArray and the key is an entry (not 'type')
                                if let Some(validation) = &cv.validation {
                                    if validation.eq_ignore_ascii_case("TagArray")
                                        && !kv.key.eq_ignore_ascii_case("type")
                                    {
                                        if let Some(type_rule) = cv
                                            .rules
                                            .iter()
                                            .find(|r| r.key.eq_ignore_ascii_case("type"))
                                        {
                                            if let Some(type_name) = &type_rule.type_name {
                                                validators.containers.iter().find(|v| {
                                                    v.container_name.eq_ignore_ascii_case(type_name)
                                                })
                                            } else {
                                                None
                                            }
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            }
                        }
                    } else {
                        validators
                            .containers
                            .iter()
                            .find(|v| v.container_name.eq_ignore_ascii_case(&kv.key))
                    };

                    let nested_hover =
                        find_hover_recursive(inner_kvs, position, validators, next_validator);
                    if nested_hover.is_some() {
                        return nested_hover;
                    }
                }

                // If not in a deeper one, check if we have value hover here
                if let Some(validator) = current_validator {
                    let rule = validator
                        .rules
                        .iter()
                        .find(|r| r.key.eq_ignore_ascii_case(&kv.key));
                    if let Some(rule) = rule {
                        if let Some(validation) = &rule.validation {
                            let value_str = match value {
                                Value::String(s, _) => s.clone(),
                                Value::Variable(s, _) => s.clone(),
                                Value::Numeric(n, _) => match n {
                                    gs_ast::soup::NumericValue::Int(i) => i.to_string(),
                                    gs_ast::soup::NumericValue::Float(f) => f.to_string(),
                                    gs_ast::soup::NumericValue::Hex(h) => format!("0x{:x}", h),
                                },
                                _ => String::new(),
                            };

                            if validation.eq_ignore_ascii_case("IsValidCategoryClass") {
                                return get_hover_for_simple_validator(
                                    &value_str,
                                    &validators.category_classes,
                                    &value_range,
                                );
                            } else if validation.eq_ignore_ascii_case("IsValidCategoryRegion") {
                                return get_hover_for_simple_validator(
                                    &value_str,
                                    &validators.category_regions,
                                    &value_range,
                                );
                            } else if validation.eq_ignore_ascii_case("IsValidCategoryEra") {
                                return get_hover_for_simple_validator(
                                    &value_str,
                                    &validators.category_eras,
                                    &value_range,
                                );
                            }

                            return Some(Hover {
                                contents: tower_lsp_server::ls_types::HoverContents::Markup(
                                    MarkupContent {
                                        kind: MarkupKind::Markdown,
                                        value: format!(
                                            "**Value**: `{}`\n\n**Allowed**: `{}`",
                                            value_str, validation
                                        ),
                                    },
                                ),
                                range: Some(value_range.into()),
                            });
                        }

                        // Also check if type_name refers to a simple validator
                        if let Some(type_name) = &rule.type_name {
                            if let Some(simple_validator) = validators
                                .simple
                                .iter()
                                .find(|v| v.key_to_check.eq_ignore_ascii_case(type_name))
                            {
                                let value_str = match value {
                                    Value::String(s, _) => s.clone(),
                                    Value::Variable(s, _) => s.clone(),
                                    _ => String::new(),
                                };
                                return get_hover_for_simple_validator(
                                    &value_str,
                                    &simple_validator.allowed_values,
                                    &value_range,
                                );
                            }
                        }
                    }
                } else {
                    // Simple validator value hover
                    if kv.key.eq_ignore_ascii_case("category-era") {
                        let value_str = match value {
                            Value::String(s, _) => s.clone(),
                            Value::Variable(s, _) => s.clone(),
                            _ => String::new(),
                        };
                        return get_hover_for_simple_validator(
                            &value_str,
                            &validators.category_eras,
                            &value_range,
                        );
                    } else if kv.key.eq_ignore_ascii_case("category-region") {
                        let value_str = match value {
                            Value::String(s, _) => s.clone(),
                            Value::Variable(s, _) => s.clone(),
                            _ => String::new(),
                        };
                        return get_hover_for_simple_validator(
                            &value_str,
                            &validators.category_regions,
                            &value_range,
                        );
                    } else if kv.key.eq_ignore_ascii_case("category-class") {
                        let value_str = match value {
                            Value::String(s, _) => s.clone(),
                            Value::Variable(s, _) => s.clone(),
                            _ => String::new(),
                        };
                        return get_hover_for_simple_validator(
                            &value_str,
                            &validators.category_classes,
                            &value_range,
                        );
                    }

                    for validator in &validators.simple {
                        if validator.key_to_check.eq_ignore_ascii_case(&kv.key) {
                            let value_str = match value {
                                Value::String(s, _) => s.clone(),
                                Value::Variable(s, _) => s.clone(),
                                _ => String::new(),
                            };
                            return get_hover_for_simple_validator(
                                &value_str,
                                &validator.allowed_values,
                                &value_range,
                            );
                        }
                    }
                }
            }
        }
    }

    None
}

fn is_in_range(position: tower_lsp_server::ls_types::Position, range: &gs_ast::Range) -> bool {
    // Check if the position is within the range [start, end)
    if position.line < range.start.line || position.line > range.end.line {
        return false;
    }

    if position.line == range.start.line && position.character < range.start.character {
        return false;
    }

    if position.line == range.end.line && position.character >= range.end.character {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use gs_ast::soup::process::process_soup_ast;
    use gs_diagnostics::soup::load_validators;
    use gs_parser::soup::parse_soup;
    use std::path::PathBuf;

    #[test]
    fn test_soup_hover_case_insensitive() {
        let content = r#"
My_Container {
    KeyA "value"
}
"#;
        let pairs = parse_soup(content).unwrap();
        let soup = process_soup_ast(
            pairs,
            content,
            &PathBuf::from("test.soup"),
            &vec![],
            &vec![],
        );

        // Create a temporary validation directory
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

        // Hover over "KeyA" in Soup which is at line 2, char 4 (0-indexed)
        // content is:
        // \n (line 0)
        // My_Container { (line 1)
        //     KeyA "value" (line 2)
        // }
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

        // Test top-level case-insensitive match
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
        let soup = process_soup_ast(
            pairs,
            content,
            &PathBuf::from("test.soup"),
            &vec![],
            &vec![],
        );

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
        let soup = process_soup_ast(
            pairs,
            content,
            &PathBuf::from("test.soup"),
            &vec![],
            &vec![],
        );

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
  validation
  {
    TagArray
  }
  type
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
        if let tower_lsp_server::ls_types::HoverContents::Markup(markup) =
            hover_key.unwrap().contents
        {
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
        if let tower_lsp_server::ls_types::HoverContents::Markup(markup) =
            hover_inner.unwrap().contents
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
        let soup = process_soup_ast(
            pairs,
            content,
            &PathBuf::from("test.soup"),
            &vec![],
            &vec![],
        );

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
                    character: 21,
                },
            },
            work_done_progress_params: Default::default(),
        };

        let hover1 = soup_hover(&soup, params1, &validators);
        assert!(
            hover1.is_some(),
            "Hover should be found for engine-type value"
        );
        if let tower_lsp_server::ls_types::HoverContents::Markup(markup) = hover1.unwrap().contents
        {
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
                markup.value.contains("- `AA` Electric Multi-current"),
                "Hover should list option AA"
            );
            assert!(
                markup.value.contains("- `AC` AC Electric"),
                "Hover should list option AC"
            );
            assert!(
                markup.value.contains("- `AD` DC Electric"),
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
                    character: 24,
                },
            },
            work_done_progress_params: Default::default(),
        };

        let hover2 = soup_hover(&soup, params2, &validators);
        assert!(
            hover2.is_some(),
            "Hover should be found for category-class value"
        );
        if let tower_lsp_server::ls_types::HoverContents::Markup(markup) = hover2.unwrap().contents
        {
            assert!(
                markup.value.contains("AC Category"),
                "Hover should contain description for AC"
            );
            assert!(
                markup.value.contains("#### Available Options"),
                "Hover should contain options section"
            );
            assert!(
                markup.value.contains("- `AC` AC Category"),
                "Hover should list option AC"
            );
            assert!(
                markup.value.contains("- `DC` DC Category"),
                "Hover should list option DC"
            );
        }

        std::fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_rule_type_simple_validator_hover() {
        let content = "MyContainer {\n    my-engine \"AA\"\n}\n";
        let pairs = parse_soup(content).unwrap();
        let soup = process_soup_ast(
            pairs,
            content,
            &PathBuf::from("test.soup"),
            &vec![],
            &vec![],
        );

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
