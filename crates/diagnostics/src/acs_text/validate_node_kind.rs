use crate::acs_text::validate_node_element::validate_node_element;
use crate::acs_text::validate_node_structure;
use crate::acs_text::validate_node_value::validate_node_value;
use std::path::Path;
use tower_lsp_server::ls_types::Diagnostic;
use tracing::{debug, trace, warn};
use trainz_acs_text_validators::validation_graph::node::RuleNodeKind;
use trainz_ast::acs_text::KeyValuePair;

/// Validates a `KeyValuePair` against a `RuleNodeKind`.
///
/// # Arguments
///
/// * `kind` - The `RuleNodeKind` to validate against.
/// * `key_value_pair` - The `KeyValuePair` AST node to validate.
/// * `trainz_build` - The Trainz build version.
/// * `base_path` - Optional base path for file resolution during validation.
///
/// # Returns
/// An `Option<Vec<Diagnostic>>` containing any errors or warnings found, if any.
#[tracing::instrument(skip(kind, key_value_pair, trainz_build, base_path))]
pub(crate) fn validate_node_kind(
    kind: &RuleNodeKind,
    key_value_pair: &KeyValuePair,
    trainz_build: f64,
    base_path: &Option<&Path>,
) -> Option<Vec<Diagnostic>> {
    match kind {
        RuleNodeKind::Value(value) => {
            debug!("Validating value: {}", key_value_pair.key);
            trace!("Validating value: {:?} - {:?}", value, key_value_pair.value);
            validate_node_value(value, key_value_pair, trainz_build, base_path)
        }
        RuleNodeKind::Element(element) => {
            debug!("Validating element: {}", key_value_pair.key);
            trace!(
                "Validating element: {:?} - {:?}",
                element, key_value_pair.value
            );
            validate_node_element(element, key_value_pair, trainz_build, base_path)
        }
        RuleNodeKind::Structure(structure) => {
            debug!("Validating structure: {}", key_value_pair.key);
            trace!(
                "Validating structure: {:?} - {:?}",
                structure, key_value_pair.value
            );
            validate_node_structure::validate_node_structure(
                structure,
                key_value_pair,
                trainz_build,
                base_path,
            )
        }
    }
}
