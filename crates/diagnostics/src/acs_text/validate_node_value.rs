use crate::acs_text::common::{
    expected_value_item_for_key, expected_value_item_for_key_but_got, expected_value_type_for_key,
    expected_value_type_for_key_but_got,
};
use rayon::prelude::*;
use std::path::Path;
use tower_lsp_server::ls_types::Diagnostic;
use tracing::{debug, trace};
use trainz_acs_text_validators::validation_graph::value::RuleNodeValue;
use trainz_acs_text_validators::{
    parse_as_numeric, parse_as_numeric_list, parse_as_string, parse_numeric_value,
};
use trainz_ast::acs_text::{KeyValuePair, Value};

/// Validates a `KeyValuePair` against a `RuleNodeValue`.
///
/// # Arguments
///
/// * `value` - The `RuleNodeValue` to validate against.
/// * `key_value_pair` - The `KeyValuePair` AST node to validate.
/// * `_trainz_build` - The Trainz build version.
/// * `_base_path` - Optional base path for file resolution during validation.
///
/// # Returns
/// An `Option<Vec<Diagnostic>>` containing any errors or warnings found, if any.
#[tracing::instrument(skip(value, key_value_pair, _trainz_build, _base_path))]
pub(crate) fn validate_node_value(
    value: &RuleNodeValue,
    key_value_pair: &KeyValuePair,
    _trainz_build: f64,
    _base_path: &Option<&Path>,
) -> Option<Vec<Diagnostic>> {
    debug!("Validating node value for key: {:?}", key_value_pair.key);
    trace!(
        "Validating node value for key: {:?}, value: {:?}",
        key_value_pair.key, key_value_pair.value
    );
    match value {
        RuleNodeValue::String(_) => {
            if parse_as_string(key_value_pair).is_some() {
                None
            } else {
                Some(vec![expected_value_type_for_key("string", key_value_pair)])
            }
        }
        RuleNodeValue::Float(_) => {
            if parse_as_numeric::<f64>(key_value_pair).is_some() {
                None
            } else {
                Some(vec![expected_value_type_for_key("float", key_value_pair)])
            }
        }
        RuleNodeValue::Integer(_) => {
            if parse_as_numeric::<i64>(key_value_pair).is_some() {
                None
            } else {
                Some(vec![expected_value_type_for_key("integer", key_value_pair)])
            }
        }
        RuleNodeValue::Bool(_) => {
            if let Some(parsed) = parse_as_numeric::<u64>(key_value_pair) {
                if parsed == 0 || parsed == 1 {
                    None
                } else {
                    Some(vec![expected_value_type_for_key("boolean", key_value_pair)])
                }
            } else {
                Some(vec![expected_value_type_for_key("boolean", key_value_pair)])
            }
        }
        RuleNodeValue::Rgb(_) => {
            if let Some(parsed) = parse_as_numeric_list::<u8>(key_value_pair) {
                let base_len = parsed.len();
                if base_len != parsed.par_iter().filter_map(|v| *v).count() || base_len != 3 {
                    Some(vec![expected_value_type_for_key("RGB", key_value_pair)])
                } else {
                    None
                }
            } else {
                Some(vec![expected_value_type_for_key("RGB", key_value_pair)])
            }
        }
        RuleNodeValue::ComboBox(combobox) => {
            if let Some(parsed) = parse_as_string(key_value_pair) {
                if combobox.has_value(&parsed) {
                    None
                } else {
                    Some(vec![expected_value_item_for_key_but_got(
                        &combobox.display_options_inline(),
                        &parsed,
                        key_value_pair,
                    )])
                }
            } else {
                Some(vec![expected_value_item_for_key(
                    &combobox.display_options_inline(),
                    key_value_pair,
                )])
            }
        }
        RuleNodeValue::IntComboBox(combobox) => {
            if let Some(parsed) = parse_as_numeric::<u64>(key_value_pair) {
                if combobox.has_value(&parsed) {
                    None
                } else {
                    Some(vec![expected_value_item_for_key_but_got(
                        &combobox.display_options_inline(),
                        &parsed.to_string(),
                        key_value_pair,
                    )])
                }
            } else {
                Some(vec![expected_value_item_for_key(
                    &combobox.display_options_inline(),
                    key_value_pair,
                )])
            }
        }
        RuleNodeValue::FloatComboBox(combobox) => {
            if let Some(parsed) = parse_as_numeric::<f64>(key_value_pair) {
                if combobox.has_value(&format!("{:.1}", parsed)) {
                    None
                } else {
                    Some(vec![expected_value_item_for_key_but_got(
                        &combobox.display_options_inline(),
                        &parsed.to_string(),
                        key_value_pair,
                    )])
                }
            } else {
                Some(vec![expected_value_item_for_key(
                    &combobox.display_options_inline(),
                    key_value_pair,
                )])
            }
        }
        RuleNodeValue::ListBox(listbox) => {
            if let Some(parsed) = parse_as_string(key_value_pair) {
                let split = parsed
                    .trim()
                    .trim_start_matches(";")
                    .trim_end_matches(";")
                    .split(";")
                    .collect::<Vec<&str>>();
                if split.par_iter().all(|option| listbox.has_value(option)) {
                    None
                } else {
                    Some(
                        split
                            .par_iter()
                            .filter_map(|option| {
                                if listbox.has_value(option) {
                                    None
                                } else {
                                    Some(expected_value_item_for_key_but_got(
                                        &listbox.display_options_inline(),
                                        option,
                                        key_value_pair,
                                    ))
                                }
                            })
                            .collect(),
                    )
                }
            } else {
                Some(vec![expected_value_item_for_key(
                    &listbox.display_options_inline(),
                    key_value_pair,
                )])
            }
        }
        RuleNodeValue::Kuid(_) => {
            if let Some(Value::Kuid(_, _)) = &key_value_pair.value {
                None
            } else {
                Some(vec![expected_value_type_for_key("kuid", key_value_pair)])
            }
        }
        RuleNodeValue::KuidBrowser(_) => {
            if let Some(Value::Kuid(_, _)) = &key_value_pair.value {
                None
            } else {
                Some(vec![expected_value_type_for_key("kuid", key_value_pair)])
            }
        }
        RuleNodeValue::FilePath(_) => {
            // todo verify that the file exists
            None
        }
        RuleNodeValue::FloatList(_) => {
            if let Some(Value::Numeric(numeric, _)) = &key_value_pair.value
                && parse_numeric_value::<f64>(numeric).is_none()
            {
                Some(vec![expected_value_type_for_key(
                    "float list",
                    key_value_pair,
                )])
            } else {
                if let Some(parsed) = parse_as_numeric_list::<f64>(key_value_pair) {
                    debug!("Parsed float list: {:?}", parsed);
                    let base_len = parsed.len();
                    if base_len != parsed.par_iter().filter_map(|v| *v).count() {
                        Some(vec![expected_value_type_for_key(
                            "float list",
                            key_value_pair,
                        )])
                    } else {
                        None
                    }
                } else {
                    Some(vec![expected_value_type_for_key(
                        "float list",
                        key_value_pair,
                    )])
                }
            }
        }
        RuleNodeValue::Vector2(_) => {
            if let Some(parsed) = parse_as_numeric_list::<f64>(key_value_pair) {
                let base_len = parsed.len();
                let transformed_len = parsed.par_iter().filter_map(|v| *v).count();
                if base_len != transformed_len || base_len != 2 {
                    Some(vec![expected_value_type_for_key_but_got(
                        "vector2",
                        &format!("vector{}", transformed_len),
                        key_value_pair,
                    )])
                } else {
                    None
                }
            } else {
                Some(vec![expected_value_type_for_key("vector2", key_value_pair)])
            }
        }
        RuleNodeValue::Vector3(_) => {
            if let Some(parsed) = parse_as_numeric_list::<f64>(key_value_pair) {
                let base_len = parsed.len();
                let transformed_len = parsed.par_iter().filter_map(|v| *v).count();
                if base_len != transformed_len || base_len != 3 {
                    Some(vec![expected_value_type_for_key_but_got(
                        "vector3",
                        &format!("vector{}", transformed_len),
                        key_value_pair,
                    )])
                } else {
                    None
                }
            } else {
                Some(vec![expected_value_type_for_key("vector3", key_value_pair)])
            }
        }
        RuleNodeValue::Vector4(_) => {
            if let Some(parsed) = parse_as_numeric_list::<f64>(key_value_pair) {
                let base_len = parsed.len();
                let transformed_len = parsed.par_iter().filter_map(|v| *v).count();
                if base_len != transformed_len || base_len != 4 {
                    Some(vec![expected_value_type_for_key_but_got(
                        "vector4",
                        &format!("vector{}", transformed_len),
                        key_value_pair,
                    )])
                } else {
                    None
                }
            } else {
                Some(vec![expected_value_type_for_key("vector4", key_value_pair)])
            }
        }
        RuleNodeValue::Vector5(_) => {
            if let Some(parsed) = parse_as_numeric_list::<f64>(key_value_pair) {
                let base_len = parsed.len();
                let transformed_len = parsed.par_iter().filter_map(|v| *v).count();
                if base_len != transformed_len || base_len != 5 {
                    Some(vec![expected_value_type_for_key_but_got(
                        "vector5",
                        &format!("vector{}", transformed_len),
                        key_value_pair,
                    )])
                } else {
                    None
                }
            } else {
                Some(vec![expected_value_type_for_key("vector5", key_value_pair)])
            }
        }
        RuleNodeValue::Vector6(_) => {
            if let Some(parsed) = parse_as_numeric_list::<f64>(key_value_pair) {
                let base_len = parsed.len();
                let transformed_len = parsed.par_iter().filter_map(|v| *v).count();
                if base_len != transformed_len || base_len != 6 {
                    Some(vec![expected_value_type_for_key_but_got(
                        "vector6",
                        &format!("vector{}", transformed_len),
                        key_value_pair,
                    )])
                } else {
                    None
                }
            } else {
                Some(vec![expected_value_type_for_key("vector6", key_value_pair)])
            }
        }
    }
}
