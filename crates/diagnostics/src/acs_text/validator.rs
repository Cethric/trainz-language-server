use crate::acs_text::validate_container::validate_container;
use crate::acs_text::validate_simple_value::validate_simple_value;
use crate::acs_text::validate_value::validate_value;
use std::collections::HashMap;
use std::path::Path;
use tower_lsp_server::ls_types::Diagnostic;
use tracing::{debug, trace};
use trainz_acs_text_validators::Validators;
use trainz_ast::acs_text::base::AcsText;
use trainz_ast::acs_text::key_value_pair::KeyValuePair;
use trainz_ast::acs_text::value::Value;

#[tracing::instrument(skip(acs_text, validators, base_path))]
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
    if let Some(kv) = kind_kv
        && kv.value.is_none()
    {
        diagnostics.push(Diagnostic {
            range: kv.key_range,
            severity: Some(tower_lsp_server::ls_types::DiagnosticSeverity::ERROR),
            message: "Missing value for 'kind'".to_string(),
            source: Some(String::from("acs-validator")),
            ..Default::default()
        });
    }

    let kind_validator_name = kind_kv.as_ref().and_then(|kv| match &kv.value {
        Some(Value::String(kind_name, _)) => Some(kind_name.clone()),
        Some(Value::Variable(kind_name, _)) => Some(kind_name.clone()),
        _ => None,
    });

    let trainz_build_kv = acs_text_map.get("trainz-build").and_then(|v| v.first());
    let trainz_build_version = trainz_build_kv.and_then(|kv| match &kv.value {
        Some(Value::Numeric(trainz_ast::acs_text::NumericValue::Float(f), _)) => Some(*f),
        Some(Value::Numeric(trainz_ast::acs_text::NumericValue::Int(i), _)) => Some(*i as f64),
        Some(Value::String(s, _)) => s.parse::<f64>().ok(),
        _ => None,
    });

    // Evaluate all trainz-build tags if they exist at the top level
    if let Some(t_build_kvs) = acs_text_map.get("trainz-build") {
        for kv in t_build_kvs {
            if let Some(validator) = validators.container_map.get("trainz-build")
                && let Some(value) = &kv.value
            {
                validate_value(
                    value,
                    &validator.rules[0],
                    validators,
                    &mut diagnostics,
                    "top-level",
                    base_path,
                    trainz_build_version,
                );
            }
        }
    }

    // Evaluate all kind tags if they exist at the top level
    if let Some(kind_kvs) = acs_text_map.get("kind") {
        for kv in kind_kvs {
            if let Some(validator) = validators.container_map.get("kind")
                && let Some(value) = &kv.value
            {
                validate_value(
                    value,
                    &validator.rules[0],
                    validators,
                    &mut diagnostics,
                    "top-level",
                    base_path,
                    trainz_build_version,
                );
            }
        }
    }

    let kind_validator = kind_validator_name.as_ref().and_then(|name| {
        trace!("Looking for validator: {}", name);
        let val = validators.container_map.get(&name.to_ascii_lowercase());
        if val.is_none() {
            debug!(
                "Validator not found for: {}, available keys: {:?}",
                name,
                validators.container_map.keys()
            );
            diagnostics.push(tower_lsp_server::ls_types::Diagnostic {
                range: kind_kv.unwrap().range,
                severity: Some(tower_lsp_server::ls_types::DiagnosticSeverity::ERROR),
                message: format!("Unknown kind '{}'", name),
                source: Some(String::from("acs-validator")),
                ..Default::default()
            });
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
        if !validator.top_level {
            debug!(
                "Validator for kind '{}' found but is not top-level",
                validator.container_name
            );
            diagnostics.push(tower_lsp_server::ls_types::Diagnostic {
                range: kind_kv.unwrap().range,
                severity: Some(tower_lsp_server::ls_types::DiagnosticSeverity::ERROR),
                message: format!(
                    "Kind '{}' is not a top-level container",
                    validator.container_name
                ),
                source: Some(String::from("acs-validator")),
                ..Default::default()
            });
            return diagnostics;
        }
        validate_container(
            &acs_text.key_value_pairs,
            validator,
            validators,
            &mut diagnostics,
            None,
            base_path,
            trainz_build_version,
        );
    } else {
        // Fallback or multi-container mode: validate every top-level container that matches a validator
        // This is important for tests and legacy files that don't have a 'kind' tag
        for kv in &acs_text.key_value_pairs {
            let key_lower = kv.key.to_lowercase();

            // Skip 'kind' as it was already handled or reported as missing
            if key_lower == "kind" || key_lower == "trainz-build" {
                continue;
            }

            // Check for simple validator
            if let Some(validation_values) = validators.simple.get(&key_lower)
                && let Some(value) = &kv.value
            {
                validate_simple_value(value, &kv.key, validation_values, &mut diagnostics);
            }

            // Check for container validator
            if let Some(validator) = validators.container_map.get(&key_lower) {
                if let Some(Value::Container(inner_kvs, _, _)) = &kv.value {
                    // Most common case for tests: root container matches a validator
                    validate_container(
                        inner_kvs,
                        validator,
                        validators,
                        &mut diagnostics,
                        Some(&kv.key),
                        base_path,
                        trainz_build_version,
                    );
                }
            }
        }

        // If 'kind' is completely missing and no containers were validated, ONLY THEN report it as invalid
        if kind_kv.is_some() || acs_text.key_value_pairs.is_empty() {
            diagnostics.push(Diagnostic {
                range: acs_text.range,
                severity: Some(tower_lsp_server::ls_types::DiagnosticSeverity::ERROR),
                message: "'kind' is invalid".to_string(),
                source: Some(String::from("acs-validator")),
                ..Default::default()
            });
        }
    }

    diagnostics
}
