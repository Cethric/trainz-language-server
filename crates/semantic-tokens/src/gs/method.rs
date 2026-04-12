use crate::gs::stmt::collect_stmt_tokens;
use crate::gs::types::collect_type_tokens;
use tower_lsp_server::ls_types::{Range, SemanticTokenModifier, SemanticTokenType};
use trainz_ast::gs::{Identifier, MethodModifier, Param, Stmt};

#[tracing::instrument]
pub fn collect_method_tokens(
    modifiers: &[(MethodModifier, Range)],
    return_type: &trainz_ast::gs::types::TypeOrVoid,
    name: &Identifier,
    params: &[Param],
    body: Option<&[Stmt]>,
    raw_tokens: &mut Vec<(Range, SemanticTokenType, Vec<SemanticTokenModifier>)>,
    known_classes: &std::collections::HashSet<String>,
    has_separate_declaration: bool,
) {
    let is_native = modifiers
        .iter()
        .any(|(m, _)| matches!(m, MethodModifier::Native));

    for (modifier, range) in modifiers {
        let (token_type, modifiers_bitset) = match modifier {
            MethodModifier::Static => (
                SemanticTokenType::KEYWORD,
                vec![SemanticTokenModifier::STATIC],
            ),
            MethodModifier::Obsolete(_) => (
                SemanticTokenType::KEYWORD,
                vec![SemanticTokenModifier::DEPRECATED],
            ),
            _ => (SemanticTokenType::KEYWORD, vec![]),
        };
        raw_tokens.push((*range, token_type, modifiers_bitset));
    }

    match return_type {
        trainz_ast::gs::types::TypeOrVoid::Type(ty) => collect_type_tokens(ty, raw_tokens),
        trainz_ast::gs::types::TypeOrVoid::Void(range) => {
            raw_tokens.push((*range, SemanticTokenType::TYPE, vec![]))
        }
    }

    let mut method_modifiers = vec![];
    if is_native {
        method_modifiers.push(SemanticTokenModifier::DECLARATION);
        method_modifiers.push(SemanticTokenModifier::DEFINITION);
    } else if body.is_some() {
        method_modifiers.push(SemanticTokenModifier::DEFINITION);
        if !has_separate_declaration {
            method_modifiers.push(SemanticTokenModifier::DECLARATION);
        }
    } else {
        method_modifiers.push(SemanticTokenModifier::DECLARATION);
    }

    raw_tokens.push((name.range, SemanticTokenType::METHOD, method_modifiers));

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
