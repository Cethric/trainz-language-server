pub mod value;

#[cfg(test)]
mod tests;

use crate::soup::value::collect_value_tokens;
use rayon::prelude::*;
use tower_lsp_server::ls_types::{Range, SemanticTokenModifier, SemanticTokenType};
use trainz_ast::soup::Value;
use trainz_ast::soup::base::Soup;
use trainz_soup_validators::{ArrayElementType, Validators};

#[tracing::instrument]
pub fn soup_semantic_tokens(
    soup: &Soup,
    validators: Option<&Validators>,
) -> Vec<(Range, SemanticTokenType, Vec<SemanticTokenModifier>)> {
    let mut raw_tokens: Vec<(Range, SemanticTokenType, Vec<SemanticTokenModifier>)> = vec![];

    let kind_kv = soup
        .key_value_pairs
        .par_iter()
        .find_first(|kv| kv.key.eq_ignore_ascii_case("kind"));
    let validator = kind_kv.and_then(|kv| match &kv.value {
        Some(Value::String(s, _)) | Some(Value::Variable(s, _)) => validators.and_then(|vs| {
            vs.containers
                .par_iter()
                .find_first(|v| v.container_name.eq_ignore_ascii_case(s))
        }),
        _ => None,
    });

    for kv in &soup.key_value_pairs {
        let rule = validator.and_then(|v| {
            v.rules
                .par_iter()
                .find_first(|r| r.key.eq_ignore_ascii_case(&kv.key))
                .or_else(|| {
                    v.sub_possibilities
                        .par_iter()
                        .find_first(|r| r.key.eq_ignore_ascii_case(&kv.key))
                })
        });
        let mut modifiers = vec![];
        if rule.and_then(|r| r.obsolete_tag).unwrap_or(false) {
            modifiers.push(SemanticTokenModifier::DEPRECATED);
        }

        raw_tokens.push((kv.key_range, SemanticTokenType::KEYWORD, modifiers));
        if let Some(value) = &kv.value {
            let inner_validator = rule
                .and_then(|r| r.type_name.as_ref())
                .and_then(|type_name| {
                    validators.and_then(|vs| {
                        vs.containers
                            .par_iter()
                            .find_first(|v| v.container_name.eq_ignore_ascii_case(type_name))
                    })
                })
                .or_else(|| {
                    validator
                        .and_then(|v| v.array_element.as_ref())
                        .and_then(|ae| {
                            let tn = match ae {
                                ArrayElementType::Array(s) => Some(s),
                                ArrayElementType::Tuple(types) => {
                                    kv.key.parse::<usize>().ok().and_then(|idx| types.get(idx))
                                }
                            };
                            tn.and_then(|tn| {
                                validators.and_then(|vs| {
                                    vs.containers
                                        .par_iter()
                                        .find_first(|v| v.container_name.eq_ignore_ascii_case(tn))
                                })
                            })
                        })
                });

            collect_value_tokens(
                value,
                &mut raw_tokens,
                inner_validator,
                validators,
                &soup.src,
            );
        }
    }

    raw_tokens
}
