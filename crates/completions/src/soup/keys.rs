use log::debug;
use rayon::prelude::*;
use tower_lsp_server::ls_types::{
    CompletionItem, CompletionItemKind, Documentation, InsertTextFormat,
};
use trainz_soup_validators::{ArrayElementType, ContainerValidator};

pub fn add_key_completions_from_validator(
    validator: &ContainerValidator,
    completions: &mut Vec<CompletionItem>,
    current_text: &str,
) {
    debug!(
        "Adding key completions from validator '{}' with filter '{}'",
        validator.container_name, current_text
    );
    let is_complete_key_match = validator
        .rules
        .iter()
        .any(|r| r.key.eq_ignore_ascii_case(current_text))
        || validator
            .sub_possibilities
            .iter()
            .any(|r| r.key.eq_ignore_ascii_case(current_text));
    let filter = if is_complete_key_match {
        ""
    } else {
        current_text
    };

    for rule in &validator.rules {
        if filter.is_empty() || rule.key.to_lowercase().contains(&filter.to_lowercase()) {
            completions.push(CompletionItem {
                label: rule.key.clone(),
                kind: Some(CompletionItemKind::FIELD),
                detail: rule.type_name.clone(),
                documentation: rule.validation.as_ref().map(|v| {
                    Documentation::String(format!(
                        "Validation:\n{}",
                        v.par_iter()
                            .map(|v| format!("- {:?}", v))
                            .collect::<Vec<_>>()
                            .join("\n")
                    ))
                }),
                ..Default::default()
            });
        }
    }

    // Also subpossibilities for top-level
    if validator.top_level {
        for sub in &validator.sub_possibilities {
            if filter.is_empty() || sub.key.to_lowercase().contains(&filter.to_lowercase()) {
                completions.push(CompletionItem {
                    label: sub.key.clone(),
                    kind: Some(CompletionItemKind::FIELD),
                    detail: sub.type_name.clone(),
                    documentation: sub.validation.as_ref().map(|v| {
                        Documentation::String(format!(
                            "Validation:\n{}",
                            v.par_iter()
                                .map(|v| format!("- {:?}", v))
                                .collect::<Vec<_>>()
                                .join("\n")
                        ))
                    }),
                    ..Default::default()
                });
            }
        }
    }

    // If it's an array-element container, suggest that it takes any key
    if let Some(array_element_type) = &validator.array_element {
        let detail = format!("Element type: {}", array_element_type);
        let documentation = match array_element_type {
            ArrayElementType::Array(s) => format!(
                "This container accepts elements of type '{}' with any unique name.",
                s
            ),
            ArrayElementType::Tuple(types) => format!(
                "This container is a tuple of types: [{}].",
                types.join(", ")
            ),
        };

        completions.push(CompletionItem {
            label: match array_element_type {
                ArrayElementType::Array(_) => "element_name".to_string(),
                ArrayElementType::Tuple(_) => "0".to_string(), // Start with first index for tuple
            },
            kind: Some(CompletionItemKind::SNIPPET),
            detail: Some(detail),
            documentation: Some(Documentation::String(documentation)),
            insert_text: Some("${1:name} {\n\t$0\n}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
    }
}
