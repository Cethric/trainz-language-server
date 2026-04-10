use rayon::prelude::*;
use std::collections::HashMap;
use tower_lsp_server::ls_types::{Diagnostic, DiagnosticSeverity};
use trainz_ast::Range;
use trainz_ast::soup::Value;

pub fn validate_simple_value(
    value: &Value,
    key_to_check: &str,
    allowed_values: &HashMap<String, Option<String>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let value_str = match value {
        Value::String(s, _) => s.clone(),
        Value::Variable(s, _) => s.clone(),
        _ => return,
    };

    validate_simple_value_str(
        value.range(),
        value_str.as_str(),
        key_to_check,
        allowed_values,
        diagnostics,
    );
}

pub fn validate_simple_value_str(
    range: Range,
    value: &str,
    key_to_check: &str,
    allowed_values: &HashMap<String, Option<String>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !allowed_values.contains_key(value) {
        diagnostics.push(Diagnostic {
            range,
            severity: Some(DiagnosticSeverity::ERROR),
            message: format!(
                "Invalid value(s) '{}' for key '{}'. Allowed values are: {}",
                value,
                key_to_check,
                allowed_values
                    .par_iter()
                    .map(|(k, v)| if let Some(v) = v {
                        format!("{}: {}", k, v)
                    } else {
                        k.to_string()
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            source: Some(String::from("soup-validator")),
            ..Default::default()
        });
    }
}
