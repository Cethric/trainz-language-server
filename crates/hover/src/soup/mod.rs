
use tower_lsp_server::ls_types::{Hover, HoverParams, MarkupContent, MarkupKind, Position};
use trainz_ast::soup::key_value_pair::KeyValuePair;
use trainz_ast::soup::soup::Soup;
use trainz_ast::soup::value::Value;
use trainz_soup_validators::{ArrayElementType, ContainerValidator, Validators};

pub mod rule;
pub mod util;
pub mod value;

#[cfg(test)]
mod tests;

use crate::soup::rule::create_hover_from_rule;
use crate::soup::util::{get_value_range, is_in_range};
use crate::soup::value::{find_hover_in_value, get_hover_for_simple_validator};
use trainz_common::wiki::{get_wiki_container_name, get_wiki_kind_name};

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
        })
        .or_else(|| {
            // If no 'kind' tag, the top-level keys themselves might be container names
            // Check if ANY top-level key matches a top-level container definition
            soup.key_value_pairs.iter().find_map(|kv| {
                validators
                    .containers
                    .iter()
                    .find(|v| v.top_level && v.container_name.eq_ignore_ascii_case(&kv.key))
            })
        });

    find_hover_recursive(&soup.key_value_pairs, position, validators, kind_validator)
}

pub fn find_hover_recursive(
    kvs: &[KeyValuePair],
    position: Position,
    validators: &Validators,
    current_validator: Option<&ContainerValidator>,
) -> Option<Hover> {
    for kv in kvs {
        if is_in_range(position, &kv.key_range) {
            if let Some(validator) = current_validator {
                let rule = validator
                    .rules
                    .iter()
                    .find(|r| r.key.eq_ignore_ascii_case(&kv.key));
                if let Some(rule) = rule {
                    let mut hover = create_hover_from_rule(rule, &kv.key_range);
                    if let Some(h) = &mut hover {
                        if let Some(kind_name) = &rule.kind {
                            let wiki_name = get_wiki_container_name(kind_name);
                            if let tower_lsp_server::ls_types::HoverContents::Markup(markup) =
                                &mut h.contents
                            {
                                if validators
                                    .containers
                                    .iter()
                                    .any(|v| v.container_name.eq_ignore_ascii_case(kind_name))
                                {
                                    markup.value.push_str(&format!(
                                        "**Wiki**: [\"{}\" container](https://online.ts2009.com/mediaWiki/index.php/\"{}\"_container)\n\n",
                                        wiki_name, wiki_name
                                    ));
                                }
                            }
                        }
                    }
                    return hover;
                }

                // Check if current container is a TagArray and if it has a type rule

                if let Some(tag_array) = &validator.tag_array {
                    if let Some(type_name) = &tag_array.type_name {
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
            } else {
                // Top-level or simple validator
                for (key, values) in &validators.simple {
                    if key.eq_ignore_ascii_case(&kv.key) {
                        return Some(Hover {
                            contents: tower_lsp_server::ls_types::HoverContents::Markup(
                                MarkupContent {
                                    kind: MarkupKind::Markdown,
                                    value: format!(
                                        "### Key: `{}`\n### Values:\n{}",
                                        kv.key,
                                        values
                                            .iter()
                                            .map(|(k, v)| if let Some(v) = v {
                                                format!("- `{}`: {}", k, v)
                                            } else {
                                                format!("- `{}`", k)
                                            })
                                            .collect::<Vec<String>>()
                                            .join("\n")
                                    ),
                                },
                            ),
                            range: Some(kv.key_range.into()),
                        });
                    }
                }

                // Check top-level container match
                let container_validator = validators
                    .containers
                    .iter()
                    .find(|v| v.container_name.eq_ignore_ascii_case(&kv.key));
                if let Some(validator) = container_validator {
                    let wiki_name = get_wiki_container_name(&validator.container_name);
                    return Some(Hover {
                        contents: tower_lsp_server::ls_types::HoverContents::Markup(
                            MarkupContent {
                                kind: MarkupKind::Markdown,
                                value: format!(
                                    "### Container: `{}`\n\n**Wiki**: [\"{}\" container](https://online.ts2009.com/mediaWiki/index.php/\"{}\"_container)",
                                    kv.key, wiki_name, wiki_name
                                ),
                            },
                        ),
                        range: Some(kv.key_range.into()),
                    });
                }
            }
        }

        // Hover over value
        if let Some(value) = &kv.value {
            let value_range = get_value_range(value);
            if is_in_range(position, &value_range) {
                if let Value::Container(inner_kvs, _, _) = value {
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
                                if let Some(tag_array) = &cv.tag_array {
                                    if let Some(type_name) = &tag_array.type_name {
                                        validators.containers.iter().find(|v| {
                                            v.container_name.eq_ignore_ascii_case(&type_name)
                                        })
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
                if kv.key.eq_ignore_ascii_case("kind") {
                    let value_str = match value {
                        Value::String(s, _) => s.clone(),
                        Value::Variable(s, _) => s.clone(),
                        _ => String::new(),
                    };

                    if !value_str.is_empty() {
                        let wiki_name = get_wiki_kind_name(&value_str);

                        let title = format!("### Kind: `{}`", value_str);

                        return Some(Hover {
                            contents: tower_lsp_server::ls_types::HoverContents::Markup(
                                MarkupContent {
                                    kind: MarkupKind::Markdown,
                                    value: format!(
                                        "{}\n\n**Wiki**: [KIND {}](https://online.ts2009.com/mediaWiki/index.php/KIND_{})",
                                        title, wiki_name, wiki_name
                                    ),
                                },
                            ),
                            range: Some(value_range.into()),
                        });
                    }
                }
                // Check simple validators for top-level keys
                if let Some(validator) = validators.simple.get(&kv.key) {
                    let value_str = match value {
                        Value::String(s, _) => s.clone(),
                        Value::Variable(s, _) => s.clone(),
                        _ => String::new(),
                    };
                    return get_hover_for_simple_validator(&value_str, validator, &value_range);
                }

                // If no other hover found, check if it's a hover in the value itself
                return find_hover_in_value(value, position, validators, None);
            }
        }
    }

    None
}
