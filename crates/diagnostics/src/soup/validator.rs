use crate::soup::validate_simple_value::validate_simple_value;
use crate::soup::{validate_container, validate_value};
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::Path;
use tower_lsp_server::ls_types::{Diagnostic, DiagnosticSeverity};
use tracing::{debug, trace};
use trainz_ast::soup::base::Soup;
use trainz_ast::soup::key_value_pair::KeyValuePair;
use trainz_ast::soup::value::Value;
use trainz_ast::{Position, Range};
use trainz_soup_validators::Validators;

pub fn soup_diagnostics(
    soup: &Soup,
    validators: &Validators,
    base_path: Option<&Path>,
) -> Vec<Diagnostic> {
    let mut diagnostics = vec![];

    let mut soup_map: HashMap<String, Vec<&KeyValuePair>> = HashMap::new();
    for kv in &soup.key_value_pairs {
        soup_map
            .entry(kv.key.to_ascii_lowercase())
            .or_default()
            .push(kv);
    }

    let kind_kv = soup_map.get("kind").and_then(|v| v.first());
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
            // Validate all subpossibilities at the top level
            for sub in &validator.sub_possibilities {
                let soup_kv = soup_map
                    .get(&sub.key.to_ascii_lowercase())
                    .and_then(|v| v.first())
                    .copied();
                if let Some(kv) = soup_kv {
                    if sub.obsolete_tag.unwrap_or(false) {
                        diagnostics.push(Diagnostic {
                            range: kv.key_range,
                            severity: Some(DiagnosticSeverity::WARNING),
                            message: format!(
                                "Key '{}' is obsolete/deprecated in container '{}'",
                                kv.key, validator.container_name
                            ),
                            source: Some(String::from("soup-validator")),
                            ..Default::default()
                        });
                    }
                    if let Some(value) = &kv.value {
                        validate_value::validate_value(
                            value,
                            sub,
                            validators,
                            &mut diagnostics,
                            &validator.container_name,
                            base_path,
                        );
                    }
                } else if sub.compulsory.is_some_and(|c| c >= 1.0) && !sub.disabled.unwrap_or(false)
                {
                    diagnostics.push(Diagnostic {
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
                        source: Some(String::from("soup-validator")),
                        ..Default::default()
                    });
                }
            }

            // Also validate rules if any (in addition to subpossibilities)
            for rule in &validator.rules {
                let soup_kv = soup_map
                    .get(&rule.key.to_ascii_lowercase())
                    .and_then(|v| v.first())
                    .copied();
                if let Some(kv) = soup_kv {
                    if rule.obsolete_tag.unwrap_or(false) {
                        diagnostics.push(Diagnostic {
                            range: kv.key_range,
                            severity: Some(DiagnosticSeverity::WARNING),
                            message: format!(
                                "Key '{}' is obsolete/deprecated in container '{}'",
                                kv.key, validator.container_name
                            ),
                            source: Some(String::from("soup-validator")),
                            ..Default::default()
                        });
                    }
                    if let Some(value) = &kv.value {
                        validate_value::validate_value(
                            value,
                            rule,
                            validators,
                            &mut diagnostics,
                            &validator.container_name,
                            base_path,
                        );
                    }
                } else if rule.compulsory.is_some_and(|c| c >= 1.0)
                    && !rule.disabled.unwrap_or(false)
                {
                    diagnostics.push(Diagnostic {
                        range: soup.range,
                        severity: Some(DiagnosticSeverity::ERROR),
                        message: format!("Compulsory key '{}' is missing.", rule.key),
                        source: Some(String::from("soup-validator")),
                        ..Default::default()
                    });
                }
            }

            // Check for unknown top-level keys
            for kv in &soup.key_value_pairs {
                if kv.key.eq_ignore_ascii_case("kind") {
                    continue;
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
                        continue;
                    }

                    diagnostics.push(Diagnostic {
                        range: kv.range,
                        severity: Some(DiagnosticSeverity::WARNING),
                        message: format!(
                            "Unknown top-level key '{}' for kind '{}'",
                            kv.key, validator.container_name
                        ),
                        source: Some(String::from("soup-validator")),
                        ..Default::default()
                    });
                }
            }
        } else {
            validate_container::validate_container(
                &soup.key_value_pairs,
                validator,
                validators,
                &mut diagnostics,
                Some("kind"),
                base_path,
            );
        }
    }

    // Simple validators
    for soup_kv in &soup.key_value_pairs {
        if soup_kv.key.eq_ignore_ascii_case("kind") {
            continue;
        }
        if let Some(validation_values) = validators.simple.get(&soup_kv.key)
            && let Some(value) = &soup_kv.value
        {
            validate_simple_value(value, &soup_kv.key, validation_values, &mut diagnostics);
        }
    }

    // Container validators
    for validator in &validators.containers {
        if let Some(name) = &kind_validator_name
            && validator.container_name.eq_ignore_ascii_case(name)
        {
            continue;
        }
        for soup_kv in &soup.key_value_pairs {
            if soup_kv.key.eq_ignore_ascii_case(&validator.container_name) {
                if soup_kv.key.eq_ignore_ascii_case("kind") {
                    continue;
                }
                if let Some(Value::Container(container_kv, _, _)) = &soup_kv.value {
                    validate_container::validate_container(
                        container_kv,
                        validator,
                        validators,
                        &mut diagnostics,
                        None,
                        base_path,
                    );
                }
            }
        }
    }

    // Final sanity check for unknown top-level keys if no kind-based validation was done
    if kind_validator.is_none() {
        // ... (we could add logic here for top-level keys when no kind is present)
    }

    diagnostics
}
