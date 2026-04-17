use crate::acs_text::validate_value::validate_value;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::path::Path;
use tower_lsp_server::ls_types::{Diagnostic, DiagnosticSeverity};
use trainz_acs_text_validators::{ArrayElementType, ContainerValidator, Validation, Validators};
use trainz_ast::acs_text::Value;

#[tracing::instrument(skip(
    container_kv,
    found_keys,
    validator,
    diagnostics,
    trainz_build_version
))]
fn validate_compulsory_keys(
    container_kv: &[trainz_ast::acs_text::KeyValuePair],
    found_keys: &std::collections::HashSet<String>,
    validator: &ContainerValidator,
    diagnostics: &mut Vec<Diagnostic>,
    trainz_build_version: Option<f64>,
) {
    // Check compulsory keys in rules
    for rule in &validator.rules {
        let is_compulsory = match rule.compulsory {
            Some(c) if c >= 1.0 => {
                if c > 1.0 {
                    trainz_build_version.is_some_and(|v| v >= c)
                } else {
                    true
                }
            }
            _ => false,
        };

        if is_compulsory
            && !rule.disabled.unwrap_or(false)
            && !found_keys.contains(&rule.key.to_lowercase())
            && rule.default_value.is_none()
        {
            let range = container_kv.first().map(|kv| kv.range).unwrap_or_default();
            diagnostics.push(Diagnostic {
                range,
                severity: Some(DiagnosticSeverity::ERROR),
                message: format!(
                    "Compulsory key '{}' is missing in container '{}'.",
                    rule.key, validator.container_name
                ),
                source: Some(String::from("acs-validator")),
                ..Default::default()
            });
        }
    }

    // Check compulsory keys in subpossibilities
    for rule in &validator.sub_possibilities {
        let is_compulsory = match rule.compulsory {
            Some(c) if c >= 1.0 => {
                if c > 1.0 {
                    trainz_build_version.is_some_and(|v| v >= c)
                } else {
                    true
                }
            }
            _ => false,
        };

        if is_compulsory
            && !rule.disabled.unwrap_or(false)
            && !found_keys.contains(&rule.key.to_lowercase())
            && rule.default_value.is_none()
        {
            let range = container_kv.first().map(|kv| kv.range).unwrap_or_default();
            diagnostics.push(Diagnostic {
                range,
                severity: Some(DiagnosticSeverity::ERROR),
                message: format!(
                    "Compulsory key '{}' is missing in container '{}'.",
                    rule.key, validator.container_name
                ),
                source: Some(String::from("acs-validator")),
                ..Default::default()
            });
        }
    }
}

#[tracing::instrument(skip(
    kv,
    index,
    array_type,
    all_validators,
    diagnostics,
    container_name,
    base_path,
    trainz_build_version
))]
fn validate_element(
    kv: &trainz_ast::acs_text::KeyValuePair,
    index: usize,
    array_type: &ArrayElementType,
    all_validators: &Validators,
    diagnostics: &mut Vec<Diagnostic>,
    container_name: &str,
    base_path: Option<&Path>,
    trainz_build_version: Option<f64>,
) {
    match array_type {
        ArrayElementType::Array(_, name) => {
            let rule = trainz_acs_text_validators::ContainerRule {
                key: kv.key.clone(),
                type_name: Some(name.clone()),
                ..Default::default()
            };
            if let Some(value) = &kv.value {
                validate_value(
                    value,
                    &rule,
                    all_validators,
                    diagnostics,
                    container_name,
                    base_path,
                    trainz_build_version,
                );
            }
        }
        ArrayElementType::Tuple(types) => {
            if let Some((_, name)) = types.get(index) {
                let rule = trainz_acs_text_validators::ContainerRule {
                    key: kv.key.clone(),
                    type_name: Some(name.clone()),
                    ..Default::default()
                };
                if let Some(value) = &kv.value {
                    validate_value(
                        value,
                        &rule,
                        all_validators,
                        diagnostics,
                        container_name,
                        base_path,
                        trainz_build_version,
                    );
                }
            } else {
                diagnostics.push(Diagnostic {
                    range: kv.range,
                    severity: Some(DiagnosticSeverity::ERROR),
                    message: format!(
                        "Index '{}' out of bounds for tuple in container '{}'. Max index is '{}'.",
                        index,
                        container_name,
                        types.len() - 1
                    ),
                    source: Some(String::from("acs-validator")),
                    ..Default::default()
                });
            }
        }
        ArrayElementType::Inline(validator) => {
            if let Some(Value::Container(inner_kv, _, _)) = &kv.value {
                validate_container(
                    inner_kv,
                    validator,
                    all_validators,
                    diagnostics,
                    None,
                    base_path,
                    trainz_build_version,
                );
            }
        }
        ArrayElementType::Rule(rule) => {
            if let Some(value) = &kv.value {
                validate_value(
                    value,
                    rule,
                    all_validators,
                    diagnostics,
                    container_name,
                    base_path,
                    trainz_build_version,
                );
            }
        }
    }
}

#[tracing::instrument(skip(
    container_kv,
    validator,
    all_validators,
    diagnostics,
    key_to_ignore,
    base_path,
    trainz_build_version
))]
pub fn validate_container(
    container_kv: &[trainz_ast::acs_text::KeyValuePair],
    validator: &ContainerValidator,
    all_validators: &Validators,
    diagnostics: &mut Vec<Diagnostic>,
    key_to_ignore: Option<&str>,
    base_path: Option<&Path>,
    trainz_build_version: Option<f64>,
) {
    let mut found_keys = std::collections::HashSet::new();
    let unique_names = validator.validation.as_ref().is_some_and(|v| {
        v.par_iter().any(|val| {
            matches!(val, Validation::Named(name) if name.eq_ignore_ascii_case("UniqueNames") || name.eq_ignore_ascii_case("SubPossibilities"))
        })
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
                source: Some(String::from("acs-validator")),
                ..Default::default()
            });
        } else {
            found_keys.insert(kv.key.to_lowercase());
        }
    }

    validate_compulsory_keys(
        container_kv,
        &found_keys,
        validator,
        diagnostics,
        trainz_build_version,
    );

    let is_tag_array = validator.tag_array.is_some()
        || validator.array_element.is_some()
        || validator.validation.as_ref().is_some_and(|v| {
            v.par_iter().any(|val| {
                matches!(val, Validation::Named(name) if name.eq_ignore_ascii_case("tagarray") || name.eq_ignore_ascii_case("tag-array") || name.eq_ignore_ascii_case("FilepathTableFilesExist"))
            })
        });

    // Special validation for array-element
    if let Some(array_element) = &validator.array_element {
        let mut kv_indices = Vec::new();
        for kv in container_kv {
            if let Ok(idx) = kv.key.parse::<usize>() {
                kv_indices.push((idx, kv));
            }
        }

        // Check for sequentiality and increasing order
        let is_tuple = matches!(array_element, ArrayElementType::Tuple(_));
        let mut last_idx: Option<usize> = None;

        for (i, (idx, kv)) in kv_indices.iter().enumerate() {
            if let Some(last) = last_idx
                && *idx < last
            {
                diagnostics.push(Diagnostic {
                        range: kv.key_range,
                        severity: Some(DiagnosticSeverity::WARNING),
                        message: format!(
                            "Non-increasing array index '{}' in container '{}'. Indices should appear in increasing order.",
                            kv.key, validator.container_name
                        ),
                        source: Some(String::from("acs-validator")),
                        ..Default::default()
                    });
            }

            if is_tuple && i != *idx {
                diagnostics.push(Diagnostic {
                    range: kv.key_range,
                    severity: Some(DiagnosticSeverity::WARNING),
                    message: format!(
                        "Non-sequential array index '{}' in container '{}'. Expected '{}'.",
                        kv.key, validator.container_name, i
                    ),
                    source: Some(String::from("acs-validator")),
                    ..Default::default()
                });
            }
            last_idx = Some(*idx);

            validate_element(
                kv,
                *idx,
                array_element,
                all_validators,
                diagnostics,
                &validator.container_name,
                base_path,
                trainz_build_version,
            );
        }

        if let ArrayElementType::Tuple(types) = array_element {
            // Check for missing elements in tuple
            for (i, (key, _)) in types.iter().enumerate() {
                if !kv_indices.iter().any(|(idx, _)| *idx == i) {
                    let range = container_kv.first().map(|kv| kv.range).unwrap_or_default();
                    diagnostics.push(Diagnostic {
                        range,
                        severity: Some(DiagnosticSeverity::ERROR),
                        message: format!(
                            "Missing tuple element '{}' ({}) in container '{}'.",
                            i, key, validator.container_name
                        ),
                        source: Some(String::from("acs-validator")),
                        ..Default::default()
                    });
                }
            }
        }
    }

    let mut tag_array_index = 0;
    for kv in container_kv {
        if let Some(ignore) = key_to_ignore
            && kv.key.eq_ignore_ascii_case(ignore)
        {
            continue;
        }
        if is_metadata_key(&kv.key) {
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
            });

        let _is_tag_array = validator.tag_array.is_some()
                || validator.validation.as_ref().is_some_and(|v| {
                    v.par_iter().any(|val| {
                        matches!(val, Validation::Named(name) if name.eq_ignore_ascii_case("tagarray") || name.eq_ignore_ascii_case("tag-array") || name.eq_ignore_ascii_case("FilepathTableFilesExist"))
                    })
                });

        match rule {
            Some(rule) => {
                // ...
                if rule.obsolete_tag.unwrap_or(false) {
                    diagnostics.push(Diagnostic {
                        range: kv.key_range,
                        severity: Some(DiagnosticSeverity::WARNING),
                        message: format!(
                            "Key '{}' is obsolete/deprecated in container '{}'",
                            kv.key, validator.container_name
                        ),
                        source: Some(String::from("acs-validator")),
                        ..Default::default()
                    });
                } else if let Some(obsolete_version) = rule.obsolete_version
                    && trainz_build_version.is_some_and(|v| v >= obsolete_version)
                {
                    let message = rule.obsolete_message.clone().unwrap_or_else(|| {
                        format!(
                            "Key '{}' is obsolete since Trainz build {} in container '{}'",
                            kv.key, obsolete_version, validator.container_name
                        )
                    });
                    diagnostics.push(Diagnostic {
                        range: kv.key_range,
                        severity: Some(DiagnosticSeverity::WARNING),
                        message,
                        source: Some(String::from("acs-validator")),
                        ..Default::default()
                    });
                }

                if let Some(minimum_version) = rule.minimum_version
                    && trainz_build_version.is_some_and(|v| v < minimum_version)
                {
                    diagnostics.push(Diagnostic {
                        range: kv.key_range,
                        severity: Some(DiagnosticSeverity::ERROR),
                        message: format!(
                            "Key '{}' requires minimum Trainz build {} in container '{}'",
                            kv.key, minimum_version, validator.container_name
                        ),
                        source: Some(String::from("acs-validator")),
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
                        trainz_build_version,
                    );
                }
            }
            None => {
                // If not matched by a rule, try tag-array
                if let Some(tag_array) = &validator.tag_array {
                    validate_element(
                        kv,
                        tag_array_index,
                        tag_array,
                        all_validators,
                        diagnostics,
                        &validator.container_name,
                        base_path,
                        trainz_build_version,
                    );
                    tag_array_index += 1;
                } else if !validator.allow_any_key && !is_tag_array {
                    // Unknown key in container
                    diagnostics.push(Diagnostic {
                        range: kv.range,
                        severity: Some(DiagnosticSeverity::WARNING),
                        message: format!(
                            "Unknown key '{}' in container '{}'",
                            kv.key, validator.container_name
                        ),
                        source: Some(String::from("acs-validator")),
                        ..Default::default()
                    });
                }
            }
        }
    }
}

fn is_metadata_key(key: &str) -> bool {
    key.eq_ignore_ascii_case("array-element")
        || key.eq_ignore_ascii_case("subpossibilities")
        || key.eq_ignore_ascii_case("validation")
        || key.eq_ignore_ascii_case("top-level")
        || key.eq_ignore_ascii_case("tagarray")
        || key.eq_ignore_ascii_case("tag-array")
        || key.eq_ignore_ascii_case("kind")
        || key.eq_ignore_ascii_case("trainz-build")
        || key.eq_ignore_ascii_case("inherit")
}
