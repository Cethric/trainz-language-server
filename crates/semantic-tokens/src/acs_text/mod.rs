pub mod value;

#[cfg(test)]
mod tests;

use crate::acs_text::value::collect_value_tokens;
use rayon::prelude::*;
use tower_lsp_server::ls_types::{Range, SemanticTokenModifier, SemanticTokenType};
use trainz_acs_text_validators::{ArrayElementType, Validators};
use trainz_ast::acs_text::Value;
use trainz_ast::acs_text::base::AcsText;

#[tracing::instrument(skip(acs_text, validators))]
pub fn acs_text_semantic_tokens(
    acs_text: &AcsText,
    validators: Option<&Validators>,
) -> Vec<(Range, SemanticTokenType, Vec<SemanticTokenModifier>)> {
    let mut raw_tokens: Vec<(Range, SemanticTokenType, Vec<SemanticTokenModifier>)> = vec![];

    let kind_kv = acs_text
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

    for (index, kv) in acs_text.key_value_pairs.iter().enumerate() {
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
            let inner_validator = if let Some(rule) = rule {
                if let Some(child_validator) = &rule.child_validator {
                    Some(child_validator.as_ref())
                } else if let Some(type_name) = &rule.type_name {
                    validators.and_then(|vs| vs.container_map.get(&type_name.to_lowercase()))
                } else {
                    None
                }
            } else if let Some(v) = validator
                && let Some(ae) = &v.array_element
            {
                match ae {
                    ArrayElementType::Array(_, s) => {
                        validators.and_then(|vs| vs.container_map.get(&s.to_lowercase()))
                    }
                    ArrayElementType::Tuple(types) => kv
                        .key
                        .parse::<usize>()
                        .ok()
                        .and_then(|idx| types.get(idx))
                        .and_then(|(_, tn)| {
                            validators.and_then(|vs| vs.container_map.get(&tn.to_lowercase()))
                        }),
                    ArrayElementType::Inline(iv) => Some(iv.as_ref()),
                    ArrayElementType::Rule(r) => {
                        if let Some(cv) = &r.child_validator {
                            Some(cv.as_ref())
                        } else {
                            r.type_name.as_ref().and_then(|tn| {
                                validators.and_then(|vs| vs.container_map.get(&tn.to_lowercase()))
                            })
                        }
                    }
                }
            } else if let Some(v) = validator
                && let Some(tag_array) = &v.tag_array
            {
                match tag_array {
                    ArrayElementType::Array(_, s) => {
                        validators.and_then(|vs| vs.container_map.get(&s.to_lowercase()))
                    }
                    ArrayElementType::Tuple(types) => types.get(index).and_then(|(_, tn)| {
                        validators.and_then(|vs| vs.container_map.get(&tn.to_lowercase()))
                    }),
                    ArrayElementType::Inline(iv) => Some(iv.as_ref()),
                    ArrayElementType::Rule(r) => {
                        if let Some(cv) = &r.child_validator {
                            Some(cv.as_ref())
                        } else {
                            r.type_name.as_ref().and_then(|tn| {
                                validators.and_then(|vs| vs.container_map.get(&tn.to_lowercase()))
                            })
                        }
                    }
                }
            } else {
                None
            };

            collect_value_tokens(value, &mut raw_tokens, inner_validator, validators);
        }
    }

    raw_tokens
}
