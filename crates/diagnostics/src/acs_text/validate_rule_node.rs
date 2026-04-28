use crate::acs_text::common::key_is_obsolete;
use crate::acs_text::validate_node_kind::validate_node_kind;
use rayon::prelude::*;
use std::path::Path;
use std::sync::Arc;
use tower_lsp_server::ls_types::Diagnostic;
use tracing::trace;
use trainz_acs_text_validators::RuleNode;
use trainz_ast::acs_text::KeyValuePair;

/// Validates a `KeyValuePair` against a `RuleNode`.
///
/// # Arguments
///
/// * `rule_node` - The `RuleNode` to validate against.
/// * `key_value_pair` - The `KeyValuePair` AST node to validate.
/// * `trainz_build` - The Trainz build version.
/// * `base_path` - Optional base path for file resolution during validation.
///
/// # Returns
/// An `Option<Vec<Diagnostic>>` containing any errors or warnings found, if any.
#[tracing::instrument(skip(rule_node, key_value_pair, trainz_build, base_path))]
pub(crate) fn validate_rule_node(
    rule_node: &Arc<RuleNode>,
    key_value_pair: &KeyValuePair,
    trainz_build: f64,
    base_path: &Option<&Path>,
) -> Option<Vec<Diagnostic>> {
    if let Some(kind) = rule_node.kind() {
        trace!(
            "Validating rule node: {:?}, key: {:?}",
            rule_node.name(),
            key_value_pair.key
        );
        let mut diagnostics: Vec<Diagnostic> = if rule_node.is_obsolete(&trainz_build) {
            vec![key_is_obsolete(
                key_value_pair,
                rule_node.obsolete_since().unwrap_or(0.0f64),
            )]
        } else {
            vec![]
        };
        if let Some(validation) = validate_node_kind(kind, key_value_pair, trainz_build, base_path)
        {
            diagnostics.par_extend(validation);
        }
        if diagnostics.is_empty() {
            None
        } else {
            Some(diagnostics)
        }
    } else {
        None
    }
}
