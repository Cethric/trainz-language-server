use gs_ast::soup::Value;
use gs_diagnostics::soup::{ContainerValidator, Validators};
use tower_lsp_server::ls_types::{Range, SemanticTokenModifier, SemanticTokenType};

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
            if range.start.line == range.end.line {
                raw_tokens.push((*range, SemanticTokenType::STRING, vec![]));
            } else {
                // Split multi-line string
                let start_offset = get_offset(src, range.start);
                let end_offset = get_offset(src, range.end);
                let text = &src[start_offset..end_offset];

                let mut current_line = range.start.line;
                let mut current_char = range.start.character;

                let lines = text.split('\n');
                for (i, line) in lines.enumerate() {
                    let line_trimmed = line.trim_end_matches('\r');
                    let len = line_trimmed.len() as u32;

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
        Value::Container(kv_pairs, _range) => {
            for kv in kv_pairs {
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
                if let Some(v) = &kv.value {
                    let inner_validator = rule
                        .and_then(|r| r.type_name.as_ref())
                        .and_then(|type_name| {
                            all_validators.and_then(|vs| {
                                vs.containers
                                    .iter()
                                    .find(|v| v.container_name.eq_ignore_ascii_case(type_name))
                            })
                        })
                        .or_else(|| {
                            validator
                                .and_then(|v| v.array_element.as_ref())
                                .and_then(|type_name| {
                                    all_validators.and_then(|vs| {
                                        vs.containers.iter().find(|v| {
                                            v.container_name.eq_ignore_ascii_case(type_name)
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
    let mut offset = 0;
    for (i, line) in src.lines().enumerate() {
        if i == pos.line as usize {
            return offset + pos.character as usize;
        }
        offset += line.len() + 1; // +1 for \n
    }
    offset
}
