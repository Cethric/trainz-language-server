use crate::gs::stmt::collect_stmt_tokens;
use crate::gs::types::collect_type_tokens;
use gs_ast::gs::{Identifier, MethodModifier, Param, Stmt};
use tower_lsp_server::ls_types::{Range, SemanticTokenModifier, SemanticTokenType};

pub fn collect_method_tokens(
    modifiers: &[(MethodModifier, Range)],
    return_type: &gs_ast::gs::types::TypeOrVoid,
    name: &Identifier,
    params: &[Param],
    body: Option<&[Stmt]>,
    raw_tokens: &mut Vec<(Range, SemanticTokenType, Vec<SemanticTokenModifier>)>,
    known_classes: &std::collections::HashSet<String>,
) {
    for (modifier, range) in modifiers {
        let (token_type, modifiers_bitset) = match modifier {
            MethodModifier::Static => (
                SemanticTokenType::MODIFIER,
                vec![SemanticTokenModifier::STATIC],
            ),
            MethodModifier::Obsolete(_) => (
                SemanticTokenType::MODIFIER,
                vec![SemanticTokenModifier::DEPRECATED],
            ),
            _ => (SemanticTokenType::MODIFIER, vec![]),
        };
        raw_tokens.push((*range, token_type, modifiers_bitset));
    }

    match return_type {
        gs_ast::gs::types::TypeOrVoid::Type(ty) => collect_type_tokens(ty, raw_tokens),
        gs_ast::gs::types::TypeOrVoid::Void(range) => {
            raw_tokens.push((*range, SemanticTokenType::TYPE, vec![]))
        }
    }

    raw_tokens.push((
        name.range,
        SemanticTokenType::METHOD,
        vec![
            SemanticTokenModifier::DECLARATION,
            SemanticTokenModifier::DEFINITION,
        ],
    ));

    for param in params {
        collect_type_tokens(&param.ty, raw_tokens);
        raw_tokens.push((
            param.name.range,
            SemanticTokenType::PARAMETER,
            vec![
                SemanticTokenModifier::DECLARATION,
                SemanticTokenModifier::DEFINITION,
            ],
        ));
    }

    if let Some(statements) = body {
        for stmt in statements {
            collect_stmt_tokens(stmt, raw_tokens, known_classes);
        }
    }
}
