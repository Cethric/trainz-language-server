use tower_lsp_server::ls_types::{
    CompletionItem, CompletionItemKind, Documentation, InsertTextFormat, InsertTextMode,
    MarkupContent, MarkupKind, Position, Range,
};
use tracing::debug;
use trainz_ast::soup::value::Value;
use trainz_common::wiki::get_wiki_kind_name;
use trainz_soup_validators::Validators;

pub fn add_value_completions(
    key: &str,
    value: Option<&Value>,
    position: Position,
    validators: &Validators,
    rule: Option<&trainz_soup_validators::ContainerRule>,
    completions: &mut Vec<CompletionItem>,
) {
    debug!("Adding value completions for key '{}'", key);

    let current_val = match value {
        Some(Value::String(s, _)) => s.clone(),
        _ => "".to_string(),
    };

    let selected_values: Vec<String> = current_val
        .split(';')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();

    let range = match value {
        Some(Value::String(_, r)) => *r,
        _ => Range::default(),
    };

    let mut filter_text = "".to_string();
    if position.line == range.start.line && position.character > range.start.character {
        let offset = position.character as usize - range.start.character as usize;
        if offset <= current_val.len() {
            filter_text = current_val[..offset].to_lowercase();
        } else {
            filter_text = current_val.to_lowercase();
        }
    }

    if filter_text.contains(';') {
        filter_text = filter_text
            .split(';')
            .next_back()
            .unwrap_or("")
            .trim()
            .to_string();
    } else {
        filter_text = filter_text.trim().to_string();
    }

    debug!(
        "DEBUG: filter_text: '{}', selected_values: {:?}",
        filter_text, selected_values
    );

    // Named validators from the rule
    if let Some(rule) = rule
        && let Some(validations) = &rule.validation
    {
        for validation in validations {
            if let trainz_soup_validators::Validation::Named(name) = validation {
                // Try mapping named validation to simple validator key
                let simple_key = match name.as_str() {
                    "IsValidCategoryEra" => Some("category-era"),
                    "IsValidCategoryRegion" => Some("category-region"),
                    "IsValidCategoryClass" => Some("category-class"),
                    _ => None,
                };

                if let Some(s_key) = simple_key
                    && let Some(options) = validators.simple.get(s_key)
                {
                    for (value_str, description) in options {
                        if selected_values.contains(&value_str.to_lowercase())
                            && filter_text.is_empty()
                        {
                            continue;
                        }
                        let label = value_str.clone();
                        let detail = description.clone();
                        completions.push(CompletionItem {
                            label,
                            detail,
                            kind: Some(CompletionItemKind::ENUM_MEMBER),
                            insert_text: Some(value_str.to_string()),
                            insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                            insert_text_mode: Some(InsertTextMode::AS_IS),
                            ..Default::default()
                        });
                    }
                } else {
                    // Treat as comma-separated list of values
                    let values = name.split(',');
                    for val in values {
                        let val = val.trim();
                        if val.is_empty()
                            || (selected_values.contains(&val.to_lowercase())
                                && filter_text.is_empty())
                        {
                            continue;
                        }
                        completions.push(CompletionItem {
                            label: val.to_string(),
                            kind: Some(CompletionItemKind::ENUM_MEMBER),
                            insert_text: Some(val.to_string()),
                            insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                            insert_text_mode: Some(InsertTextMode::AS_IS),
                            ..Default::default()
                        });
                    }
                }
            }
        }
    }

    // Simple validators from the global validators map
    if let Some(options) = validators.simple.get(key) {
        for (value_str, description) in options {
            if selected_values.contains(&value_str.to_lowercase()) && filter_text.is_empty() {
                continue;
            }
            let label = value_str.clone();
            let detail = description.clone();
            completions.push(CompletionItem {
                label,
                detail,
                kind: Some(CompletionItemKind::ENUM_MEMBER),
                insert_text: Some(value_str.to_string()),
                insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                insert_text_mode: Some(InsertTextMode::AS_IS),
                ..Default::default()
            });
        }
    }

    // Special keys that suggest container names (kind, inheritance, etc.)
    if key.eq_ignore_ascii_case("kind")
        || key.eq_ignore_ascii_case("inheritance")
        || key.eq_ignore_ascii_case("effect-layer")
        || key.eq_ignore_ascii_case("container")
    {
        let range = match value {
            Some(Value::String(_, r)) => *r,
            _ => Range::default(),
        };

        let mut filter_text = "".to_string();
        if position.line == range.start.line && position.character > range.start.character {
            let offset = position.character as usize - (range.start.character as usize + 1);
            if offset <= current_val.len() {
                filter_text = current_val[..offset].to_lowercase();
            } else {
                filter_text = current_val.to_lowercase();
            }
        }

        // For multi-value strings like inheritance, filter text should be from last semicolon
        if filter_text.contains(';') {
            filter_text = filter_text
                .split(';')
                .next_back()
                .unwrap_or("")
                .trim()
                .to_string();
        }

        let selected_values: Vec<String> = current_val
            .split(';')
            .map(|s| s.trim().to_lowercase())
            .collect();

        for validator in &validators.containers {
            if selected_values.contains(&validator.container_name.to_lowercase()) {
                continue;
            }

            // suggest top-level for 'kind' at top-level, or any container for other keys/contexts
            if (validator.top_level || !key.eq_ignore_ascii_case("kind"))
                && (filter_text.is_empty()
                    || validator
                        .container_name
                        .to_lowercase()
                        .contains(&filter_text))
            {
                let wiki_name = get_wiki_kind_name(&validator.container_name);
                completions.push(CompletionItem {
                    label: validator.container_name.clone(),
                    kind: Some(CompletionItemKind::ENUM_MEMBER),
                    documentation: Some(Documentation::MarkupContent(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: format!(
                            "**Wiki**: [KIND {}](https://online.ts2009.com/mediaWiki/index.php/KIND_{})\n\n",
                            wiki_name, wiki_name
                        ),
                    })),
                    ..Default::default()
                });
            }
        }
    }
}
