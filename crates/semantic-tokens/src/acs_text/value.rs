use rayon::prelude::*;
use tower_lsp_server::ls_types::{Range, SemanticTokenModifier, SemanticTokenType};
use trainz_acs_text_validators::{ArrayElementType, ContainerValidator, Validators};
use trainz_ast::acs_text::Value;

#[tracing::instrument]
pub fn collect_value_tokens(
    value: &Value,
    raw_tokens: &mut Vec<(Range, SemanticTokenType, Vec<SemanticTokenModifier>)>,
    validator: Option<&ContainerValidator>,
    all_validators: Option<&Validators>,
    src: &str,
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
            for kv in kv_pairs {
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
                    let inner_validator = rule
                        .and_then(|r| r.type_name.as_ref())
                        .and_then(|type_name| {
                            all_validators.and_then(|vs| {
                                vs.containers.par_iter().find_first(|v| {
                                    v.container_name.eq_ignore_ascii_case(type_name)
                                })
                            })
                        })
                        .or_else(|| {
                            validator
                                .and_then(|v| v.array_element.as_ref())
                                .and_then(|ae| {
                                    let tn = match ae {
                                        ArrayElementType::Array(s) => Some(s),
                                        ArrayElementType::Tuple(types) => kv
                                            .key
                                            .parse::<usize>()
                                            .ok()
                                            .and_then(|idx| types.get(idx)),
                                    };
                                    tn.and_then(|tn| {
                                        all_validators.and_then(|vs| {
                                            vs.containers.par_iter().find_first(|v| {
                                                v.container_name.eq_ignore_ascii_case(tn)
                                            })
                                        })
                                    })
                                })
                        });

                    collect_value_tokens(v, raw_tokens, inner_validator, all_validators, src);
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
        let _src = "multi-line\nstring";
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

        collect_value_tokens(&value, &mut raw_tokens, None, None, _src);
    }
}
