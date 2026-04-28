use crate::acs_text::acs_text_validator::acs_text_validator;
use crate::sources::DIAGNOSTIC_SOURCE;
use rayon::prelude::*;
use std::path::Path;
use std::sync::{Arc, Weak};
use tower_lsp_server::ls_types::Diagnostic;
use tracing::trace;
use trainz_acs_text_validators::text_util::{get_kind_from_text, get_trainz_build_from_text};
use trainz_acs_text_validators::{RuleNode, RulesRoot};
use trainz_ast::acs_text::AcsText;
use trainz_ast::{Position, Range};

mod acs_text_validator;
mod common;
#[cfg(test)]
mod tests;
mod validate_node_element;
mod validate_node_kind;
mod validate_node_structure;
mod validate_node_value;
mod validate_rule_node;

/// Performs diagnostic validation on an `AcsText` AST node.
///
/// # Arguments
///
/// * `acs_text` - The `AcsText` structure to validate.
/// * `graph` - The `RulesRoot` defining validation rules.
/// * `base_path` - Optional base path for file resolution during validation.
///
/// # Returns
/// A `Vec<Diagnostic>` containing any errors or warnings found.
#[tracing::instrument(skip(acs_text, graph, base_path))]
pub fn acs_text_diagnostics(
    acs_text: &AcsText,
    graph: &RulesRoot,
    base_path: Option<&Path>,
) -> Vec<Diagnostic> {
    let mut diagnostics = vec![];

    let kind = get_kind_from_text(acs_text);

    if let Some((kind, kind_range, is_multiple)) = kind {
        if is_multiple {
            diagnostics.push(Diagnostic {
                range: kind_range,
                severity: Some(tower_lsp_server::ls_types::DiagnosticSeverity::ERROR),
                message: "Multiple 'kind' tags found".to_string(),
                source: Some(String::from(DIAGNOSTIC_SOURCE)),
                ..Default::default()
            });
        }

        let trainz_build = get_trainz_build_from_text(acs_text);

        let validator: Vec<Arc<RuleNode>> = graph
            .get_top_level_nodes()
            .par_iter()
            .filter_map(|node| {
                if let Some(node) = Weak::upgrade(node)
                    && node.matches_name(&kind)
                {
                    Some(node)
                } else {
                    None
                }
            })
            .collect();

        if validator.is_empty() {
            diagnostics.push(Diagnostic {
                range: kind_range,
                severity: Some(tower_lsp_server::ls_types::DiagnosticSeverity::ERROR),
                message: format!("Unknown kind '{}'", kind),
                source: Some(String::from(DIAGNOSTIC_SOURCE)),
                ..Default::default()
            });
        }

        if validator.len() > 1 {
            diagnostics.push(Diagnostic {
                range: kind_range,
                severity: Some(tower_lsp_server::ls_types::DiagnosticSeverity::WARNING),
                message: format!("Multiple rules found for kind '{}'", kind),
                source: Some(String::from(DIAGNOSTIC_SOURCE)),
                ..Default::default()
            });
        }

        let validator = validator.first();

        if let Some(validator) = validator {
            trace!("Validating with {:?}", validator);
            diagnostics.par_extend(acs_text_validator(
                validator,
                acs_text,
                trainz_build,
                &base_path,
            ));
        }
    } else {
        diagnostics.push(Diagnostic {
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
            severity: Some(tower_lsp_server::ls_types::DiagnosticSeverity::ERROR),
            message: "'kind' is missing".to_string(),
            source: Some(String::from(DIAGNOSTIC_SOURCE)),
            ..Default::default()
        });
    }

    diagnostics
}
