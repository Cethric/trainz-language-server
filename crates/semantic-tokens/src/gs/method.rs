use crate::gs::stmt::collect_stmt_tokens;
use crate::gs::types::collect_type_tokens;
use tower_lsp_server::ls_types::{Range, SemanticTokenModifier, SemanticTokenType};
use trainz_ast::gs::{MethodDef, MethodModifier};

#[tracing::instrument]
pub fn collect_method_tokens(
    method: &MethodDef,
    raw_tokens: &mut Vec<(Range, SemanticTokenType, Vec<SemanticTokenModifier>)>,
    known_classes: &std::collections::HashSet<String>,
    has_separate_declaration: bool,
) {
    let is_native = method
        .modifiers
        .iter()
        .any(|(m, _)| matches!(m, MethodModifier::Native));

    for (modifier, range) in &method.modifiers {
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

    match &method.return_type {
        trainz_ast::gs::types::TypeOrVoid::Type(ty) => collect_type_tokens(ty, raw_tokens),
        trainz_ast::gs::types::TypeOrVoid::Void(range) => {
            raw_tokens.push((*range, SemanticTokenType::TYPE, vec![]))
        }
    }

    let mut method_modifiers = vec![];
    if is_native {
        method_modifiers.push(SemanticTokenModifier::DECLARATION);
        method_modifiers.push(SemanticTokenModifier::DEFINITION);
    } else if method.body.is_some() {
        method_modifiers.push(SemanticTokenModifier::DEFINITION);
        if !has_separate_declaration {
            method_modifiers.push(SemanticTokenModifier::DECLARATION);
        }
    } else {
        method_modifiers.push(SemanticTokenModifier::DECLARATION);
    }

    raw_tokens.push((
        method.name.range,
        SemanticTokenType::METHOD,
        method_modifiers,
    ));

    for param in &method.params {
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

    if let Some(block) = &method.body {
        for stmt in &block.statements {
            collect_stmt_tokens(stmt, raw_tokens, known_classes);
        }
    }
}
