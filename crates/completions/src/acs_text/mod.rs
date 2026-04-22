use tower_lsp_server::ls_types::{CompletionItem, CompletionParams};
use tracing::debug;
use trainz_ast::acs_text::base::AcsText;

#[tracing::instrument(skip(_acs_text, params, _asset_cache_path))]
pub fn acs_text_completions(
    _acs_text: &AcsText,
    params: CompletionParams,
    _asset_cache_path: Option<&std::path::Path>,
) -> Vec<CompletionItem> {
    debug!(
        "Computing acs_text completions at line {}, character {}",
        params.text_document_position.position.line,
        params.text_document_position.position.character
    );

    vec![]
}
