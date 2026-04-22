use rayon::prelude::*;
use std::collections::HashMap;
use tower_lsp_server::ls_types::{Hover, MarkupContent, MarkupKind, Position};
use trainz_ast::acs_text::value::Value;

#[tracing::instrument(skip(value, position, base_path, script_resolver, trainz_build_version))]
pub fn find_hover_in_value(
    value: &Value,
    position: Position,
    base_path: Option<&std::path::Path>,
    script_resolver: Option<&dyn trainz_definition::acs_text::definitions::ScriptResolver>,
    trainz_build_version: Option<f64>,
) -> Option<Hover> {
    match value {
        Value::Container(container_kv, _, _) => {
            crate::acs_text::acs_text_hover_recursive_wrapper::acs_text_hover_recursive_wrapper(
                container_kv,
                position,
                base_path,
                script_resolver,
                trainz_build_version,
            )
        }
        _ => None,
    }
}

#[tracing::instrument(skip(value_str, allowed_values, value_range))]
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
