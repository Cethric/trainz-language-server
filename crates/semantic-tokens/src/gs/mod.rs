pub mod expr;
pub mod method;
pub mod stmt;
pub mod types;

#[cfg(test)]
mod tests;

use crate::gs::expr::collect_expr_tokens;
use crate::gs::method::collect_method_tokens;
use crate::gs::types::collect_type_tokens;
use rayon::prelude::*;
use tower_lsp_server::ls_types::{Range, SemanticTokenModifier, SemanticTokenType};
use trainz_ast::gs::program::Program;
use trainz_ast::gs::{ClassModifier, FieldModifier};

pub fn semantic_tokens(
    program: &Program,
) -> Vec<(Range, SemanticTokenType, Vec<SemanticTokenModifier>)> {
    // Collect all tokens with their type and modifiers
    let mut raw_tokens: Vec<(Range, SemanticTokenType, Vec<SemanticTokenModifier>)> = vec![];

    // Gather all known class names
    let known_classes: std::collections::HashSet<String> = program
        .classes
        .par_iter()
        .map(|(name, _)| name.clone())
        .collect();

    // Includes
    raw_tokens.extend(
        program
            .includes
            .par_iter()
            .flat_map(|include| {
                let mut result: Vec<(Range, SemanticTokenType, Vec<SemanticTokenModifier>)> =
                    vec![(
                        include.keyword_include_range,
                        SemanticTokenType::KEYWORD,
                        vec![],
                    )];
                if let Some(path_range) = include.path_range {
                    result.push((path_range, SemanticTokenType::STRING, vec![]));
                }

                result
            })
            .collect::<Vec<(Range, SemanticTokenType, Vec<SemanticTokenModifier>)>>(),
    );

    // Classes in parallel
    let class_tokens: Vec<(Range, SemanticTokenType, Vec<SemanticTokenModifier>)> = program
        .classes
        .par_iter()
        .flat_map(|(_, class)| {
            let mut class_raw_tokens = vec![];
            for (modifier, range) in &class.modifiers {
                let (token_type, modifiers) = match modifier {
                    ClassModifier::Obsolete(_) => (
                        SemanticTokenType::MODIFIER,
                        vec![SemanticTokenModifier::DEPRECATED],
                    ),
                    _ => (SemanticTokenType::MODIFIER, vec![]),
                };
                class_raw_tokens.push((*range, token_type, modifiers));
            }

            class_raw_tokens.push((
                class.keyword_class_range,
                SemanticTokenType::KEYWORD,
                vec![],
            ));
            class_raw_tokens.push((
                class.name.range,
                SemanticTokenType::CLASS,
                vec![
                    SemanticTokenModifier::DECLARATION,
                    SemanticTokenModifier::DEFINITION,
                ],
            ));

            if let Some(r) = class.keyword_is_class_range {
                class_raw_tokens.push((r, SemanticTokenType::KEYWORD, vec![]));
            }

            for superclass in &class.superclasses {
                class_raw_tokens.push((superclass.range, SemanticTokenType::CLASS, vec![]));
            }

            for field in class.fields.values() {
                for (modifier, range) in &field.modifiers {
                    let (token_type, modifiers) = match modifier {
                        FieldModifier::Static => (
                            SemanticTokenType::MODIFIER,
                            vec![SemanticTokenModifier::STATIC],
                        ),
                        FieldModifier::Public => (SemanticTokenType::MODIFIER, vec![]),
                        FieldModifier::Define => (
                            SemanticTokenType::MODIFIER,
                            vec![SemanticTokenModifier::READONLY],
                        ),
                        FieldModifier::Obsolete(_) => (
                            SemanticTokenType::MODIFIER,
                            vec![SemanticTokenModifier::DEPRECATED],
                        ),
                    };
                    class_raw_tokens.push((*range, token_type, modifiers));
                }
                // Types
                collect_type_tokens(&field.ty, &mut class_raw_tokens);

                class_raw_tokens.push((
                    field.name.range,
                    SemanticTokenType::PROPERTY,
                    vec![
                        SemanticTokenModifier::DECLARATION,
                        SemanticTokenModifier::DEFINITION,
                    ],
                ));
                if let Some(init) = &field.initializer {
                    collect_expr_tokens(init, &mut class_raw_tokens, &known_classes);
                }
            }

            for methods in class.methods.values() {
                for method in methods {
                    let has_separate_declaration = class
                        .methods
                        .get(&method.name.name)
                        .map(|ms| {
                            ms.iter().any(|m| {
                                m.body.is_none()
                                    && !m.modifiers.iter().any(|(mod_type, _)| {
                                        matches!(mod_type, trainz_ast::gs::MethodModifier::Native)
                                    })
                                    && m.range != method.range
                            })
                        })
                        .unwrap_or(false);

                    collect_method_tokens(
                        &method.modifiers,
                        &method.return_type,
                        &method.name,
                        &method.params,
                        method.body.as_ref().map(|b| &b.statements[..]),
                        &mut class_raw_tokens,
                        &known_classes,
                        has_separate_declaration,
                    );
                }
            }
            class_raw_tokens
        })
        .collect();

    raw_tokens.extend(class_tokens);

    raw_tokens
}
