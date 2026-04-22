use crate::acs_text::common::{expected_value_type_for_key, missing_key};
use crate::acs_text::validate_rule_node::validate_rule_node;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::path::Path;
use tower_lsp_server::ls_types::Diagnostic;
use tracing::{debug, trace, warn};
use trainz_acs_text_validators::validation_graph::structure::RuleNodeStructure;
use trainz_ast::acs_text::{KeyValuePair, Value};

/// Validates a `KeyValuePair` against a `RuleNodeStructure`.
///
/// # Arguments
///
/// * `structure` - The `RuleNodeStructure` to validate against.
/// * `key_value_pair` - The `KeyValuePair` AST node to validate.
/// * `trainz_build` - The Trainz build version.
/// * `base_path` - Optional base path for file resolution during validation.
///
/// # Returns
/// An `Option<Vec<Diagnostic>>` containing any errors or warnings found, if any.
pub fn validate_node_structure(
    structure: &RuleNodeStructure,
    key_value_pair: &KeyValuePair,
    trainz_build: f64,
    base_path: &Option<&Path>,
) -> Option<Vec<Diagnostic>> {
    if let Some(value) = &key_value_pair.value {
        if let Value::Container(entries, _, _) = value {
            let diagnostics = structure
                .array_elements()
                .par_iter()
                .filter_map(|element| element.upgrade())
                .map(|element| element.inheritance())
                .filter_map(|rules| {
                    let grouped = entries
                        .par_iter()
                        .filter_map(|entry| {
                            let parsed = rules
                                .par_iter()
                                .filter_map(|rule| {
                                    validate_rule_node(rule, entry, trainz_build, base_path)
                                })
                                .flatten()
                                .collect::<Vec<Diagnostic>>();

                            if parsed.is_empty() {
                                None
                            } else {
                                Some(parsed)
                            }
                        })
                        .flatten()
                        .collect::<Vec<Diagnostic>>();

                    if grouped.is_empty() {
                        None
                    } else {
                        Some(grouped)
                    }
                })
                .collect::<Vec<Vec<Diagnostic>>>();

            if structure.tag_array().is_none()
                && !diagnostics.is_empty()
                && diagnostics.par_iter().any(|d| !d.is_empty())
                && let Some(diagnostics) = diagnostics.first()
            {
                return Some(diagnostics.clone());
            }

            if let Some(tag_array) = structure.tag_array() {
                debug!(
                    "Validating node kind for structure with tag array {}",
                    key_value_pair.key
                );
                trace!(
                    "Validating node kind for structure with tag array {} - {:?} -> {:?}",
                    key_value_pair.key,
                    tag_array,
                    tag_array.upgrade()
                );

                if let Some(tag_array) = tag_array.upgrade() {
                    let inherited = tag_array.inheritance();

                    let diagnostics = entries
                        .par_iter()
                        .filter_map(|entry| {
                            let result = inherited
                                .par_iter()
                                .filter_map(|rule| {
                                    validate_rule_node(rule, entry, trainz_build, base_path)
                                })
                                .flatten()
                                .collect::<Vec<Diagnostic>>();
                            if result.is_empty() {
                                None
                            } else {
                                Some(result)
                            }
                        })
                        .flatten()
                        .collect::<Vec<Diagnostic>>();

                    if !diagnostics.is_empty() {
                        return Some(diagnostics);
                    }
                }
            }

            let diagnostics = entries
                .par_iter()
                .filter_map(|entry| {
                    if let Some(rule) = structure.get_possibility(&entry.key)
                        && let Some(rule) = rule.upgrade()
                    {
                        let diagnostics = rule
                            .inheritance()
                            .par_iter()
                            .filter_map(|rule| {
                                validate_rule_node(rule, entry, trainz_build, base_path)
                            })
                            .flatten()
                            .collect::<Vec<Diagnostic>>();

                        if diagnostics.is_empty() {
                            None
                        } else {
                            Some(diagnostics)
                        }
                    } else {
                        warn!("No validation found for: {:?}", entry.key);
                        None
                    }
                })
                .flatten()
                .collect::<Vec<Diagnostic>>();

            if diagnostics.is_empty() {
                None
            } else {
                Some(diagnostics)
            }
        } else {
            Some(vec![expected_value_type_for_key(
                "container",
                key_value_pair,
            )])
        }
    } else {
        Some(vec![missing_key(key_value_pair)])
    }
}
