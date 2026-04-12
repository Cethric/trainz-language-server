use crate::soup::validate_container::validate_container;
use crate::soup::validate_simple_value;
use rayon::prelude::*;
use std::path::Path;
use tower_lsp_server::ls_types::{Diagnostic, DiagnosticSeverity};
use tracing::warn;
use trainz_ast::soup::{NumericValue, Value};
use trainz_soup_validators::{ContainerRule, Validation, Validators};

pub fn validate_value(
    value: &Value,
    rule: &ContainerRule,
    all_validators: &Validators,
    diagnostics: &mut Vec<Diagnostic>,
    container_name: &str,
    base_path: Option<&Path>,
) {
    if let Some(type_name) = &rule.type_name {
        let actual_type = match value {
            Value::String(_, _) => "string",
            Value::Numeric(_, _) => "numeric",
            Value::Array(v, _) => {
                if v.par_iter().any(|n| matches!(n, NumericValue::Float(_))) {
                    "floatlist"
                } else {
                    "array"
                }
            }
            Value::Container(_, _, _) => "container",
            Value::Kuid(_, _) => "kuid",
            Value::Variable(_, _) => "variable",
        };

        let is_numeric = match value {
            Value::Numeric(_, _) => true,
            Value::Array(v, _) if v.len() == 1 => true,
            Value::String(s, _) | Value::Variable(s, _) => {
                s.parse::<f64>().is_ok() || s.starts_with("0x") || s.starts_with("0X")
            }
            _ => false,
        };

        if rule.key == "region" {
            log::error!(
                "DEBUG: Key: {}, type_name: {:?}, value: {:?}",
                rule.key,
                type_name,
                value
            );
        }

        let is_array_type = actual_type == "array" || actual_type == "floatlist";

        let type_mismatch = if type_name == "numeric" {
            !is_numeric
        } else if type_name == "int" || type_name == "float" {
            actual_type != "numeric" && actual_type != "variable"
        } else if type_name == "bool" {
            if actual_type == "numeric" {
                match value {
                    Value::Numeric(NumericValue::Int(n), _) => *n != 0 && *n != 1,
                    Value::Numeric(NumericValue::Float(f), _) => {
                        (*f - 0.0).abs() > 1e-9 && (*f - 1.0).abs() > 1e-9
                    }
                    _ => true,
                }
            } else {
                actual_type != "bool" && actual_type != "variable"
            }
        } else if type_name == "kuid"
            || type_name == "kuidbrowser"
            || type_name == "stringkuidbrowser"
        {
            actual_type != "kuid"
        } else if type_name == "filepath" {
            actual_type != "string" && actual_type != "variable"
        } else if type_name.starts_with("vector") || type_name == "floatlist" {
            !is_array_type && actual_type != "variable"
        } else if type_name == "combobox" || type_name == "listbox" || type_name == "filepathedit" {
            actual_type != "string" && actual_type != "variable"
        } else if type_name == "doublestring" {
            actual_type != "string" && actual_type != "variable" && actual_type != "container"
        } else {
            actual_type != type_name.as_str()
                && actual_type != "variable"
                && !(type_name == "array" && is_array_type)
        };

        if type_mismatch {
            let message = format!(
                "Invalid type for key '{}' in container '{}'. Expected '{}', found '{}', kind '{:?}'",
                rule.key, container_name, type_name, actual_type, rule.kind
            );
            diagnostics.push(Diagnostic {
                range: value.range(),
                severity: Some(DiagnosticSeverity::ERROR),
                message,
                source: Some(String::from("soup-validator")),
                ..Default::default()
            });
        }

        if type_name == "filepathedit"
            && !type_mismatch
            && let Some(base_path) = base_path
            && let Value::String(s, _) = value
        {
            let file_path = if let Some(parent) = base_path.parent() {
                parent.join(s)
            } else {
                Path::new(s).to_path_buf()
            };

            if !file_path.exists() {
                diagnostics.push(Diagnostic {
                    range: value.range(),
                    severity: Some(DiagnosticSeverity::ERROR),
                    message: format!("File '{}' for key '{}' does not exist.", s, rule.key),
                    source: Some(String::from("soup-validator")),
                    ..Default::default()
                });
            }
        }
    }

    if let Some(validations) = &rule.validation {
        for validation in validations {
            if let Validation::Named(name) = validation {
                if name == "IsValidCategoryClass"
                    && let Some(allowed_values) = all_validators.simple.get("category-class")
                    && let Value::String(s, _) = value
                {
                    validate_simple_value::validate_simple_value_str(
                        value.range(),
                        s,
                        "category-class",
                        allowed_values,
                        diagnostics,
                    );
                } else if name == "IsValidCategoryRegion"
                    && let Some(allowed_values) = all_validators.simple.get("category-region")
                    && let Value::String(s, _) = value
                {
                    validate_simple_value::validate_simple_value_str(
                        value.range(),
                        s,
                        "category-region",
                        allowed_values,
                        diagnostics,
                    );
                } else if name == "IsValidCategoryEra"
                    && let Some(allowed_values) = all_validators.simple.get("category-era")
                    && let Value::String(s, _) = value
                {
                    for era in s.split(';') {
                        if era.is_empty() {
                            continue;
                        }
                        if !allowed_values.contains_key(era) {
                            diagnostics.push(Diagnostic {
                                range: value.range(),
                                severity: Some(DiagnosticSeverity::ERROR),
                                message: format!(
                                    "Invalid value(s) '{}' for key 'category-era'. Allowed values are: {}",
                                    era,
                                    allowed_values
                                        .keys()
                                        .map(|k| k.to_string())
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                ),
                                source: Some(String::from("soup-validator")),
                                ..Default::default()
                            });
                        }
                    }
                }
            }
        }
    }

    match value {
        Value::Container(container_kv, _, _) => {
            if let Some(kind_name) = &rule.kind {
                if let Some(validator) = all_validators
                    .containers
                    .par_iter()
                    .find_first(|v| v.container_name.eq_ignore_ascii_case(kind_name))
                {
                    validate_container(
                        container_kv,
                        validator,
                        all_validators,
                        diagnostics,
                        None,
                        base_path,
                    );
                }
            } else if let Some(type_name) = &rule.type_name
                && let Some(validator) = all_validators
                    .containers
                    .par_iter()
                    .find_first(|v| v.container_name.eq_ignore_ascii_case(type_name))
            {
                validate_container(
                    container_kv,
                    validator,
                    all_validators,
                    diagnostics,
                    None,
                    base_path,
                );
            }
        }
        Value::String(s, _) | Value::Variable(s, _) => {
            if let Some(type_name) = &rule.type_name {
                if type_name == "combobox" {
                    if let Some(validator) = all_validators.simple.get(&rule.key) {
                        validate_simple_value::validate_simple_value(
                            value,
                            &rule.key,
                            validator,
                            diagnostics,
                        );
                    }
                } else if type_name == "listbox" {
                    let values: Vec<&str> = s
                        .split(';')
                        .map(|e| e.trim())
                        .filter(|e| !e.is_empty())
                        .collect();
                    if let Some(validator) = all_validators.simple.get(&rule.key) {
                        for entry in values {
                            validate_simple_value::validate_simple_value_str(
                                value.range(),
                                entry,
                                &rule.key,
                                validator,
                                diagnostics,
                            );
                        }
                    }
                } else if let Some(validator) = all_validators.simple.get(type_name) {
                    validate_simple_value::validate_simple_value(
                        value,
                        &rule.key,
                        validator,
                        diagnostics,
                    );
                }
            }
            if let Some(filter) = &rule.filter
                && !s.to_lowercase().contains(&filter.to_lowercase())
            {
                diagnostics.push(Diagnostic {
                    range: value.range(),
                    severity: Some(DiagnosticSeverity::ERROR),
                    message: format!(
                        "Value '{}' for key '{}' does not match filter '{}'.",
                        s, rule.key, filter
                    ),
                    source: Some(String::from("soup-validator")),
                    ..Default::default()
                });
            }
            if let Some(validations) = &rule.validation {
                for validator in validations {
                    match validator {
                        Validation::IntRange(min, max) => {
                            warn!(
                                "Range validator not implemented yet for {} {:?} {} {}",
                                rule.key, value, min, max
                            )
                        }
                        Validation::HexRange(min, max) => {
                            warn!(
                                "Range validator not implemented yet for {} {:?} {} {}",
                                rule.key, value, min, max
                            )
                        }
                        Validation::FloatRange(min, max) => {
                            warn!(
                                "Range validator not implemented yet for {} {:?} {} {}",
                                rule.key, value, min, max
                            )
                        }
                        Validation::NeedCollateMeshes(_meshes) => {
                            // todo!(
                            //     "NeedCollateMeshes validator not implemented yet for {} {:?} {:?}",
                            //     rule.key,
                            //     value,
                            //     meshes
                            // );
                        }
                        Validation::NotOwnParent => {
                            // todo!(
                            //     "NotOwnParent validator not implemented yet for {} {:?}",
                            //     rule.key,
                            //     value
                            // );
                        }
                        Validation::Named(validation) => {
                            if validation.eq_ignore_ascii_case("ScriptFileExists") {
                                if let Some(base_path) = base_path {
                                    let file_path = if let Some(parent) = base_path.parent() {
                                        parent.join(s)
                                    } else {
                                        Path::new(s).to_path_buf()
                                    };

                                    let mut exists = file_path.exists();
                                    if !exists && !s.to_lowercase().ends_with(".gs") {
                                        let gs_path = file_path.with_added_extension("gs");
                                        if gs_path.exists() {
                                            exists = true;
                                        }
                                    }

                                    if !exists {
                                        diagnostics.push(Diagnostic {
                                            range: value.range(),
                                            severity: Some(DiagnosticSeverity::ERROR),
                                            message: format!(
                                                "File '{}' for key '{}' does not exist.",
                                                s, rule.key
                                            ),
                                            source: Some(String::from("soup-validator")),
                                            ..Default::default()
                                        });
                                    }
                                }
                            } else if let Some(validator) =
                                all_validators.simple.get(&validation.to_lowercase())
                            {
                                if !validator.keys().any(|k| k.eq_ignore_ascii_case(s)) {
                                    diagnostics.push(Diagnostic {
                                        range: value.range(),
                                        severity: Some(DiagnosticSeverity::ERROR),
                                        message: format!(
                                            "Value '{}' for key '{}' is not a valid {}.",
                                            s, rule.key, validation
                                        ),
                                        source: Some(String::from("soup-validator")),
                                        ..Default::default()
                                    });
                                }
                            } else {
                                warn!(
                                    "Unknown validation type: {} - {} {:?}",
                                    validation, rule.key, value
                                );
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }
}
