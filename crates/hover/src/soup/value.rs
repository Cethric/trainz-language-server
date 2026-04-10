use crate::soup::find_hover_recursive;
use rayon::prelude::*;
use std::collections::HashMap;
use tower_lsp_server::ls_types::{Hover, MarkupContent, MarkupKind, Position};
use trainz_ast::soup::value::Value;
use trainz_soup_validators::{ContainerRule, Validators};

pub fn find_hover_in_value(
    value: &Value,
    position: Position,
    validators: &Validators,
    rule: Option<&ContainerRule>,
) -> Option<Hover> {
    match value {
        Value::Container(container_kv, _, _) => {
            let mut next_validator = None;
            if let Some(rule) = rule {
                if let Some(kind_name) = &rule.kind {
                    next_validator = validators
                        .containers
                        .par_iter()
                        .find_first(|v| v.container_name.eq_ignore_ascii_case(kind_name));
                } else if let Some(type_name) = &rule.type_name {
                    next_validator = validators
                        .containers
                        .par_iter()
                        .find_first(|v| v.container_name.eq_ignore_ascii_case(type_name));
                }
            }
            find_hover_recursive(container_kv, position, validators, next_validator)
        }
        _ => None,
    }
}

pub fn get_hover_for_simple_validator(
    value_str: &str,
    allowed_values: &HashMap<String, Option<String>>,
    value_range: &trainz_ast::Range,
) -> Option<Hover> {
    let mut value_doc = String::new();
    let values: Vec<&str> = value_str.split(';').map(|s| s.trim()).collect();

    let mut found = false;
    for v in &values {
        if let Some(desc) = allowed_values.get(*v) {
            value_doc.push_str(&if let Some(desc) = desc {
                format!("**Value**: `{}`\n\n**Description**: {}\n\n", v, desc)
            } else {
                format!("**Value**: `{}`\n\n", v)
            });
            found = true;
        }
    }

    if !found && !allowed_values.is_empty() {
        value_doc.push_str(&format!("**Value**: `{}`\n\n", value_str));
    }

    if !allowed_values.is_empty() {
        value_doc.push_str("#### Available Options\n");
        let mut sorted_options: Vec<_> = allowed_values.par_iter().collect();
        sorted_options.sort_by(|a, b| a.0.cmp(b.0));
        for (opt, desc) in sorted_options {
            value_doc.push_str(&if let Some(desc) = desc {
                format!("- `{}`: {}\n", opt, desc)
            } else {
                format!("- `{}`\n", opt)
            });
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
        range: Some(*value_range),
    })
}
