use rayon::prelude::*;
use std::sync::{Arc, Weak};
use tower_lsp_server::ls_types::{
    CompletionItem, CompletionItemKind, CompletionItemLabelDetails, CompletionItemTag,
    CompletionParams, Documentation, MarkupContent, MarkupKind,
};
use tracing::debug;
use trainz_acs_text_validators::text_util::{get_kind_from_text, get_trainz_build_from_text};
use trainz_acs_text_validators::validation_graph::node::RuleNodeKind;
use trainz_acs_text_validators::{RuleNode, RulesRoot};
use trainz_ast::acs_text::AcsText;

#[tracing::instrument(skip(acs_text, params, _asset_cache_path, graph))]
pub fn acs_text_completions(
    acs_text: &AcsText,
    graph: Option<&RulesRoot>,
    params: CompletionParams,
    _asset_cache_path: Option<&std::path::Path>,
) -> Vec<CompletionItem> {
    debug!(
        "Computing acs_text completions at line {}, character {}",
        params.text_document_position.position.line,
        params.text_document_position.position.character
    );

    let Some(graph) = graph else {
        return vec![];
    };

    if let Some((kind, _, _)) = get_kind_from_text(acs_text) {
        let trainz_build = get_trainz_build_from_text(acs_text);

        let validators: Vec<Arc<RuleNode>> = graph
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

        let Some(validator) = validators.first() else {
            return vec![];
        };

        let allowed_nodes = validator
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

        allowed_nodes
            .into_iter()
            .filter(|node| {
                !acs_text
                    .key_value_pairs
                    .iter()
                    .any(|kv| kv.key.eq_ignore_ascii_case(&node.name()))
                    && node.is_supported(trainz_build)
            })
            .map(|node| CompletionItem {
                label: node.name(),
                label_details: Some(CompletionItemLabelDetails {
                    detail: node.details(),
                    description: node.description(),
                }),
                kind: Some(CompletionItemKind::PROPERTY),
                detail: node.details(),
                documentation: node.documentation(&trainz_build).map(|documentation| {
                    Documentation::MarkupContent(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: documentation,
                    })
                }),
                deprecated: if node.is_obsolete(trainz_build) {
                    Some(true)
                } else {
                    None
                },
                tags: if node.is_obsolete(trainz_build) {
                    Some(vec![CompletionItemTag::DEPRECATED])
                } else {
                    None
                },
                ..Default::default()
            })
            .collect()
    } else {
        vec![]
    }
}
