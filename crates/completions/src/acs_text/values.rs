use tower_lsp_server::ls_types::{
    CompletionItem, CompletionItemKind, Documentation, InsertTextFormat, InsertTextMode,
    MarkupContent, MarkupKind, Position, Range,
};
use tracing::debug;
use trainz_acs_text_validators::Validators;
use trainz_ast::acs_text::value::Value;
use trainz_common::wiki::get_wiki_kind_name;

#[tracing::instrument(skip(key, value, position, validators, rule, completions, asset_cache_path))]
pub fn add_value_completions(
    key: &str,
    value: Option<&Value>,
    position: Position,
    validators: &Validators,
    rule: Option<&trainz_acs_text_validators::ContainerRule>,
    completions: &mut Vec<CompletionItem>,
    asset_cache_path: Option<&std::path::Path>,
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

    if let Some(rule) = rule
        && let Some(type_name) = &rule.type_name
    {
        let validator_name = rule.source.as_ref().unwrap_or(type_name);
        if let Some(options) = validators.simple.get(validator_name) {
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

        if (type_name == "kuidbrowser" || type_name == "stringkuidbrowser")
            && let Some(cache_path) = asset_cache_path
            && let Ok(conn) = rusqlite::Connection::open(cache_path)
            && let Ok(mut stmt) = conn.prepare("SELECT kuid, username, acs FROM assets")
        {
            let asset_iter = stmt.query_map([], |row| {
                let kuid_val: i64 = row.get(0)?;
                let username: Option<String> = row.get(1)?;
                let acs: Vec<u8> = row.get(2)?;
                Ok((kuid_val, username, acs))
            });

            if let Ok(asset_iter) = asset_iter {
                for asset in asset_iter.flatten() {
                    let (kuid_val, username, acs) = asset;
                    let (user_id, content_id, version) = trainz_tdx::TdxValue::split_kuid(kuid_val);
                    let kuid_str = if type_name == "stringkuidbrowser" {
                        format!("<kuid2:{}:{}:{}>", user_id, content_id, version)
                    } else {
                        format!("kuid2:{}:{}:{}", user_id, content_id, version)
                    };

                    let mut documentation = String::new();
                    if let Ok(parsed_acs) = trainz_tdx::acs_bin::AcsParser::new(&acs).parse()
                        && let trainz_tdx::TdxValue::Container(entries) = parsed_acs
                    {
                        for (tag, val) in entries {
                            if tag == "description"
                                && let trainz_tdx::TdxValue::String(s) = val
                            {
                                documentation = s;
                            }
                        }
                    }

                    completions.push(CompletionItem {
                        label: kuid_str.clone(),
                        detail: username,
                        kind: Some(CompletionItemKind::VALUE),
                        documentation: if documentation.is_empty() {
                            None
                        } else {
                            Some(Documentation::String(documentation))
                        },
                        insert_text: Some(kuid_str),
                        ..Default::default()
                    });
                }
            }
        }
    }

    // Named validators from the rule
    if let Some(rule) = rule
        && let Some(validations) = &rule.validation
    {
        for validation in validations {
            if let trainz_acs_text_validators::Validation::Named(name) = validation {
                // Try mapping named validation to simple validator key
                let simple_key = match name.as_str() {
                    "IsValidCategoryEra" => Some("category-era"),
                    "IsValidCategoryRegion" => Some("category-region"),
                    "IsValidCategoryClass" => Some("category-class"),
                    _ => Some(name.as_str()),
                };

                if let Some(s_key) = simple_key
                    && (validators.simple.contains_key(s_key)
                        || validators.simple.contains_key(&s_key.to_lowercase()))
                {
                    let options = validators
                        .simple
                        .get(s_key)
                        .or_else(|| validators.simple.get(&s_key.to_lowercase()))
                        .unwrap();
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
