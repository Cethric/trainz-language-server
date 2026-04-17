use rayon::prelude::*;
use tower_lsp_server::ls_types::{Hover, HoverParams, MarkupContent, MarkupKind, Position};
use trainz_acs_text_validators::{ArrayElementType, ContainerValidator, Validators};
use trainz_ast::acs_text::base::AcsText;
use trainz_ast::acs_text::key_value_pair::KeyValuePair;
use trainz_ast::acs_text::value::Value;

pub mod rule;
pub mod util;
pub mod value;

#[cfg(test)]
mod tests;

use crate::acs_text::rule::create_hover_from_rule;
use crate::acs_text::util::{get_value_range, is_in_range};
use crate::acs_text::value::{find_hover_in_value, get_hover_for_simple_validator};
use std::path::Path;
use trainz_common::wiki::{get_wiki_container_name, get_wiki_kind_name};

#[tracing::instrument(skip(script_resolver))]
pub fn acs_text_hover(
    acs_text: &AcsText,
    params: HoverParams,
    validators: &Validators,
    base_path: Option<&Path>,
    script_resolver: Option<&dyn trainz_definition::acs_text::definitions::ScriptResolver>,
) -> Option<Hover> {
    let position = params.text_document_position_params.position;

    // Find kind-based validator for the whole acs_text if 'kind' key exists
    let kind_validator = acs_text
        .key_value_pairs
        .iter()
        .find(|kv| kv.key.eq_ignore_ascii_case("kind"))
        .and_then(|kv| {
            if let Some(Value::String(kind_name, _)) = &kv.value {
                validators
                    .containers
                    .par_iter()
                    .find_first(|v| v.container_name.eq_ignore_ascii_case(kind_name))
            } else if let Some(Value::Variable(kind_name, _)) = &kv.value {
                validators
                    .containers
                    .par_iter()
                    .find_first(|v| v.container_name.eq_ignore_ascii_case(kind_name))
            } else {
                None
            }
        });

    find_hover_recursive(
        &acs_text.key_value_pairs,
        position,
        validators,
        kind_validator,
        base_path,
        script_resolver,
        None,
        vec![],
    )
}

#[tracing::instrument(skip(script_resolver))]
pub fn find_hover_recursive(
    kvs: &[KeyValuePair],
    position: Position,
    validators: &Validators,
    current_validator: Option<&ContainerValidator>,
    base_path: Option<&Path>,
    script_resolver: Option<&dyn trainz_definition::acs_text::definitions::ScriptResolver>,
    trainz_build_version: Option<f64>,
    path: Vec<String>,
) -> Option<Hover> {
    // Find trainz-build version in current scope if not provided
    let trainz_build_version = trainz_build_version.or_else(|| {
        kvs.iter()
            .find(|kv| kv.key.eq_ignore_ascii_case("trainz-build"))
            .and_then(|kv| match &kv.value {
                Some(Value::Numeric(trainz_ast::acs_text::NumericValue::Float(f), _)) => Some(*f),
                Some(Value::Numeric(trainz_ast::acs_text::NumericValue::Int(i), _)) => {
                    Some(*i as f64)
                }
                Some(Value::String(s, _)) => s.parse::<f64>().ok(),
                _ => None,
            })
    });
    let validator_to_use = current_validator; // Simplified for now as we don't have kind_validator at this level easily

    for (index, kv) in kvs.iter().enumerate() {
        if is_in_range(position, &kv.key_range) {
            if let Some(validator) = validator_to_use {
                let mut rule_info = validator
                    .rules
                    .par_iter()
                    .find_first(|r| r.key.eq_ignore_ascii_case(&kv.key))
                    .map(|r| (r, false));

                if rule_info.is_none() {
                    rule_info = validator
                        .sub_possibilities
                        .par_iter()
                        .find_first(|r| r.key.eq_ignore_ascii_case(&kv.key))
                        .map(|r| (r, true));
                }

                if let Some((rule, _is_subpossibility)) = rule_info {
                    let mut current_path = path.clone();
                    current_path.push(rule.key.clone());

                    let mut hover = create_hover_from_rule(
                        rule,
                        &kv.key_range,
                        trainz_build_version,
                        &current_path,
                    );
                    if let Some(h) = &mut hover
                        && let Some(kind_name) = &rule.kind
                    {
                        let wiki_name = get_wiki_container_name(kind_name);
                        if let tower_lsp_server::ls_types::HoverContents::Markup(markup) =
                            &mut h.contents
                            && validators
                                .container_map
                                .contains_key(&kind_name.to_lowercase())
                        {
                            markup.value.push_str(&format!(
                                "**Wiki**: [\"{}\" container](https://online.ts2009.com/mediaWiki/index.php/\"{}\"_container)\n\n",
                                wiki_name, wiki_name
                            ));
                        }
                    }
                    return hover;
                }

                // Check if current container is a TagArray and if it has a type rule
                if let Some(tag_array) = &validator.tag_array {
                    let type_info = match tag_array {
                        ArrayElementType::Array(_, s) => Some(("tagarray".to_string(), s.clone())),
                        ArrayElementType::Tuple(types) => {
                            types.get(index).map(|(k, v)| (k.clone(), v.clone()))
                        }
                        ArrayElementType::Rule(rule) => rule
                            .type_name
                            .as_ref()
                            .map(|tn| ("tagarray".to_string(), tn.clone())),
                        ArrayElementType::Inline(_) => None,
                    };

                    if let Some((_type_key, type_name)) = type_info {
                        let type_validator =
                            validators.container_map.get(&type_name.to_lowercase());
                        if let Some(type_validator) = type_validator {
                            let mut current_path = path.clone();
                            current_path.push(kv.key.clone());

                            return Some(Hover {
                                contents: tower_lsp_server::ls_types::HoverContents::Markup(
                                    MarkupContent {
                                        kind: MarkupKind::Markdown,
                                        value: format!(
                                            "**Validator Path**: `{}`\n\n### TagArray Entry: `{}`\n\n**Validated against**: `{}`",
                                            current_path.join("/"),
                                            kv.key,
                                            type_validator.container_name
                                        ),
                                    },
                                ),
                                range: Some(kv.key_range),
                            });
                        }
                    }
                }

                // Check if current container has array-element and if the key is numeric
                if let Some(array_element) = &validator.array_element
                    && let Ok(idx) = kv.key.parse::<usize>()
                {
                    let _ = idx; // Suppress unused warning if needed, though we use it in Tuple
                    let type_info = match array_element {
                        ArrayElementType::Array(_, s) => {
                            Some(("container-type0".to_string(), s.clone()))
                        }
                        ArrayElementType::Tuple(types) => {
                            types.get(idx).map(|(k, v)| (k.clone(), v.clone()))
                        }
                        ArrayElementType::Rule(rule) => rule
                            .type_name
                            .as_ref()
                            .map(|tn| ("array-element".to_string(), tn.clone())),
                        ArrayElementType::Inline(_) => None,
                    };

                    if let Some((_type_key, type_name)) = type_info {
                        let type_validator =
                            validators.container_map.get(&type_name.to_lowercase());
                        if let Some(type_validator) = type_validator {
                            let mut current_path = path.clone();
                            current_path.push(kv.key.clone());

                            return Some(Hover {
                                contents: tower_lsp_server::ls_types::HoverContents::Markup(
                                    MarkupContent {
                                        kind: MarkupKind::Markdown,
                                        value: format!(
                                            "**Validator Path**: `{}`\n\n### Array Element: `{}`\n\n**Validated against**: `{}`",
                                            current_path.join("/"),
                                            kv.key,
                                            type_validator.container_name
                                        ),
                                    },
                                ),
                                range: Some(kv.key_range),
                            });
                        }
                    }
                }
            }

            // Top-level or simple validator (fall-through)
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
                                        .par_iter()
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
                        range: Some(kv.key_range),
                    });
                }
            }

            // Check top-level container match
            let container_validator = validators
                .containers
                .par_iter()
                .find_first(|v| v.container_name.eq_ignore_ascii_case(&kv.key));
            if let Some(validator) = container_validator {
                let wiki_name = get_wiki_container_name(&validator.container_name);
                return Some(Hover {
                    contents: tower_lsp_server::ls_types::HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: format!(
                            "### Container: `{}`\n\n**Wiki**: [\"{}\" container](https://online.ts2009.com/mediaWiki/index.php/\"{}\"_container)",
                            kv.key, wiki_name, wiki_name
                        ),
                    }),
                    range: Some(kv.key_range),
                });
            }
        }

        // Hover over value
        if let Some(value) = &kv.value {
            let value_range = get_value_range(value);
            if is_in_range(position, &value_range) {
                if let Value::Container(inner_kvs, _, _) = value {
                    // Determine the validator for this container.
                    let next_validator = if let Some(cv) = validator_to_use {
                        if let Some(array_element_type) = &cv.array_element {
                            match array_element_type {
                                ArrayElementType::Array(_, s) => {
                                    validators.container_map.get(&s.to_lowercase())
                                }
                                ArrayElementType::Tuple(types) => kv
                                    .key
                                    .parse::<usize>()
                                    .ok()
                                    .and_then(|idx| types.get(idx))
                                    .and_then(|(_, tn)| {
                                        validators.container_map.get(&tn.to_lowercase())
                                    }),
                                ArrayElementType::Inline(iv) => Some(iv.as_ref()),
                                ArrayElementType::Rule(rule) => {
                                    rule.child_validator.as_ref().map(|b| b.as_ref()).or_else(
                                        || {
                                            rule.type_name.as_ref().and_then(|tn| {
                                                validators.container_map.get(&tn.to_lowercase())
                                            })
                                        },
                                    )
                                }
                            }
                        } else {
                            let rule = cv
                                .rules
                                .par_iter()
                                .find_first(|r| r.key.eq_ignore_ascii_case(&kv.key))
                                .or_else(|| {
                                    cv.sub_possibilities
                                        .par_iter()
                                        .find_first(|r| r.key.eq_ignore_ascii_case(&kv.key))
                                });
                            if let Some(rule) = rule {
                                if let Some(child_validator) = &rule.child_validator {
                                    Some(child_validator.as_ref())
                                } else if let Some(type_name) = &rule.type_name {
                                    validators.container_map.get(&type_name.to_lowercase())
                                } else {
                                    None
                                }
                            } else {
                                // Check if this is a TagArray and the key is an entry (not 'type')
                                if let Some(tag_array) = &cv.tag_array {
                                    match tag_array {
                                        ArrayElementType::Array(_, s) => {
                                            validators.container_map.get(&s.to_lowercase())
                                        }
                                        ArrayElementType::Tuple(types) => {
                                            types.get(index).and_then(|(_, tn)| {
                                                validators.container_map.get(&tn.to_lowercase())
                                            })
                                        }
                                        ArrayElementType::Inline(iv) => Some(iv.as_ref()),
                                        ArrayElementType::Rule(rule) => rule
                                            .child_validator
                                            .as_ref()
                                            .map(|b| b.as_ref())
                                            .or_else(|| {
                                                rule.type_name.as_ref().and_then(|tn| {
                                                    validators.container_map.get(&tn.to_lowercase())
                                                })
                                            }),
                                    }
                                } else {
                                    None
                                }
                            }
                        }
                    } else {
                        validators.container_map.get(&kv.key.to_lowercase())
                    };

                    let mut next_path = path.clone();
                    if let Some(cv) = validator_to_use {
                        if cv.array_element.is_some() {
                            next_path.push(kv.key.clone());
                        } else if let Some(rule) = cv
                            .rules
                            .par_iter()
                            .find_first(|r| r.key.eq_ignore_ascii_case(&kv.key))
                        {
                            next_path.push(rule.key.clone());
                        } else if let Some(rule) = cv
                            .sub_possibilities
                            .par_iter()
                            .find_first(|r| r.key.eq_ignore_ascii_case(&kv.key))
                        {
                            next_path.push(rule.key.clone());
                        } else if cv.tag_array.is_some() {
                            next_path.push(kv.key.clone());
                        }
                    } else {
                        next_path.push(kv.key.clone());
                    };

                    let nested_hover = find_hover_recursive(
                        inner_kvs,
                        position,
                        validators,
                        next_validator,
                        base_path,
                        script_resolver,
                        trainz_build_version,
                        next_path,
                    );
                    if nested_hover.is_some() {
                        return nested_hover;
                    }
                }

                // Check for script/class specific hover
                if (kv.key.eq_ignore_ascii_case("script") || kv.key.eq_ignore_ascii_case("class"))
                    && let Some(Value::String(s, _)) = &kv.value
                    && let Some(base) = base_path
                    && let Some(resolver) = script_resolver
                {
                    let script_name = if kv.key.eq_ignore_ascii_case("script") {
                        Some(s.as_str())
                    } else {
                        kvs.iter().find_map(|kv| {
                            if kv.key.eq_ignore_ascii_case("script")
                                && let Some(Value::String(s, _)) = &kv.value
                            {
                                Some(s.as_str())
                            } else {
                                None
                            }
                        })
                    };

                    if let Some(script_name) = script_name
                        && let Some((_uri, program)) = resolver.resolve_script(base, script_name)
                    {
                        let hover_text = if kv.key.eq_ignore_ascii_case("script") {
                            format!("### Script: `{}`\n\nPath: `{}`", s, program.src)
                        } else {
                            if let Some(class_def) = program.classes.get(s) {
                                let mut text = format!("### Class: `{}`\n\n", s);
                                if !class_def.superclasses.is_empty() {
                                    text.push_str("**Inherits from**: ");
                                    text.push_str(
                                        &class_def
                                            .superclasses
                                            .iter()
                                            .map(|sc| format!("`{}`", sc.name))
                                            .collect::<Vec<_>>()
                                            .join(", "),
                                    );
                                    text.push_str("\n\n");
                                }
                                text
                            } else {
                                format!(
                                    "### Class: `{}` (Not found in script `{}`)",
                                    s, script_name
                                )
                            }
                        };

                        return Some(Hover {
                            contents: tower_lsp_server::ls_types::HoverContents::Markup(
                                MarkupContent {
                                    kind: MarkupKind::Markdown,
                                    value: hover_text,
                                },
                            ),
                            range: Some(value_range),
                        });
                    }
                }

                // Check for image preview
                if let Some(validator) = current_validator {
                    let rule = validator
                        .rules
                        .par_iter()
                        .find_first(|r| r.key.eq_ignore_ascii_case(&kv.key));
                    if let Some(rule) = rule
                        && rule.kind.as_deref() == Some("image")
                        && let Some(Value::String(s, _)) = &kv.value
                        && let Some(base) = base_path
                    {
                        let file_path = if let Some(parent) = base.parent() {
                            parent.join(s)
                        } else {
                            Path::new(s).to_path_buf()
                        };
                        if file_path.exists() {
                            return Some(Hover {
                                contents: tower_lsp_server::ls_types::HoverContents::Markup(
                                    MarkupContent {
                                        kind: MarkupKind::Markdown,
                                        value: format!(
                                            "### Image: `{}`\n\n![Preview](file://{})",
                                            s,
                                            file_path.to_string_lossy()
                                        ),
                                    },
                                ),
                                range: Some(value_range),
                            });
                        }
                    }
                }

                // If not in a deeper one, check if we have value hover here
                if kv.key.eq_ignore_ascii_case("kind") {
                    let value_str = match &kv.value {
                        Some(Value::String(s, _)) => s.clone(),
                        Some(Value::Variable(s, _)) => s.clone(),
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
                            range: Some(value_range),
                        });
                    }
                }
                // Check simple validators for top-level keys
                if let Some(validator) = validators.simple.get(&kv.key) {
                    let value_str = match &kv.value {
                        Some(Value::String(s, _)) => s.clone(),
                        Some(Value::Variable(s, _)) => s.clone(),
                        _ => String::new(),
                    };
                    return get_hover_for_simple_validator(&value_str, validator, &value_range);
                }

                // If no other hover found, check if it's a hover in the value itself
                if let Some(v) = &kv.value {
                    let mut next_path = path.clone();
                    if let Some(cv) = validator_to_use {
                        next_path.push(cv.container_name.clone());
                    }
                    return find_hover_in_value(
                        v,
                        position,
                        validators,
                        None,
                        base_path,
                        script_resolver,
                        trainz_build_version,
                        next_path,
                    );
                }
            }
        }
    }

    None
}
