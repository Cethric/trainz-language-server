use crate::sources::DIAGNOSTIC_SOURCE;
use std::str::FromStr;
use tower_lsp_server::ls_types::{
    Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, DiagnosticTag, Location, Uri,
};
use trainz_ast::Range;
use trainz_ast::acs_text::{KeyValuePair, Value};

/// Returns the range of the value in the `KeyValuePair`, or the full range if the value is missing.
fn key_value_pair_value_range(key_value_pair: &KeyValuePair) -> Range {
    if let Some(value) = &key_value_pair.value {
        value.range()
    } else {
        key_value_pair.range
    }
}

/// Generates a diagnostic for a missing value for a required key.
#[tracing::instrument(skip(key_value_pair))]
pub fn missing_key(key_value_pair: &KeyValuePair) -> Diagnostic {
    Diagnostic {
        range: key_value_pair.key_range,
        severity: Some(DiagnosticSeverity::ERROR),
        message: format!("Missing value for key '{}'", key_value_pair.key),
        source: Some(String::from(DIAGNOSTIC_SOURCE)),
        ..Default::default()
    }
}

/// Converts an `Option<Value>` to its corresponding type name string.
#[tracing::instrument(skip(value))]
fn value_to_type(value: &Option<Value>) -> String {
    match value {
        Some(Value::Container(_, _, _)) => "container",
        Some(Value::String(_, _)) => "string",
        Some(Value::Variable(_, _)) => "string",
        Some(Value::Kuid(_, _)) => "kuid",
        Some(Value::Array(_, _)) => "array",
        Some(Value::Numeric(_, _)) => "numeric",
        None => "none",
    }
    .to_string()
}

/// Generates a diagnostic for a type mismatch in a `KeyValuePair`.
#[tracing::instrument(skip(value_type, key_value_pair))]
pub fn expected_value_type_for_key_but_got(
    value_type: &str,
    got: &str,
    key_value_pair: &KeyValuePair,
) -> Diagnostic {
    Diagnostic {
        range: key_value_pair_value_range(key_value_pair),
        severity: Some(DiagnosticSeverity::ERROR),
        message: format!(
            "Expected {} value for key '{}', but got '{}'",
            value_type, key_value_pair.key, got
        ),
        source: Some(String::from(DIAGNOSTIC_SOURCE)),
        ..Default::default()
    }
}

/// Generates a diagnostic for a type mismatch in a `KeyValuePair`, automatically resolving the got type.
#[tracing::instrument(skip(value_type, key_value_pair))]
pub fn expected_value_type_for_key(value_type: &str, key_value_pair: &KeyValuePair) -> Diagnostic {
    expected_value_type_for_key_but_got(
        value_type,
        &value_to_type(&key_value_pair.value),
        key_value_pair,
    )
}

/// Generates a diagnostic for an invalid value item in a `KeyValuePair`.
#[tracing::instrument(skip(values, value, key_value_pair))]
pub fn expected_value_item_for_key_but_got(
    values: &str,
    value: &str,
    key_value_pair: &KeyValuePair,
) -> Diagnostic {
    Diagnostic {
        range: key_value_pair_value_range(key_value_pair),
        severity: Some(DiagnosticSeverity::ERROR),
        message: format!(
            "Invalid value for key '{}'. Expected one of {} but got '{}'",
            key_value_pair.key, values, value
        ),
        source: Some(String::from(DIAGNOSTIC_SOURCE)),
        ..Default::default()
    }
}

/// Generates a diagnostic for an invalid value item in a `KeyValuePair` with unknown value.
#[tracing::instrument(skip(values, key_value_pair))]
pub fn expected_value_item_for_key(values: &str, key_value_pair: &KeyValuePair) -> Diagnostic {
    expected_value_item_for_key_but_got(values, "unknown", key_value_pair)
}

/// Generates a hint diagnostic indicating a key is obsolete.
#[tracing::instrument(skip(key_value_pair, obsolete_since))]
pub fn key_is_obsolete(key_value_pair: &KeyValuePair, obsolete_since: f64) -> Diagnostic {
    Diagnostic {
        range: key_value_pair.range,
        severity: Some(DiagnosticSeverity::HINT),
        tags: Some(vec![DiagnosticTag::DEPRECATED]),
        message: format!("{} is obsolete", key_value_pair.key),
        related_information: Some(vec![DiagnosticRelatedInformation {
            location: Location {
                uri: Uri::from_str("https://online.ts2009.com/mediaWiki/index.php/Main_Page")
                    .unwrap(),
                range: key_value_pair.range,
            },
            message: format!("Obsolete since trainz {}", obsolete_since),
        }]),
        source: Some(String::from(DIAGNOSTIC_SOURCE)),
        ..Default::default()
    }
}
