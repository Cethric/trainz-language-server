use rayon::prelude::*;
use std::path::Path;
use std::sync::Arc;
use tower_lsp_server::ls_types::{Hover, HoverContents, HoverParams, MarkedString};
use trainz_acs_text_validators::text_util::{get_kind_from_text, get_trainz_build_from_text};
use trainz_acs_text_validators::{RuleNode, RulesRoot};
use trainz_ast::acs_text::{AcsText, KeyValuePair};

#[tracing::instrument(skip(acs_text, graph, params, _base_path, _script_resolver))]
pub fn acs_text_hover(
    acs_text: &AcsText,
    graph: &RulesRoot,
    params: HoverParams,
    _base_path: Option<&Path>,
    _script_resolver: Option<&dyn trainz_definition::acs_text::definitions::ScriptResolver>,
) -> Option<Hover> {
    let container_path =
        acs_text.get_kvp_to_position(params.text_document_position_params.position);
    let last_item = container_path.last();

    if let Some((kind, _, _)) = get_kind_from_text(acs_text)
        && let Some((found, chain)) = graph.get_rule_from_path(&kind, &container_path)
    {
        let trainz_build = get_trainz_build_from_text(acs_text);

        if found && let Some(rule) = chain.last() {
            build_hover_item(last_item, rule, trainz_build)
        } else if !found {
            chain
                .last()
                .and_then(|node| build_hover_item(last_item, node, trainz_build))
        } else {
            None
        }
    } else {
        None
    }
}

fn build_hover_item(
    key_value_pair: Option<&KeyValuePair>,
    rule: &Arc<RuleNode>,
    trainz_build: f64,
) -> Option<Hover> {
    key_value_pair.map(|kvp| {
        let mut contents: Vec<MarkedString> = vec![];
        if let Some(details) = rule.details() {
            contents.push(MarkedString::String(details.to_string()));
        }
        if let Some(description) = rule.description() {
            contents.push(MarkedString::String(description.to_string()));
        }
        if let Some(documentation) = rule.documentation(&trainz_build)
            && !documentation.is_empty()
        {
            contents.par_extend(
                documentation
                    .par_iter()
                    .filter_map(|doc| Some(MarkedString::String(doc.to_string()))),
            );
        }
        Hover {
            contents: HoverContents::Array(contents),
            range: Some(kvp.key_range),
        }
    })
}
