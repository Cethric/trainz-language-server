use crate::acs_text::validate_container::validate_container;
use crate::acs_text::validate_simple_value::{validate_simple_value, validate_simple_value_str};
use rayon::prelude::*;
use std::path::Path;
use tower_lsp_server::ls_types::{Diagnostic, DiagnosticSeverity};
use tracing::warn;
use trainz_acs_text_validators::{ContainerRule, Validation, Validators};
use trainz_ast::acs_text::{NumericValue, Value};

#[tracing::instrument(skip(
    value,
    rule,
    all_validators,
    diagnostics,
    container_name,
    base_path,
    trainz_build_version
))]
pub fn validate_value(
    value: &Value,
    rule: &ContainerRule,
    all_validators: &Validators,
    diagnostics: &mut Vec<Diagnostic>,
    container_name: &str,
    base_path: Option<&Path>,
    trainz_build_version: Option<f64>,
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
            } else if actual_type == "string" {
                match value {
                    Value::String(s, _) => s != "0" && s != "1",
                    _ => true,
                }
            } else {
                actual_type != "bool" && actual_type != "variable"
            }
        } else if type_name == "rgb" {
            if actual_type == "array" {
                if let Value::Array(parts, _) = value {
                    if parts.len() != 3 {
                        true
                    } else {
                        parts.iter().any(|p| {
                            if let NumericValue::Int(i) = p {
                                *i > 256i64 || *i < 0
                            } else {
                                true
                            }
                        })
                    }
                } else {
                    true
                }
            } else {
                true
            }
        } else if type_name == "kuid"
            || type_name == "kuidbrowser"
            || type_name == "stringkuidbrowser"
        {
            actual_type != "kuid"
        } else if type_name == "filepath" {
            actual_type != "string" && actual_type != "variable"
        } else if type_name.starts_with("vector") || type_name == "floatlist" {
            if !is_array_type && actual_type != "variable" {
                true
            } else if let Value::Array(v, _) = value {
                if type_name == "vector2" && v.len() != 2 {
                    true
                } else if type_name == "vector3" && v.len() != 2 {
                    // Issue update says: "A vector3 is the same as a vector2"
                    true
                } else {
                    type_name == "floatlist" && v.len() < 2
                }
            } else {
                false
            }
        } else if type_name == "combobox" || type_name == "listbox" || type_name == "filepathedit" {
            actual_type != "string" && actual_type != "variable"
        } else if type_name == "doublestring" {
            actual_type != "string" && actual_type != "variable" && actual_type != "container"
        } else {
            actual_type != type_name.as_str()
                && actual_type != "variable"
                && !(type_name == "array" && is_array_type)
                && !(actual_type == "container"
                    && all_validators
                        .container_map
                        .contains_key(&type_name.to_lowercase()))
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
                source: Some(String::from("acs-validator")),
                ..Default::default()
            });
        }

        if (type_name == "filepathedit" || type_name == "filepath")
            && !type_mismatch
            && let Some(base_path) = base_path
            && let Value::String(s, _) = value
        {
            let mut check_paths = vec![s.clone()];

            if s.to_lowercase().ends_with(".trainzmesh") {
                let stem = &s[..s.len() - 11];
                check_paths.push(format!("{}.fbx", stem));
            }

            if rule.kind.as_deref() == Some("texture") {
                if s.to_lowercase().ends_with(".texture") {
                    check_paths.push(format!("{}.txt", s));
                }
            } else if rule.kind.as_deref() == Some("image") {
                let lower = s.to_lowercase();
                if !lower.ends_with(".png")
                    && !lower.ends_with(".jpg")
                    && !lower.ends_with(".jpeg")
                    && !lower.ends_with(".tga")
                {
                    diagnostics.push(Diagnostic {
                        range: value.range(),
                        severity: Some(DiagnosticSeverity::ERROR),
                        message: format!(
                            "File '{}' for key '{}' must be png, jpg, jpeg or tga.",
                            s, rule.key
                        ),
                        source: Some(String::from("acs-validator")),
                        ..Default::default()
                    });
                }
            } else if rule.kind.as_deref() == Some("html") {
                if !s.to_lowercase().ends_with(".html") {
                    diagnostics.push(Diagnostic {
                        range: value.range(),
                        severity: Some(DiagnosticSeverity::ERROR),
                        message: format!(
                            "File '{}' for key '{}' must be a html file.",
                            s, rule.key
                        ),
                        source: Some(String::from("acs-validator")),
                        ..Default::default()
                    });
                }
            } else if rule.kind.as_deref() == Some("animation") {
                // assume extension check if we knew it, for now just existence
            }

            let mut exists = false;
            for p in &check_paths {
                let file_path = if let Some(parent) = base_path.parent() {
                    parent.join(p)
                } else {
                    Path::new(p).to_path_buf()
                };
                if file_path.exists() {
                    exists = true;
                    break;
                }
            }

            if !exists {
                diagnostics.push(Diagnostic {
                    range: value.range(),
                    severity: Some(DiagnosticSeverity::ERROR),
                    message: format!("File '{}' for key '{}' does not exist.", s, rule.key),
                    source: Some(String::from("acs-validator")),
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
                    validate_simple_value_str(
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
                    validate_simple_value_str(
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
                    let values: Vec<&str> = s
                        .split(';')
                        .map(|e| e.trim())
                        .filter(|e| !e.is_empty())
                        .collect();
                    for entry in values {
                        validate_simple_value_str(
                            value.range(),
                            entry,
                            "category-era",
                            allowed_values,
                            diagnostics,
                        );
                    }
                }
            }
        }
    }

    match value {
        Value::Container(container_kv, _, _) => {
            if let Some(validator) = &rule.child_validator {
                validate_container(
                    container_kv,
                    validator,
                    all_validators,
                    diagnostics,
                    None,
                    base_path,
                    trainz_build_version,
                );
            } else if let Some(kind_name) = &rule.kind {
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
                        trainz_build_version,
                    );
                }
            } else if let Some(type_name) = &rule.type_name
                && let Some(validator) = all_validators.container_map.get(&type_name.to_lowercase())
            {
                validate_container(
                    container_kv,
                    validator,
                    all_validators,
                    diagnostics,
                    None,
                    base_path,
                    trainz_build_version,
                );
            }
        }
        Value::String(s, _) | Value::Variable(s, _) => {
            if let Some(type_name) = &rule.type_name {
                let validator_name = rule.source.as_ref().unwrap_or(type_name);
                if type_name == "combobox" {
                    if let Some(validator) = all_validators.simple.get(validator_name) {
                        validate_simple_value(
                            value,
                            &format!("{} {}", rule.key, type_name),
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
                    if let Some(validator) = all_validators.simple.get(validator_name) {
                        for entry in values {
                            validate_simple_value_str(
                                value.range(),
                                entry,
                                &format!("{} {}", rule.key, type_name),
                                validator,
                                diagnostics,
                            );
                        }
                    }
                } else if let Some(validator) = all_validators.simple.get(validator_name) {
                    validate_simple_value(
                        value,
                        &format!("{} {}", rule.key, type_name),
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
                    source: Some(String::from("acs-validator")),
                    ..Default::default()
                });
            }
            if let Some(validations) = &rule.validation {
                for validator in validations {
                    match validator {
                        Validation::IntRange(min, max) => {
                            let val = match value {
                                Value::Numeric(NumericValue::Int(i), _) => *i as f64,
                                Value::Numeric(NumericValue::Float(f), _) => *f,
                                Value::String(s, _) => s.parse::<f64>().unwrap_or(0.0),
                                _ => 0.0,
                            };
                            if val < *min as f64 || val > *max as f64 {
                                diagnostics.push(Diagnostic {
                                    range: value.range(),
                                    severity: Some(DiagnosticSeverity::ERROR),
                                    message: format!(
                                        "Value {} for key '{}' is out of range [{}, {}].",
                                        val, rule.key, min, max
                                    ),
                                    source: Some(String::from("acs-validator")),
                                    ..Default::default()
                                });
                            }
                        }
                        Validation::HexRange(min, max) => {
                            let val = match value {
                                Value::Numeric(NumericValue::Int(i), _) => *i as f64,
                                Value::Numeric(NumericValue::Float(f), _) => *f,
                                Value::String(s, _) => s.parse::<f64>().unwrap_or(0.0),
                                _ => 0.0,
                            };
                            if val < *min as f64 || val > *max as f64 {
                                diagnostics.push(Diagnostic {
                                    range: value.range(),
                                    severity: Some(DiagnosticSeverity::ERROR),
                                    message: format!(
                                        "Value {} for key '{}' is out of range [{}, {}].",
                                        val, rule.key, min, max
                                    ),
                                    source: Some(String::from("acs-validator")),
                                    ..Default::default()
                                });
                            }
                        }
                        Validation::FloatRange(min, max) => {
                            let val = match value {
                                Value::Numeric(NumericValue::Int(i), _) => *i as f64,
                                Value::Numeric(NumericValue::Float(f), _) => *f,
                                Value::String(s, _) => s.parse::<f64>().unwrap_or(0.0),
                                _ => 0.0,
                            };
                            if val < *min || val > *max {
                                diagnostics.push(Diagnostic {
                                    range: value.range(),
                                    severity: Some(DiagnosticSeverity::ERROR),
                                    message: format!(
                                        "Value {} for key '{}' is out of range [{}, {}].",
                                        val, rule.key, min, max
                                    ),
                                    source: Some(String::from("acs-validator")),
                                    ..Default::default()
                                });
                            }
                        }
                        Validation::MustBePaired(keys) => {
                            if let Value::Container(entries, _, _) = value {
                                for (i, kv) in entries.iter().enumerate() {
                                    let expected = &keys[i % keys.len()];
                                    if !kv.key.eq_ignore_ascii_case(expected) {
                                        diagnostics.push(Diagnostic {
                                            range: value.range(),
                                            severity: Some(DiagnosticSeverity::ERROR),
                                            message: format!(
                                                "Key '{}' at index {} violates MustBePaired group. Expected '{}'.",
                                                kv.key, i, expected
                                            ),
                                            source: Some(String::from("acs-validator")),
                                            ..Default::default()
                                        });
                                    }
                                }
                            }
                        }
                        Validation::FilepathTableFilesExist => {
                            if let Value::Container(entries, _, _) = value {
                                for kv in entries {
                                    if let Some(Value::String(s, range)) = &kv.value
                                        && let Some(base) = base_path
                                    {
                                        let mut check_paths = vec![s.clone()];

                                        if s.to_lowercase().ends_with(".trainzmesh") {
                                            let stem = &s[..s.len() - 11];
                                            check_paths.push(format!("{}.fbx", stem));
                                        }

                                        let mut exists = false;
                                        for p in &check_paths {
                                            let file_path = if let Some(parent) = base.parent() {
                                                parent.join(p)
                                            } else {
                                                Path::new(p).to_path_buf()
                                            };
                                            if file_path.exists() {
                                                exists = true;
                                                break;
                                            }
                                        }

                                        if !exists {
                                            diagnostics.push(Diagnostic {
                                                range: *range,
                                                severity: Some(DiagnosticSeverity::ERROR),
                                                message: format!(
                                                    "File '{}' in FilepathTable does not exist.",
                                                    s
                                                ),
                                                source: Some(String::from("acs-validator")),
                                                ..Default::default()
                                            });
                                        }
                                    }
                                }
                            }
                        }
                        Validation::NeedCollateMeshes(_meshes) => {
                            // ...
                        }
                        Validation::NotOwnParent => {
                            // ...
                        }
                        Validation::Named(validation) => {
                            if validation.eq_ignore_ascii_case("ScriptFileExists")
                                || validation.eq_ignore_ascii_case("scriptfileexists")
                            {
                                if let Some(base_path) = base_path {
                                    let mut exists = false;
                                    let script_names = if s.to_lowercase().ends_with(".gs") {
                                        vec![s.clone()]
                                    } else {
                                        vec![s.clone(), format!("{}.gs", s)]
                                    };

                                    for name in script_names {
                                        let file_path = if let Some(parent) = base_path.parent() {
                                            parent.join(&name)
                                        } else {
                                            Path::new(&name).to_path_buf()
                                        };
                                        if file_path.exists() {
                                            exists = true;
                                            break;
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
                                            source: Some(String::from("acs-validator")),
                                            ..Default::default()
                                        });
                                    }
                                }
                            } else if validation.eq_ignore_ascii_case("IsNotZero") {
                                let val = match value {
                                    Value::Numeric(NumericValue::Int(i), _) => *i as f64,
                                    Value::Numeric(NumericValue::Float(f), _) => *f,
                                    Value::String(s, _) => s.parse::<f64>().unwrap_or(1.0), // assume non-zero if not a number?
                                    _ => 0.0,
                                };
                                if val == 0.0 {
                                    diagnostics.push(Diagnostic {
                                        range: value.range(),
                                        severity: Some(DiagnosticSeverity::ERROR),
                                        message: format!(
                                            "Value for key '{}' must not be zero.",
                                            rule.key
                                        ),
                                        source: Some(String::from("acs-validator")),
                                        ..Default::default()
                                    });
                                }
                            } else if validation.eq_ignore_ascii_case("IsPositive") {
                                let val = match value {
                                    Value::Numeric(NumericValue::Int(i), _) => *i as f64,
                                    Value::Numeric(NumericValue::Float(f), _) => *f,
                                    Value::String(s, _) => s.parse::<f64>().unwrap_or(0.0),
                                    _ => -1.0,
                                };
                                if val < 0.0 {
                                    diagnostics.push(Diagnostic {
                                        range: value.range(),
                                        severity: Some(DiagnosticSeverity::ERROR),
                                        message: format!(
                                            "Value for key '{}' must be positive.",
                                            rule.key
                                        ),
                                        source: Some(String::from("acs-validator")),
                                        ..Default::default()
                                    });
                                }
                            } else if validation.eq_ignore_ascii_case("HasValidTextureFile") {
                                if let Some(base_path) = base_path {
                                    let file_path = if let Some(parent) = base_path.parent() {
                                        parent.join(s)
                                    } else {
                                        Path::new(s).to_path_buf()
                                    };
                                    if !file_path.exists() {
                                        diagnostics.push(Diagnostic {
                                            range: value.range(),
                                            severity: Some(DiagnosticSeverity::ERROR),
                                            message: format!(
                                                "Texture file '{}' does not exist.",
                                                s
                                            ),
                                            source: Some(String::from("acs-validator")),
                                            ..Default::default()
                                        });
                                    } else {
                                        diagnostics.push(Diagnostic {
                                            range: value.range(),
                                            severity: Some(DiagnosticSeverity::WARNING),
                                            message: String::from(
                                                "Texture validation has not been done yet.",
                                            ),
                                            source: Some(String::from("acs-validator")),
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
                                        source: Some(String::from("acs-validator")),
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
