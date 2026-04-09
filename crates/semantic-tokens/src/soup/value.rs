use rayon::prelude::*;
use tower_lsp_server::ls_types::{Range, SemanticTokenModifier, SemanticTokenType};
use trainz_ast::soup::Value;
use trainz_soup_validators::{ArrayElementType, ContainerValidator, Validators};

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
            let start_offset = get_offset(src, range.start);
            let end_offset = get_offset(src, range.end);
            let text = &src[start_offset..end_offset];

            if range.start.line == range.end.line {
                let len: u32 = text.chars().map(|c| c.len_utf16() as u32).sum();
                let r = Range {
                    start: range.start,
                    end: tower_lsp_server::ls_types::Position {
                        line: range.start.line,
                        character: range.start.character + len,
                    },
                };
                raw_tokens.push((r, SemanticTokenType::STRING, vec![]));
            } else {
                // Split multi-line string
                let mut current_line = range.start.line;
                let mut current_char = range.start.character;

                let lines = text.split('\n');
                for (i, line) in lines.enumerate() {
                    let line_trimmed = line.trim_end_matches('\r');
                    let len: u32 = line_trimmed.chars().map(|c| c.len_utf16() as u32).sum();

                    if len > 0 || i == 0 {
                        let r = Range {
                            start: tower_lsp_server::ls_types::Position {
                                line: current_line,
                                character: current_char,
                            },
                            end: tower_lsp_server::ls_types::Position {
                                line: current_line,
                                character: current_char + len,
                            },
                        };
                        raw_tokens.push((r, SemanticTokenType::STRING, vec![]));
                    }

                    current_line += 1;
                    current_char = 0;
                }
            }
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

fn get_offset(src: &str, pos: tower_lsp_server::ls_types::Position) -> usize {
    let mut line = 0;
    let mut character = 0;

    for (offset, c) in src.char_indices() {
        if line == pos.line as usize && character == pos.character as usize {
            return offset;
        }

        if c == '\n' {
            line += 1;
            character = 0;
        } else {
            character += c.len_utf16() as usize;
        }
    }
    src.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower_lsp_server::ls_types::{Position, Range};
    use trainz_ast::soup::Value;

    #[test]
    fn test_collect_value_tokens_utf8_panic() {
        let src = "multi-line\nstring with é and е:\nsecond line";
        let range = Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 2,
                character: 5,
            },
        };
        let value = Value::String(
            "multi-line\nstring with é and е:\nsecond line".to_string(),
            range,
        );
        let mut raw_tokens = Vec::new();

        // This should not panic
        println!(
            "Start offset: {}, End offset: {}",
            get_offset(src, range.start),
            get_offset(src, range.end)
        );
        collect_value_tokens(&value, &mut raw_tokens, None, None, src);
    }

    #[test]
    fn test_get_offset_with_surrogate_pair() {
        let src = "💩a";
        // '💩' is U+1F4A9, which takes 2 UTF-16 units.
        // 'a' is at UTF-16 offset 2.
        let pos = Position {
            line: 0,
            character: 2,
        };
        let offset = get_offset(src, pos);
        assert_eq!(offset, 4); // '💩' is 4 bytes in UTF-8
    }
}
