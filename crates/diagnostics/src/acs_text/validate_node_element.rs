use crate::acs_text::validate_rule_node::validate_rule_node;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::path::Path;
use tower_lsp_server::ls_types::Diagnostic;
use tracing::{debug, warn};
use trainz_acs_text_validators::validation_graph::element::RuleNodeElement;
use trainz_ast::acs_text::KeyValuePair;

/// Validates a node element against a `RuleNodeElement`.
///
/// # Arguments
///
/// * `element` - The `RuleNodeElement` to validate against.
/// * `key_value_pair` - The `KeyValuePair` AST node to validate.
/// * `trainz_build` - The Trainz build version.
/// * `base_path` - Optional base path for file resolution during validation.
///
/// # Returns
/// An `Option<Vec<Diagnostic>>` containing any errors or warnings found, if any.
#[tracing::instrument(skip(element, key_value_pair, trainz_build, base_path))]
pub(crate) fn validate_node_element(
    element: &RuleNodeElement,
    key_value_pair: &KeyValuePair,
    trainz_build: f64,
    base_path: &Option<&Path>,
) -> Option<Vec<Diagnostic>> {
    if let Some(node_rule) = element.element()
        && let Some(node_rule) = node_rule.upgrade()
    {
        debug!("Validating element: {:?}", node_rule);
        let rules = node_rule.inheritance();

        // TODO handle element.is_num_array()
        // if element.is_num_array() {
        //     let mut last: i64 = 0;
        //     for kv in container {
        //         if let Ok(id) = kv.key.parse::<i64>() {
        //             if id < last {
        //                 diagnostics.push(Diagnostic {
        //                     range: kv.key_range,
        //                     severity: Some(DiagnosticSeverity::WARNING),
        //                     message: "Array elements should be in ascending order"
        //                         .to_string(),
        //                     source: Some(String::from(DIAGNOSTIC_SOURCE)),
        //                     ..Default::default()
        //                 });
        //                 break;
        //             }
        //             last = id;
        //         } else {
        //             diagnostics.push(Diagnostic {
        //                 range: kv.key_range,
        //                 severity: Some(DiagnosticSeverity::ERROR),
        //                 message: "Keys should be numbers".to_string(),
        //                 source: Some(String::from(DIAGNOSTIC_SOURCE)),
        //                 ..Default::default()
        //             });
        //         }
        //     }
        // }

        let diagnostics = rules
            .par_iter()
            .filter_map(|rule| validate_rule_node(&rule, key_value_pair, trainz_build, base_path))
            .flatten()
            .collect::<Vec<Diagnostic>>();
        if diagnostics.is_empty() {
            None
        } else {
            Some(diagnostics)
        }
    } else {
        warn!("No rules found for container {}", key_value_pair.key);
        None
    }
}
