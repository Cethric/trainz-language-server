use crate::acs_text::validate_rule_node::validate_rule_node;
use crate::sources::DIAGNOSTIC_SOURCE;
use rayon::prelude::*;
use std::path::Path;
use std::sync::Arc;
use tower_lsp_server::ls_types::{Diagnostic, DiagnosticSeverity};
use tracing::{trace, warn};
use trainz_acs_text_validators::RuleNode;
use trainz_acs_text_validators::validation_graph::node::RuleNodeKind;
use trainz_ast::acs_text::AcsText;
use trainz_ast::{Position, Range};

/// Performs diagnostic validation on an `AcsText` node based on a `RuleNode`.
///
/// # Arguments
///
/// * `rule_node` - The `RuleNode` to use for validation.
/// * `acs_text` - The `AcsText` structure to validate.
/// * `trainz_build` - The Trainz build version.
/// * `base_path` - Optional base path for file resolution during validation.
///
/// # Returns
/// A `Vec<Diagnostic>` containing any errors or warnings found.
#[tracing::instrument(skip(rule_node, acs_text, trainz_build, base_path))]
pub(crate) fn acs_text_validator(
    rule_node: &Arc<RuleNode>,
    acs_text: &AcsText,
    trainz_build: f64,
    base_path: &Option<&Path>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if rule_node.is_top_level() {
        let rule_nodes = rule_node
            .inheritance()
            .par_iter()
            .filter_map(|node| {
                if let Some(RuleNodeKind::Structure(structure)) = node.kind() {
                    Some(
                        structure
                            .possibilities()
                            .par_iter()
                            .filter_map(|(_, node)| node.upgrade())
                            .map(|node| node.inheritance())
                            .flatten()
                            .collect(),
                    )
                } else if let Some(RuleNodeKind::Element(element)) = node.kind()
                    && let Some(element_node) = element.element()
                    && let Some(element_node) = element_node.upgrade()
                {
                    Some(element_node.inheritance())
                } else {
                    None
                }
            })
            .flatten()
            .collect::<Vec<Arc<RuleNode>>>();

        let key_value_pairs = &acs_text.key_value_pairs;

        diagnostics.par_extend(key_value_pairs.par_iter().filter_map(|kv| {
            trace!(
                "Checking if key '{}' is valid. {:?}",
                kv.key,
                rule_nodes
                    .par_iter()
                    .map(|node| node.name())
                    .collect::<Vec<String>>()
            );
            if !rule_nodes.par_iter().any(|node| node.matches_name(&kv.key)) {
                Some(Diagnostic {
                    range: kv.key_range,
                    severity: Some(DiagnosticSeverity::WARNING),
                    message: format!("Unknown key '{}'", kv.key),
                    source: Some(String::from(DIAGNOSTIC_SOURCE)),
                    ..Default::default()
                })
            } else {
                None
            }
        }));

        diagnostics.par_extend(
            rule_nodes
                .par_iter()
                .filter_map(|node| {
                    if key_value_pairs
                        .par_iter()
                        .any(|kv| node.matches_name(&kv.key))
                    {
                        None
                    } else {
                        Some(node)
                    }
                })
                .filter(|node| node.is_compulsory(trainz_build))
                .map(|node| Diagnostic {
                    range: Range {
                        start: Position {
                            line: 0,
                            character: 0,
                        },
                        end: Position {
                            line: 0,
                            character: 0,
                        },
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    message: format!("Required key '{}' not found", node.name()),
                    source: Some(String::from(DIAGNOSTIC_SOURCE)),
                    ..Default::default()
                }),
        );

        diagnostics.par_extend(
            rule_nodes
                .par_iter()
                .map(|node| {
                    key_value_pairs
                        .par_iter()
                        .filter_map(|key_value_pair| {
                            if node.matches_name(&key_value_pair.key) {
                                validate_rule_node(node, key_value_pair, trainz_build, base_path)
                            } else {
                                trace!(
                                    "Key '{}' does not match rule node name: {:?}",
                                    key_value_pair.key,
                                    node.name()
                                );
                                None
                            }
                        })
                        .flatten()
                })
                .flatten(),
        );
    } else {
        warn!("Rule node is not top level: {:?}", rule_node.name());
    }

    diagnostics
}
