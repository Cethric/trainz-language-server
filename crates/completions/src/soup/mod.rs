use tower_lsp_server::ls_types::{CompletionItem, CompletionParams};
use trainz_ast::soup::base::Soup;
use trainz_soup_validators::Validators;

pub mod keys;
pub mod recursive;
pub mod utils;
pub mod values;

#[cfg(test)]
mod tests;

#[tracing::instrument]
pub fn soup_completions(
    soup: &Soup,
    params: CompletionParams,
    validators: &Validators,
) -> Vec<CompletionItem> {
    log::debug!(
        "Computing soup completions at line {}, character {}",
        params.text_document_position.position.line,
        params.text_document_position.position.character
    );

    recursive::find_completions_recursive(
        &soup.key_value_pairs,
        params.text_document_position.position,
        validators,
        None,
    )
}
