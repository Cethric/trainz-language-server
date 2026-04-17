use rayon::prelude::*;
use tower_lsp_server::ls_types::{SemanticTokenModifier, SemanticTokenType};

#[tracing::instrument(skip(target))]
pub fn get_token_type(target: SemanticTokenType) -> u32 {
    let (types, _) = get_legend();
    types.par_iter().position_first(|t| *t == target).unwrap() as u32
}

#[tracing::instrument]
pub fn get_legend() -> (Vec<SemanticTokenType>, Vec<SemanticTokenModifier>) {
    let token_types = vec![
        SemanticTokenType::CLASS,     // 0
        SemanticTokenType::METHOD,    // 1
        SemanticTokenType::PROPERTY,  // 2
        SemanticTokenType::PARAMETER, // 3
        SemanticTokenType::VARIABLE,  // 4
        SemanticTokenType::STRING,    // 5
        SemanticTokenType::NUMBER,    // 6
        SemanticTokenType::KEYWORD,   // 7
        SemanticTokenType::OPERATOR,  // 8
        SemanticTokenType::TYPE,      // 9
        SemanticTokenType::MODIFIER,  // 10
        SemanticTokenType::COMMENT,   // 11
    ];

    let token_modifiers = vec![
        SemanticTokenModifier::DEPRECATED,      // 1
        SemanticTokenModifier::DECLARATION,     // 2
        SemanticTokenModifier::DEFINITION,      // 4
        SemanticTokenModifier::READONLY,        // 8
        SemanticTokenModifier::STATIC,          // 16
        SemanticTokenModifier::DOCUMENTATION,   // 32
        SemanticTokenModifier::DEFAULT_LIBRARY, // 64
    ];

    (token_types, token_modifiers)
}
