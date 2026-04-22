use rayon::prelude::*;
use tower_lsp_server::ls_types::{DocumentSymbol, SymbolKind};
use trainz_ast::acs_text::base::AcsText;
use trainz_ast::acs_text::key_value_pair::KeyValuePair;
use trainz_common::range::clamp_range;

#[tracing::instrument(skip(acs_text))]
pub fn acs_text_symboliser(acs_text: &AcsText) -> Vec<DocumentSymbol> {
    let symbols: Vec<DocumentSymbol> = acs_text
        .key_value_pairs
        .par_iter()
        .map(process_key_value_symbol)
        .collect();

    symbols
}

#[tracing::instrument(skip(kv))]
fn process_key_value_symbol(kv: &KeyValuePair) -> DocumentSymbol {
    let children = vec![];

    #[allow(deprecated)]
    DocumentSymbol {
        name: kv.key.clone(),
        detail: None,
        kind: if children.is_empty() {
            SymbolKind::PROPERTY
        } else {
            SymbolKind::NAMESPACE
        },
        tags: None,
        deprecated: None,
        range: kv.range,
        selection_range: clamp_range(&kv.range, kv.key_range),
        children: if children.is_empty() {
            None
        } else {
            Some(children)
        },
    }
}
