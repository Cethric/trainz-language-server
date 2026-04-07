pub mod value;

#[cfg(test)]
mod tests;

use crate::soup::value::collect_value_tokens;
use gs_ast::soup::Soup;
use gs_ast::soup::Value;
use gs_diagnostics::soup::Validators;
use tower_lsp_server::ls_types::{Range, SemanticTokenModifier, SemanticTokenType};

pub fn soup_semantic_tokens(
    soup: &Soup,
    validators: Option<&Validators>,
) -> Vec<(Range, SemanticTokenType, Vec<SemanticTokenModifier>)> {
    let mut raw_tokens: Vec<(Range, SemanticTokenType, Vec<SemanticTokenModifier>)> = vec![];

    let kind_kv = soup
        .key_value_pairs
        .iter()
        .find(|kv| kv.key.eq_ignore_ascii_case("kind"));
    let validator = kind_kv.and_then(|kv| match &kv.value {
        Some(Value::String(s, _)) | Some(Value::Variable(s, _)) => validators.and_then(|vs| {
            vs.containers
                .iter()
                .find(|v| v.container_name.eq_ignore_ascii_case(s))
        }),
        _ => None,
    });

    for kv in &soup.key_value_pairs {
        let rule = validator.and_then(|v| {
            v.rules
                .iter()
                .find(|r| r.key.eq_ignore_ascii_case(&kv.key))
                .or_else(|| {
                    v.subpossibilities
                        .iter()
                        .find(|r| r.key.eq_ignore_ascii_case(&kv.key))
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
                            .iter()
                            .find(|v| v.container_name.eq_ignore_ascii_case(type_name))
                    })
                })
                .or_else(|| {
                    validator
                        .and_then(|v| v.array_element.as_ref())
                        .and_then(|type_name| {
                            validators.and_then(|vs| {
                                vs.containers
                                    .iter()
                                    .find(|v| v.container_name.eq_ignore_ascii_case(type_name))
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
