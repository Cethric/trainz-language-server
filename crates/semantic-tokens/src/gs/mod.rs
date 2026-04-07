pub mod expr;
pub mod method;
pub mod stmt;
pub mod types;

#[cfg(test)]
mod tests;

use crate::gs::expr::collect_expr_tokens;
use crate::gs::method::collect_method_tokens;
use crate::gs::types::collect_type_tokens;
use gs_ast::gs::{ClassModifier, FieldModifier, Program};
use rayon::prelude::*;
use tower_lsp_server::ls_types::{Range, SemanticTokenModifier, SemanticTokenType};

pub fn semantic_tokens(
    program: &Program,
) -> Vec<(Range, SemanticTokenType, Vec<SemanticTokenModifier>)> {
    // Collect all tokens with their type and modifiers
    let mut raw_tokens: Vec<(Range, SemanticTokenType, Vec<SemanticTokenModifier>)> = vec![];

    // Gather all known class names
    let known_classes: std::collections::HashSet<String> = program
        .classes
        .iter()
        .map(|c| c.name.name.clone())
        .collect();

    // Includes
    raw_tokens.extend(
        program
            .includes
            .par_iter()
            .flat_map(|include| {
                let mut result: Vec<(Range, SemanticTokenType, Vec<SemanticTokenModifier>)> =
                    vec![(include.range, SemanticTokenType::KEYWORD, vec![])];
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
        .flat_map(|class| {
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

            for field in &class.fields {
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

                for name in &field.names {
                    class_raw_tokens.push((
                        name.range,
                        SemanticTokenType::PROPERTY,
                        vec![
                            SemanticTokenModifier::DECLARATION,
                            SemanticTokenModifier::DEFINITION,
                        ],
                    ));
                }
                for init in &field.initializers {
                    collect_expr_tokens(init, &mut class_raw_tokens, &known_classes);
                }
            }

            for method in &class.methods {
                collect_method_tokens(
                    &method.modifiers,
                    &method.return_type,
                    &method.name,
                    &method.params,
                    Some(&method.body.statements),
                    &mut class_raw_tokens,
                    &known_classes,
                );
            }

            for method in &class.native_methods {
                collect_method_tokens(
                    &method.modifiers,
                    &method.return_type,
                    &method.name,
                    &method.params,
                    None,
                    &mut class_raw_tokens,
                    &known_classes,
                );
            }
            class_raw_tokens
        })
        .collect();

    raw_tokens.extend(class_tokens);

    raw_tokens
}
