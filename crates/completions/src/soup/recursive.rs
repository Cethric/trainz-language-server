use rayon::prelude::*;
use tower_lsp_server::ls_types::{CompletionItem, Position};
use tracing::debug;
use trainz_ast::soup::key_value_pair::KeyValuePair;
use trainz_ast::soup::value::Value;
use trainz_soup_validators::{ArrayElementType, ContainerValidator, Validators};

use crate::soup::keys::add_key_completions_from_validator;
use crate::soup::utils::is_in_range;
use crate::soup::values::add_value_completions;

#[tracing::instrument]
pub fn find_completions_recursive(
    kvs: &[KeyValuePair],
    position: Position,
    validators: &Validators,
    current_validator: Option<&ContainerValidator>,
) -> Vec<CompletionItem> {
    let mut completions = vec![];

    debug!(
        "Checking {} KVs for completions at current level",
        kvs.len()
    );
    debug!(
        "Current validator: {:?}",
        current_validator.map(|v| &v.container_name)
    );
    let mut current_validator_to_use = current_validator;

    // 0. Pre-process to find the container's kind if we're in a container or at top-level.
    // This helps in providing context-aware key suggestions.
    if let Some(kind_kv) = kvs
        .par_iter()
        .find_first(|kv| kv.key.eq_ignore_ascii_case("kind"))
        && let Some(Value::String(kind_val, _)) = &kind_kv.value
        && let Some(v) = validators
            .containers
            .par_iter()
            .find_first(|v| v.container_name.eq_ignore_ascii_case(kind_val))
    {
        debug!("Using validator for kind '{}'", kind_val);
        current_validator_to_use = Some(v);
    }

    if let Some(v) = current_validator_to_use {
        debug!(
            "Effective validator for this container: '{}'",
            v.container_name
        );
    } else {
        debug!("No effective validator for this container");
    }

    for kv in kvs {
        let rule = current_validator_to_use.as_ref().and_then(|cv| {
            cv.rules
                .par_iter()
                .find_first(|r| r.key.eq_ignore_ascii_case(&kv.key))
        });

        // If we are on the key itself, check if the cursor is within the value's range first.
        if let Some(value) = &kv.value {
            let range = value.range();
            if is_in_range(position, &range) {
                debug!(
                    "Position is in range of value for key '{}' ({:?})",
                    kv.key, range
                );
                // Handle nested container
                if let Value::Container(inner_kvs, _, _) = value {
                    debug!("Traversing into nested container '{}'", kv.key);
                    let next_validator = if let Some(cv) = current_validator_to_use {
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
                                    .par_iter()
                                    .find_first(|v| v.container_name.eq_ignore_ascii_case(tn))
                            })
                        } else {
                            if let Some(rule) = rule {
                                let mut found_validator = None;
                                if let Some(type_name) = &rule.type_name {
                                    found_validator =
                                        validators.containers.par_iter().find_first(|v| {
                                            v.container_name.eq_ignore_ascii_case(type_name)
                                        });
                                }
                                found_validator
                            } else {
                                None
                            }
                        }
                    } else {
                        debug!("Searching for validator for key '{}'", kv.key);
                        let v = validators
                            .containers
                            .par_iter()
                            .find_first(|v| v.container_name.eq_ignore_ascii_case(&kv.key));
                        if let Some(v) = v {
                            debug!("Found validator '{}'", v.container_name);
                        } else {
                            debug!("No validator found for '{}'", kv.key);
                        }
                        v
                    };

                    if let Some(nv) = next_validator {
                        debug!(
                            "Next validator for nested container: '{}'",
                            nv.container_name
                        );
                    } else {
                        debug!("No next validator found for nested container '{}'", kv.key);
                    }

                    let nested_completions =
                        find_completions_recursive(inner_kvs, position, validators, next_validator);
                    if !nested_completions.is_empty() {
                        return nested_completions;
                    }
                }

                // If not handled by recursion, compute value completions for this key
                add_value_completions(
                    &kv.key,
                    Some(value),
                    position,
                    validators,
                    rule,
                    &mut completions,
                );
                if !completions.is_empty() {
                    return completions;
                }
            }
        }

        // If we are on the key itself, we should return key completions and NOT fall through
        if is_in_range(position, &kv.key_range) {
            debug!(
                "Position is in range of key '{}' ({:?})",
                kv.key, kv.key_range
            );

            // Check if we are at the very end of the key.
            // If so, we might want to suggest values instead.
            if position.line == kv.key_range.end.line
                && position.character == kv.key_range.end.character
            {
                debug!("At end of key '{}', suggesting values", kv.key);
                add_value_completions(
                    &kv.key,
                    kv.value.as_ref(),
                    position,
                    validators,
                    None,
                    &mut completions,
                );
                if !completions.is_empty() {
                    return completions;
                }
            }

            let current_text = kv.key.to_lowercase();
            if let Some(validator) = current_validator_to_use {
                debug!(
                    "Current container validator: '{}'",
                    validator.container_name
                );
                add_key_completions_from_validator(validator, &mut completions, &current_text);
            } else {
                debug!("At top-level key level, suggesting 'kind' and containers");
                // Suggest kind values if we are at the end of 'kind' key
                if position.character == kv.key_range.end.character
                    && kv.key.eq_ignore_ascii_case("kind")
                {
                    add_value_completions(
                        &kv.key,
                        kv.value.as_ref(),
                        position,
                        validators,
                        None,
                        &mut completions,
                    );
                } else {
                    for validator in &validators.containers {
                        if validator.top_level {
                            completions.push(CompletionItem {
                                label: validator.container_name.clone(),
                                kind: Some(tower_lsp_server::ls_types::CompletionItemKind::CLASS),
                                ..Default::default()
                            });
                        }
                    }
                    if !kv.key.eq_ignore_ascii_case("kind") {
                        completions.push(CompletionItem {
                            label: "kind".to_string(),
                            kind: Some(tower_lsp_server::ls_types::CompletionItemKind::KEYWORD),
                            ..Default::default()
                        });
                    }
                }
            }
            if !completions.is_empty() {
                return completions;
            }
        }

        // Check if cursor is at end of key, check if we should suggest values
        if position.line == kv.key_range.end.line
            && position.character >= kv.key_range.end.character
        {
            // If there's no value yet, suggest values for this key
            if kv.value.is_none() {
                debug!("No value for key '{}', suggesting values", kv.key);
                add_value_completions(&kv.key, None, position, validators, None, &mut completions);
                if !completions.is_empty() {
                    return completions;
                }
            }
        }
    }

    // 2. If we're not inside any value, we are at the level of the keys of 'kvs'.
    let local_validator = if let Some(kind_kv) = kvs
        .par_iter()
        .find_first(|kv| kv.key.eq_ignore_ascii_case("kind"))
    {
        if let Some(Value::String(kind_val, _)) = &kind_kv.value {
            validators
                .containers
                .par_iter()
                .find_first(|v| v.container_name.eq_ignore_ascii_case(kind_val))
        } else {
            None
        }
    } else {
        None
    };

    let validator_to_use = local_validator.or(current_validator);

    if let Some(validator) = validator_to_use {
        debug!(
            "At key level for validator '{}', suggesting keys",
            validator.container_name
        );
        let current_text = kvs
            .par_iter()
            .find_first(|kv| is_in_range(position, &kv.key_range))
            .map(|kv| kv.key.to_lowercase())
            .unwrap_or_default();
        add_key_completions_from_validator(validator, &mut completions, &current_text);
    }

    if current_validator.is_none() {
        debug!("At top-level key level, suggesting 'kind' and containers");
        // Top-level keys (when not in any container)
        // Suggest "kind" and other top-level container names
        // Check if we are at the end of some existing key-value pair and if it's already kind
        let already_has_kind = kvs.par_iter().any(|kv| kv.key.eq_ignore_ascii_case("kind"));

        // Use substring matching for top-level key suggestions too
        let current_text = kvs
            .par_iter()
            .find_first(|kv| is_in_range(position, &kv.key_range))
            .map(|kv| kv.key.to_lowercase())
            .unwrap_or_default();

        if !already_has_kind && ("kind".contains(&current_text) || current_text.is_empty()) {
            completions.push(CompletionItem {
                label: "kind".to_string(),
                kind: Some(tower_lsp_server::ls_types::CompletionItemKind::KEYWORD),
                ..Default::default()
            });
        }

        for validator in &validators.containers {
            if validator.top_level
                && (validator
                    .container_name
                    .to_lowercase()
                    .contains(&current_text)
                    || current_text.is_empty())
            {
                completions.push(CompletionItem {
                    label: validator.container_name.clone(),
                    kind: Some(tower_lsp_server::ls_types::CompletionItemKind::CLASS),
                    ..Default::default()
                });
            }
        }
    }

    completions
}
