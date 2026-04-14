use crate::acs_text::validate_simple_value::validate_simple_value;
use crate::acs_text::{validate_container, validate_value};
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::Path;
use tower_lsp_server::ls_types::{Diagnostic, DiagnosticSeverity};
use tracing::{debug, trace};
use trainz_acs_text_validators::Validators;
use trainz_ast::acs_text::base::AcsText;
use trainz_ast::acs_text::key_value_pair::KeyValuePair;
use trainz_ast::acs_text::value::Value;
use trainz_ast::{Position, Range};

#[tracing::instrument]
pub fn acs_text_diagnostics(
    acs_text: &AcsText,
    validators: &Validators,
    base_path: Option<&Path>,
) -> Vec<Diagnostic> {
    let mut diagnostics = vec![];

    let mut acs_text_map: HashMap<String, Vec<&KeyValuePair>> = HashMap::new();
    for kv in &acs_text.key_value_pairs {
        acs_text_map
            .entry(kv.key.to_ascii_lowercase())
            .or_default()
            .push(kv);
    }

    let kind_kv = acs_text_map.get("kind").and_then(|v| v.first());
    let kind_validator_name = kind_kv.as_ref().and_then(|kv| match &kv.value {
        Some(Value::String(kind_name, _)) => Some(kind_name.clone()),
        Some(Value::Variable(kind_name, _)) => Some(kind_name.clone()),
        _ => None,
    });

    let kind_validator = kind_validator_name.as_ref().and_then(|name| {
        trace!("Looking for validator: {}", name);
        let val = validators.container_map.get(&name.to_ascii_lowercase());
        if val.is_none() {
            debug!(
                "Validator not found for: {}, available keys: {:?}",
                name,
                validators.container_map.keys()
            );
        } else {
            trace!("Validator found: {}", name);
        }
        val
    });

    if let Some(validator) = kind_validator {
        trace!(
            "Validator found: {}, top_level: {}",
            validator.container_name, validator.top_level
        );
        if validator.top_level {
            let sub_diagnostics: Vec<Diagnostic> = validator
                .sub_possibilities
                .par_iter()
                .flat_map(|sub| {
                    let mut local_diagnostics = Vec::new();
                    let acs_text_kv = acs_text_map
                        .get(&sub.key.to_ascii_lowercase())
                        .and_then(|v| v.first())
                        .copied();
                    if let Some(kv) = acs_text_kv {
                        if sub.obsolete_tag.unwrap_or(false) {
                            local_diagnostics.push(Diagnostic {
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
                            validate_value::validate_value(
                                value,
                                sub,
                                validators,
                                &mut local_diagnostics,
                                &validator.container_name,
                                base_path,
                            );
                        }
                    } else if sub.compulsory.is_some_and(|c| c >= 1.0)
                        && !sub.disabled.unwrap_or(false)
                    {
                        local_diagnostics.push(Diagnostic {
                            range: Range {
                                start: Position {
                                    line: 0,
                                    character: 0,
                                },
                                end: Position {
                                    line: 0,
                                    character: 1,
                                },
                            },
                            severity: Some(DiagnosticSeverity::ERROR),
                            message: format!("Compulsory key '{}' is missing.", sub.key),
                            source: Some(String::from("acs_text-validator")),
                            ..Default::default()
                        });
                    }
                    local_diagnostics
                })
                .collect();
            diagnostics.extend(sub_diagnostics);

            // Also validate rules if any (in addition to subpossibilities)
            let rule_diagnostics: Vec<Diagnostic> = validator
                .rules
                .par_iter()
                .flat_map(|rule| {
                    let mut local_diagnostics = Vec::new();
                    let acs_text_kv = acs_text_map
                        .get(&rule.key.to_ascii_lowercase())
                        .and_then(|v| v.first())
                        .copied();
                    if let Some(kv) = acs_text_kv {
                        if rule.obsolete_tag.unwrap_or(false) {
                            local_diagnostics.push(Diagnostic {
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
                            validate_value::validate_value(
                                value,
                                rule,
                                validators,
                                &mut local_diagnostics,
                                &validator.container_name,
                                base_path,
                            );
                        }
                    } else if rule.compulsory.is_some_and(|c| c >= 1.0)
                        && !rule.disabled.unwrap_or(false)
                    {
                        local_diagnostics.push(Diagnostic {
                            range: acs_text.range,
                            severity: Some(DiagnosticSeverity::ERROR),
                            message: format!("Compulsory key '{}' is missing.", rule.key),
                            source: Some(String::from("acs_text-validator")),
                            ..Default::default()
                        });
                    }
                    local_diagnostics
                })
                .collect();
            diagnostics.extend(rule_diagnostics);

            let unknown_key_diagnostics: Vec<Diagnostic> = acs_text
                .key_value_pairs
                .par_iter()
                .filter_map(|kv| {
                    let mut local_diagnostics = Vec::new();
                    if kv.key.eq_ignore_ascii_case("kind") {
                        return None;
                    }
                    if !validator
                        .sub_possibilities
                        .par_iter()
                        .any(|sub| sub.key.eq_ignore_ascii_case(&kv.key))
                        && !validator
                            .rules
                            .par_iter()
                            .any(|rule| rule.key.eq_ignore_ascii_case(&kv.key))
                    {
                        // Ignore metadata keys
                        if kv.key.eq_ignore_ascii_case("array-element")
                            || kv.key.eq_ignore_ascii_case("subpossibilities")
                            || kv.key.eq_ignore_ascii_case("validation")
                            || kv.key.eq_ignore_ascii_case("top-level")
                            || kv.key.eq_ignore_ascii_case("tagarray")
                            || kv.key.eq_ignore_ascii_case("tag-array")
                        {
                            return None;
                        }

                        local_diagnostics.push(Diagnostic {
                            range: kv.range,
                            severity: Some(DiagnosticSeverity::WARNING),
                            message: format!(
                                "Unknown top-level key '{}' for kind '{}'",
                                kv.key, validator.container_name
                            ),
                            source: Some(String::from("acs_text-validator")),
                            ..Default::default()
                        });
                    }
                    if local_diagnostics.is_empty() {
                        None
                    } else {
                        Some(local_diagnostics)
                    }
                })
                .flatten()
                .collect();
            diagnostics.extend(unknown_key_diagnostics);
        } else {
            validate_container::validate_container(
                &acs_text.key_value_pairs,
                validator,
                validators,
                &mut diagnostics,
                Some("kind"),
                base_path,
            );
        }
    }

    // Simple validators
    let simple_diagnostics: Vec<Diagnostic> = acs_text
        .key_value_pairs
        .par_iter()
        .filter_map(|acs_text_kv| {
            let mut local_diagnostics = Vec::new();
            if acs_text_kv.key.eq_ignore_ascii_case("kind") {
                return None;
            }
            if let Some(validation_values) = validators.simple.get(&acs_text_kv.key)
                && let Some(value) = &acs_text_kv.value
            {
                validate_simple_value(
                    value,
                    &acs_text_kv.key,
                    validation_values,
                    &mut local_diagnostics,
                );
            }
            if local_diagnostics.is_empty() {
                None
            } else {
                Some(local_diagnostics)
            }
        })
        .flatten()
        .collect();
    diagnostics.extend(simple_diagnostics);

    // Container validators
    let container_diagnostics: Vec<Diagnostic> = validators
        .containers
        .par_iter()
        .flat_map(|validator| {
            let mut local_diagnostics = Vec::new();
            if let Some(name) = &kind_validator_name
                && validator.container_name.eq_ignore_ascii_case(name)
            {
                return local_diagnostics;
            }
            for acs_text_kv in &acs_text.key_value_pairs {
                if acs_text_kv
                    .key
                    .eq_ignore_ascii_case(&validator.container_name)
                {
                    if acs_text_kv.key.eq_ignore_ascii_case("kind") {
                        continue;
                    }
                    if let Some(Value::Container(container_kv, _, _)) = &acs_text_kv.value {
                        validate_container::validate_container(
                            container_kv,
                            validator,
                            validators,
                            &mut local_diagnostics,
                            None,
                            base_path,
                        );
                    }
                }
            }
            local_diagnostics
        })
        .collect();
    diagnostics.extend(container_diagnostics);

    // Final sanity check for unknown top-level keys if no kind-based validation was done
    if kind_validator.is_none() {
        // ... (we could add logic here for top-level keys when no kind is present)
    }

    diagnostics
}
