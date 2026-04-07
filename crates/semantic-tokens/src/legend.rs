use tower_lsp_server::ls_types::{SemanticTokenModifier, SemanticTokenType};

pub fn get_legend() -> (Vec<SemanticTokenType>, Vec<SemanticTokenModifier>) {
    let token_types = vec![
        SemanticTokenType::KEYWORD,   // 0
        SemanticTokenType::CLASS,     // 1
        SemanticTokenType::STRING,    // 2
        SemanticTokenType::TYPE,      // 3
        SemanticTokenType::METHOD,    // 4
        SemanticTokenType::OPERATOR,  // 5
        SemanticTokenType::NUMBER,    // 6
        SemanticTokenType::PROPERTY,  // 7
        SemanticTokenType::PARAMETER, // 8
        SemanticTokenType::MODIFIER,  // 9
        SemanticTokenType::VARIABLE,  // 10
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
