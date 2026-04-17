use tower_lsp_server::ls_types::{Range, SemanticTokenModifier, SemanticTokenType};
use trainz_ast::find::HasRange;
use trainz_ast::gs::types::Type;

#[tracing::instrument(skip(ty, raw_tokens))]
pub fn collect_type_tokens(
    ty: &Type,
    raw_tokens: &mut Vec<(Range, SemanticTokenType, Vec<SemanticTokenModifier>)>,
) {
    match ty {
        Type::Named(id) => {
            raw_tokens.push((id.range, SemanticTokenType::CLASS, vec![]));
        }
        Type::Array(inner, _range) => {
            collect_type_tokens(inner, raw_tokens);
            // Optionally, we could add token for the [] part if we had its exact range.
            // For now, the entire array type range isn't tokenized directly,
            // only its inner type which is what we want.
        }
        _ => {
            raw_tokens.push((ty.range(), SemanticTokenType::TYPE, vec![]));
        }
    }
}
