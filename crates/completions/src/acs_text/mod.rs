use tower_lsp_server::ls_types::{CompletionItem, CompletionParams};
use trainz_acs_text_validators::Validators;
use trainz_ast::acs_text::base::AcsText;

pub mod keys;
pub mod recursive;
pub mod utils;
pub mod values;

#[cfg(test)]
mod tests;

#[tracing::instrument]
pub fn acs_text_completions(
    acs_text: &AcsText,
    params: CompletionParams,
    validators: &Validators,
) -> Vec<CompletionItem> {
    log::debug!(
        "Computing acs_text completions at line {}, character {}",
        params.text_document_position.position.line,
        params.text_document_position.position.character
    );

    recursive::find_completions_recursive(
        &acs_text.key_value_pairs,
        params.text_document_position.position,
        validators,
        None,
    )
}
