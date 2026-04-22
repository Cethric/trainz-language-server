pub mod value;

#[cfg(test)]
mod tests;

use crate::acs_text::value::collect_value_tokens;
use tower_lsp_server::ls_types::{Range, SemanticTokenModifier, SemanticTokenType};
use trainz_acs_text_validators::RulesRoot;
use trainz_ast::acs_text::base::AcsText;

#[tracing::instrument(skip(acs_text))]
pub fn acs_text_semantic_tokens(
    acs_text: &AcsText,
    graph: &RulesRoot,
) -> Vec<(Range, SemanticTokenType, Vec<SemanticTokenModifier>)> {
    let mut raw_tokens: Vec<(Range, SemanticTokenType, Vec<SemanticTokenModifier>)> = vec![];

    for kv in acs_text.key_value_pairs.iter() {
        let modifiers = vec![];

        raw_tokens.push((kv.key_range, SemanticTokenType::KEYWORD, modifiers));
        if let Some(value) = &kv.value {
            collect_value_tokens(value, &mut raw_tokens);
        }
    }

    raw_tokens
}
