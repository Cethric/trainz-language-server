use tower_lsp_server::ls_types::{Range, SemanticTokenModifier, SemanticTokenType};
use trainz_ast::acs_text::Value;

#[tracing::instrument(skip(value, raw_tokens))]
pub fn collect_value_tokens(
    value: &Value,
    raw_tokens: &mut Vec<(Range, SemanticTokenType, Vec<SemanticTokenModifier>)>,
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
                let modifiers = vec![];

                raw_tokens.push((kv.key_range, SemanticTokenType::KEYWORD, modifiers));
                if let Some(v) = &kv.value {
                    collect_value_tokens(v, raw_tokens);
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

        collect_value_tokens(&value, &mut raw_tokens);
    }
}
