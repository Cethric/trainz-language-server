use rayon::prelude::*;
use tower_lsp_server::ls_types::{Range, SemanticTokenModifier, SemanticTokenType};
use trainz_acs_text_validators::{ArrayElementType, ContainerValidator, Validators};
use trainz_ast::acs_text::Value;

#[tracing::instrument(skip(value, raw_tokens, validator, all_validators))]
pub fn collect_value_tokens(
    value: &Value,
    raw_tokens: &mut Vec<(Range, SemanticTokenType, Vec<SemanticTokenModifier>)>,
    validator: Option<&ContainerValidator>,
    all_validators: Option<&Validators>,
) {
    match value {
        Value::Numeric(_nv, range) => {
            raw_tokens.push((*range, SemanticTokenType::NUMBER, vec![]));
        }
        Value::String(_s, range) => {
            raw_tokens.push((*range, SemanticTokenType::STRING, vec![]));
        }
        Value::Kuid(_k, range) => {
            raw_tokens.push((
                *range,
                SemanticTokenType::PROPERTY,
                vec![SemanticTokenModifier::READONLY],
            ));
        }
        Value::Container(kv_pairs, _range, _) => {
            for (index, kv) in kv_pairs.iter().enumerate() {
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
                if let Some(v) = &kv.value {
                    let inner_validator = if let Some(rule) = rule {
                        if let Some(child_validator) = &rule.child_validator {
                            Some(child_validator.as_ref())
                        } else if let Some(type_name) = &rule.type_name {
                            all_validators
                                .and_then(|vs| vs.container_map.get(&type_name.to_lowercase()))
                        } else {
                            None
                        }
                    } else if let Some(v) = validator
                        && let Some(ae) = &v.array_element
                    {
                        match ae {
                            ArrayElementType::Array(_, s) => all_validators
                                .and_then(|vs| vs.container_map.get(&s.to_lowercase())),
                            ArrayElementType::Tuple(types) => kv
                                .key
                                .parse::<usize>()
                                .ok()
                                .and_then(|idx| types.get(idx))
                                .and_then(|(_, tn)| {
                                    all_validators
                                        .and_then(|vs| vs.container_map.get(&tn.to_lowercase()))
                                }),
                            ArrayElementType::Inline(iv) => Some(iv.as_ref()),
                            ArrayElementType::Rule(r) => {
                                if let Some(cv) = &r.child_validator {
                                    Some(cv.as_ref())
                                } else {
                                    r.type_name.as_ref().and_then(|tn| {
                                        all_validators
                                            .and_then(|vs| vs.container_map.get(&tn.to_lowercase()))
                                    })
                                }
                            }
                        }
                    } else if let Some(v) = validator
                        && let Some(tag_array) = &v.tag_array
                    {
                        match tag_array {
                            ArrayElementType::Array(_, s) => all_validators
                                .and_then(|vs| vs.container_map.get(&s.to_lowercase())),
                            ArrayElementType::Tuple(types) => {
                                types.get(index).and_then(|(_, tn)| {
                                    all_validators
                                        .and_then(|vs| vs.container_map.get(&tn.to_lowercase()))
                                })
                            }
                            ArrayElementType::Inline(iv) => Some(iv.as_ref()),
                            ArrayElementType::Rule(r) => {
                                if let Some(cv) = &r.child_validator {
                                    Some(cv.as_ref())
                                } else {
                                    r.type_name.as_ref().and_then(|tn| {
                                        all_validators
                                            .and_then(|vs| vs.container_map.get(&tn.to_lowercase()))
                                    })
                                }
                            }
                        }
                    } else {
                        None
                    };

                    collect_value_tokens(v, raw_tokens, inner_validator, all_validators);
                }
            }
        }
        Value::Array(_values, range) => {
            raw_tokens.push((*range, SemanticTokenType::NUMBER, vec![])); // Or loop through values?
        }
        Value::Variable(_v, range) => {
            raw_tokens.push((*range, SemanticTokenType::VARIABLE, vec![]));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower_lsp_server::ls_types::{Position, Range};
    use trainz_ast::acs_text::Value;

    #[test]
    fn test_collect_value_tokens_no_panic() {
        let range = Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 1,
                character: 5,
            },
        };
        let value = Value::String("multi-line\nstring".to_string(), range);
        let mut raw_tokens = Vec::new();

        collect_value_tokens(&value, &mut raw_tokens, None, None);
    }
}
