use crate::acs_text::acs_text_validator::acs_text_validator;
use crate::sources::DIAGNOSTIC_SOURCE;
use rayon::prelude::*;
use std::path::Path;
use std::sync::{Arc, Weak};
use tower_lsp_server::ls_types::Diagnostic;
use tracing::trace;
use trainz_acs_text_validators::{RuleNode, RulesRoot, parse_as_numeric, parse_as_string};
use trainz_ast::acs_text::{AcsText, KeyValuePair};
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

    let kind: Vec<&KeyValuePair> = acs_text
        .key_value_pairs
        .par_iter()
        .filter_map(|kv| {
            if kv.key.eq_ignore_ascii_case("kind") {
                Some(kv)
            } else {
                None
            }
        })
        .collect();

    if kind.is_empty() {
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

    if kind.len() > 1 {
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
            message: "Multiple 'kind' tags found".to_string(),
            source: Some(String::from(DIAGNOSTIC_SOURCE)),
            ..Default::default()
        });
    }
    let kind = kind.first();

    let trainz_build: Vec<&KeyValuePair> = acs_text
        .key_value_pairs
        .par_iter()
        .filter_map(|kv| {
            if kv.key.eq_ignore_ascii_case("trainz-build") {
                Some(kv)
            } else {
                None
            }
        })
        .collect();
    let trainz_build: f64 = if let Some(kv) = trainz_build.first()
        && let Some(version) = parse_as_numeric::<f64>(kv)
    {
        version
    } else {
        0.0f64
    };

    if let Some(kind) = kind
        && let Some(kind_value) = parse_as_string(kind)
    {
        let validator: Vec<Arc<RuleNode>> = graph
            .get_top_level_nodes()
            .par_iter()
            .filter_map(|node| {
                if let Some(node) = Weak::upgrade(node)
                    && node.matches_name(&kind_value)
                {
                    Some(node)
                } else {
                    None
                }
            })
            .collect();

        if validator.is_empty() {
            diagnostics.push(Diagnostic {
                range: kind.key_range,
                severity: Some(tower_lsp_server::ls_types::DiagnosticSeverity::ERROR),
                message: format!("Unknown kind '{}'", kind_value),
                source: Some(String::from(DIAGNOSTIC_SOURCE)),
                ..Default::default()
            });
        }

        if validator.len() > 1 {
            diagnostics.push(Diagnostic {
                range: kind.key_range,
                severity: Some(tower_lsp_server::ls_types::DiagnosticSeverity::WARNING),
                message: format!("Multiple rules found for kind '{}'", kind_value),
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
    }

    diagnostics
}
