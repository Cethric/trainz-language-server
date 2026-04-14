use crate::acs_text::validate_value::validate_value;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::path::Path;
use tower_lsp_server::ls_types::{Diagnostic, DiagnosticSeverity};
use trainz_acs_text_validators::{ArrayElementType, ContainerValidator, Validation, Validators};
use trainz_ast::acs_text::Value;

#[tracing::instrument]
fn validate_compulsory_keys(
    container_kv: &[trainz_ast::acs_text::KeyValuePair],
    found_keys: &std::collections::HashSet<String>,
    validator: &ContainerValidator,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Check compulsory keys in rules
    for rule in &validator.rules {
        if rule.compulsory.is_some_and(|c| c >= 1.0)
            && !rule.disabled.unwrap_or(false)
            && !found_keys.contains(&rule.key.to_lowercase())
        {
            let range = container_kv.first().map(|kv| kv.range).unwrap_or_default();
            diagnostics.push(Diagnostic {
                range,
                severity: Some(DiagnosticSeverity::ERROR),
                message: format!(
                    "Compulsory key '{}' is missing in container '{}'.",
                    rule.key, validator.container_name
                ),
                source: Some(String::from("acs_text-validator")),
                ..Default::default()
            });
        }
    }

    // Check compulsory keys in subpossibilities
    for rule in &validator.sub_possibilities {
        if rule.compulsory.is_some_and(|c| c >= 1.0)
            && !rule.disabled.unwrap_or(false)
            && !found_keys.contains(&rule.key.to_lowercase())
        {
            let range = container_kv.first().map(|kv| kv.range).unwrap_or_default();
            diagnostics.push(Diagnostic {
                range,
                severity: Some(DiagnosticSeverity::ERROR),
                message: format!(
                    "Compulsory key '{}' is missing in container '{}'.",
                    rule.key, validator.container_name
                ),
                source: Some(String::from("acs_text-validator")),
                ..Default::default()
            });
        }
    }
}

#[tracing::instrument]
pub fn validate_container(
    container_kv: &[trainz_ast::acs_text::KeyValuePair],
    validator: &ContainerValidator,
    all_validators: &Validators,
    diagnostics: &mut Vec<Diagnostic>,
    key_to_ignore: Option<&str>,
    base_path: Option<&Path>,
) {
    let mut found_keys = std::collections::HashSet::new();
    let unique_names = validator.validation.as_ref().is_some_and(|v| {
        v.par_iter().any(|val| matches!(val, Validation::Named(name) if name.eq_ignore_ascii_case("UniqueNames") || name.eq_ignore_ascii_case("SubPossibilities")))
    });

    for kv in container_kv {
        if unique_names && !found_keys.insert(kv.key.to_lowercase()) {
            diagnostics.push(Diagnostic {
                range: kv.key_range,
                severity: Some(DiagnosticSeverity::ERROR),
                message: format!(
                    "Duplicate key '{}' in container '{}'",
                    kv.key, validator.container_name
                ),
                source: Some(String::from("acs_text-validator")),
                ..Default::default()
            });
        } else {
            found_keys.insert(kv.key.to_lowercase());
        }
    }

    validate_compulsory_keys(container_kv, &found_keys, validator, diagnostics);

    let is_tag_array = validator.tag_array.is_some() || validator.validation.as_ref().is_some_and(|v| {
        v.par_iter().any(|val| matches!(val, Validation::Named(name) if name.eq_ignore_ascii_case("tagarray") || name.eq_ignore_ascii_case("tag-array")))
    });

    if let Some(tag_array) = &validator.tag_array {
        for kv in container_kv {
            if let Some(value) = &kv.value {
                validate_value(
                    value,
                    tag_array,
                    all_validators,
                    diagnostics,
                    &validator.container_name,
                    base_path,
                );
            }
        }
    }

    if let Some(array_element) = &validator.array_element {
        match array_element {
            ArrayElementType::Array(array_type) => {
                if let Some(element_validator) =
                    all_validators.container_map.get(&array_type.to_lowercase())
                {
                    for kv in container_kv {
                        if let Some(Value::Container(value, _, _)) = &kv.value {
                            validate_container(
                                value,
                                element_validator,
                                all_validators,
                                diagnostics,
                                key_to_ignore,
                                base_path,
                            );
                        }
                    }
                }
            }
            ArrayElementType::Tuple(tuple) => {
                for (index, array_type) in tuple.iter().enumerate() {
                    let key = index.to_string();
                    if let Some(kv) = container_kv
                        .par_iter()
                        .find_first(|kv| kv.key.eq_ignore_ascii_case(&key))
                        && let Some(Value::Container(value, _, _)) = &kv.value
                        && let Some(element_validator) =
                            all_validators.container_map.get(&array_type.to_lowercase())
                    {
                        validate_container(
                            value,
                            element_validator,
                            all_validators,
                            diagnostics,
                            key_to_ignore,
                            base_path,
                        );
                    } else {
                        // Diagnostic: missing element at index
                    }
                }
            }
        }
    }

    for kv in container_kv {
        if let Some(ignore) = key_to_ignore
            && kv.key.eq_ignore_ascii_case(ignore)
        {
            continue;
        }
        if kv.key.eq_ignore_ascii_case("tagarray") || kv.key.eq_ignore_ascii_case("tag-array") {
            continue;
        }

        if validator
            .array_element
            .as_ref()
            .is_some_and(|ae| matches!(ae, ArrayElementType::Tuple(_)))
            && kv.key.parse::<usize>().is_ok()
        {
            continue;
        }
        let rule = validator
            .rules
            .par_iter()
            .find_first(|r| r.key.eq_ignore_ascii_case(&kv.key))
            .or_else(|| {
                validator
                    .sub_possibilities
                    .par_iter()
                    .find_first(|r| r.key.eq_ignore_ascii_case(&kv.key))
            })
            .or(validator.tag_array.as_ref());

        match rule {
            Some(rule) => {
                if rule.obsolete_tag.unwrap_or(false) {
                    diagnostics.push(Diagnostic {
                        range: kv.key_range,
                        severity: Some(DiagnosticSeverity::WARNING),
                        message: format!(
                            "Key '{}' is obsolete/deprecated in container '{}'",
                            kv.key, validator.container_name
                        ),
                        source: Some(String::from("acs_text-validator")),
                        ..Default::default()
                    });
                }
                if let Some(value) = &kv.value {
                    validate_value(
                        value,
                        rule,
                        all_validators,
                        diagnostics,
                        &validator.container_name,
                        base_path,
                    );
                }
            }
            None => {
                // Ignore metadata keys
                if kv.key.eq_ignore_ascii_case("array-element")
                    || kv.key.eq_ignore_ascii_case("subpossibilities")
                    || kv.key.eq_ignore_ascii_case("validation")
                    || kv.key.eq_ignore_ascii_case("top-level")
                    || kv.key.eq_ignore_ascii_case("tagarray")
                    || kv.key.eq_ignore_ascii_case("tag-array")
                    || kv.key.eq_ignore_ascii_case("kind")
                {
                    continue;
                }

                // Not in rules, check if it's an array-element
                if let Some(array_element_type) = &validator.array_element {
                    if let Some(Value::Container(inner_kv, _, _)) = &kv.value {
                        let element_type = match array_element_type {
                            ArrayElementType::Array(s) => Some(s),
                            ArrayElementType::Tuple(types) => {
                                if let Ok(idx) = kv.key.parse::<usize>() {
                                    types.get(idx)
                                } else {
                                    None
                                }
                            }
                        };

                        if let Some(element_type) = element_type {
                            if let Some(element_validator) = all_validators
                                .containers
                                .par_iter()
                                .find_first(|c| &c.container_name == element_type)
                            {
                                validate_container(
                                    inner_kv,
                                    element_validator,
                                    all_validators,
                                    diagnostics,
                                    None,
                                    base_path,
                                );
                            }
                        } else if matches!(array_element_type, ArrayElementType::Tuple(_)) {
                            // index out of bounds or not a number for tuple
                            diagnostics.push(Diagnostic {
                                range: kv.range,
                                severity: Some(DiagnosticSeverity::ERROR),
                                message: format!(
                                    "Invalid index '{}' for tuple in container '{}'",
                                    kv.key, validator.container_name
                                ),
                                source: Some(String::from("acs_text-validator")),
                                ..Default::default()
                            });
                        }
                    }
                } else if !validator.allow_any_key && !is_tag_array {
                    // Unknown key in container
                    diagnostics.push(Diagnostic {
                        range: kv.range,
                        severity: Some(DiagnosticSeverity::WARNING),
                        message: format!(
                            "Unknown key '{}' in container '{}'",
                            kv.key, validator.container_name
                        ),
                        source: Some(String::from("acs_text-validator")),
                        ..Default::default()
                    });
                }
            }
        }
    }

    // Check compulsory keys in rules
    for rule in &validator.rules {
        if rule.compulsory.is_some_and(|c| c >= 1.0)
            && !rule.disabled.unwrap_or(false)
            && !found_keys.contains(&rule.key.to_lowercase())
        {
            let range = container_kv.first().map(|kv| kv.range).unwrap_or_default();
            diagnostics.push(Diagnostic {
                range,
                severity: Some(DiagnosticSeverity::ERROR),
                message: format!(
                    "Compulsory key '{}' is missing in container '{}'.",
                    rule.key, validator.container_name
                ),
                source: Some(String::from("acs_text-validator")),
                ..Default::default()
            });
        }
    }

    // Check compulsory keys in subpossibilities
    for rule in &validator.sub_possibilities {
        if rule.compulsory.is_some_and(|c| c >= 1.0)
            && !rule.disabled.unwrap_or(false)
            && !found_keys.contains(&rule.key.to_lowercase())
        {
            let range = container_kv.first().map(|kv| kv.range).unwrap_or_default();
            diagnostics.push(Diagnostic {
                range,
                severity: Some(DiagnosticSeverity::ERROR),
                message: format!(
                    "Compulsory key '{}' is missing in container '{}'.",
                    rule.key, validator.container_name
                ),
                source: Some(String::from("acs_text-validator")),
                ..Default::default()
            });
        }
    }

    // Special check for array-element sequential keys
    if let Some(array_element_type) = &validator.array_element {
        let mut indices: Vec<usize> = found_keys
            .par_iter()
            .filter_map(|k| k.parse::<usize>().ok())
            .collect();
        indices.sort_unstable();

        match array_element_type {
            ArrayElementType::Array(_) => {
                for (i, &index) in indices.iter().enumerate() {
                    if i != index {
                        // Find the KV pair for this index to get its range
                        if let Some(kv) = container_kv
                            .par_iter()
                            .find_first(|kv| kv.key == index.to_string())
                        {
                            diagnostics.push(Diagnostic {
                                range: kv.range,
                                severity: Some(DiagnosticSeverity::ERROR),
                                message: format!(
                                    "Non-sequential array index '{}' in container '{}'. Expected '{}'.",
                                    index, validator.container_name, i
                                ),
                                source: Some(String::from("acs_text-validator")),
                                ..Default::default()
                            });
                        }
                        break; // Only report the first gap
                    }
                }
            }
            ArrayElementType::Tuple(types) => {
                for (i, _type_name) in types.iter().enumerate() {
                    if !indices.contains(&i) {
                        let range = container_kv.first().map(|kv| kv.range).unwrap_or_default();
                        diagnostics.push(Diagnostic {
                            range,
                            severity: Some(DiagnosticSeverity::ERROR),
                            message: format!(
                                "Missing tuple element '{}' in container '{}'.",
                                i, validator.container_name
                            ),
                            source: Some(String::from("acs_text-validator")),
                            ..Default::default()
                        });
                    }
                }
                // Check for extraneous keys that are numeric
                for &index in &indices {
                    if index >= types.len()
                        && let Some(kv) = container_kv.iter().find(|kv| kv.key == index.to_string())
                    {
                        diagnostics.push(Diagnostic {
                            range: kv.range,
                            severity: Some(DiagnosticSeverity::ERROR),
                            message: format!(
                                "Index '{}' out of bounds for tuple in container '{}'. Max index is '{}'.",
                                index, validator.container_name, types.len() - 1
                            ),
                            source: Some(String::from("acs_text-validator")),
                            ..Default::default()
                        });
                    }
                }
            }
        }
    }
}
