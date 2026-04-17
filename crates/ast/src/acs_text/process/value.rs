use crate::acs_text::process::kuid::process_kuid;
use crate::acs_text::process::process_key_value_pair;
use crate::acs_text::{NumericValue, Value};
use pest::iterators::Pair;
use trainz_common::range::{pair_to_range, pos_to_range};
use trainz_parser::acs_text::grammar::Rule;

#[tracing::instrument(skip(pair))]
pub fn process_value(pair: Pair<Rule>) -> Value {
    let inner = pair
        .into_inner()
        .find(|p| p.as_rule() != Rule::double_quote)
        .unwrap();
    process_value_inner(inner)
}

#[tracing::instrument(skip(inner))]
fn process_value_inner(inner: Pair<Rule>) -> Value {
    let inner_range = pair_to_range(&inner);
    match inner.as_rule() {
        Rule::array_value => {
            let nums = inner
                .into_inner()
                .map(process_numeric_value)
                .collect::<Vec<_>>();
            Value::Array(nums, inner_range)
        }
        Rule::numeric_value => {
            let num_inner = inner.into_inner().next().unwrap();
            process_value_inner(num_inner)
        }
        Rule::float => {
            let s = inner.as_str();
            let s = if s.ends_with('f') || s.ends_with('F') {
                &s[..s.len() - 1]
            } else {
                s
            };
            Value::Numeric(NumericValue::Float(s.parse().unwrap()), inner_range)
        }
        Rule::hex => Value::Numeric(
            NumericValue::Hex(
                u64::from_str_radix(
                    inner
                        .as_str()
                        .trim_start_matches("0x")
                        .trim_start_matches("0X"),
                    16,
                )
                .unwrap(),
            ),
            inner_range,
        ),
        Rule::integer => Value::Numeric(
            NumericValue::Int(inner.as_str().parse().unwrap()),
            inner_range,
        ),
        Rule::string_value => {
            let string_inner = inner
                .into_inner()
                .find(|p| p.as_rule() == Rule::string)
                .unwrap();
            Value::String(string_inner.as_str().to_string(), inner_range)
        }
        Rule::variable_value => Value::Variable(inner.as_str().to_string(), inner_range),
        Rule::kuid_value => Value::Kuid(process_kuid(inner), inner_range),
        Rule::container_value => {
            let brace_open_pair = inner
                .clone()
                .into_inner()
                .find(|p| p.as_rule() == Rule::brace_open)
                .expect("container_value must have brace_open");
            let brace_close_pair = inner
                .clone()
                .into_inner()
                .filter(|p| p.as_rule() == Rule::brace_close)
                .last()
                .expect("container_value must have brace_close");

            let brace_open_start = brace_open_pair.as_span().start_pos();
            let brace_open_end = brace_open_pair.as_span().end_pos();
            let brace_close_start = brace_close_pair.as_span().start_pos();
            let brace_close_end = brace_close_pair.as_span().end_pos();

            let full_range = pos_to_range(&brace_open_start, &brace_close_end);
            let content_range = pos_to_range(&brace_open_end, &brace_close_start);

            let pairs = inner
                .into_inner()
                .filter(|p| p.as_rule() == Rule::key_value_pair)
                .map(process_key_value_pair)
                .collect();
            Value::Container(pairs, content_range, full_range)
        }
        _ => unreachable!("Unexpected rule in value: {:?}", inner.as_rule()),
    }
}

#[tracing::instrument(skip(pair))]
pub fn process_numeric_value(pair: Pair<Rule>) -> NumericValue {
    let pair = if pair.as_rule() == Rule::numeric_value {
        pair.into_inner().next().unwrap()
    } else {
        pair
    };
    match pair.as_rule() {
        Rule::float => {
            let s = pair.as_str();
            let s = if s.ends_with('f') || s.ends_with('F') {
                &s[..s.len() - 1]
            } else {
                s
            };
            NumericValue::Float(s.parse().unwrap())
        }
        Rule::hex => NumericValue::Hex(
            u64::from_str_radix(
                pair.as_str()
                    .trim_start_matches("0x")
                    .trim_start_matches("0X"),
                16,
            )
            .unwrap(),
        ),
        Rule::integer => NumericValue::Int(pair.as_str().parse().unwrap()),
        _ => unreachable!("Unexpected rule in numeric_value: {:?}", pair.as_rule()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pest::Parser;
    use trainz_parser::acs_text::grammar::{AuranConfigAcsTextParser, Rule};

    #[test]
    fn test_process_numeric_value_hex() {
        let hex_strs = ["0x1A", "0XFF", "0x0", "0X1234abcd"];
        for s in hex_strs {
            let pair = AuranConfigAcsTextParser::parse(Rule::hex, s)
                .unwrap()
                .next()
                .unwrap();
            let val = process_numeric_value(pair);
            match val {
                NumericValue::Hex(v) => {
                    let expected = u64::from_str_radix(&s[2..], 16).unwrap();
                    assert_eq!(v, expected);
                }
                _ => panic!("Expected Hex"),
            }
        }
    }

    #[test]
    fn test_process_numeric_value_float() {
        let float_strs = ["1.23", "0.0", "-4.56"];
        for s in float_strs {
            let pair = AuranConfigAcsTextParser::parse(Rule::float, s)
                .unwrap()
                .next()
                .unwrap();
            let val = process_numeric_value(pair);
            match val {
                NumericValue::Float(v) => {
                    let expected: f64 = s.parse().unwrap();
                    assert_eq!(v, expected);
                }
                _ => panic!("Expected Float"),
            }
        }
    }

    #[test]
    fn test_container_value_range() {
        let input = "\n\n{\n  key value\n}\n\n";
        let pair = AuranConfigAcsTextParser::parse(Rule::value, input)
            .unwrap()
            .next()
            .unwrap();

        let value = process_value(pair);
        if let Value::Container(_, range, _) = value {
            // Should be the range of the container value, not including leading/trailing newlines if they are outside
            // Actually the grammar includes them!
            // container_value = { (newline* ~ brace_open ~ newline* ~ brace_close ~ newline*) ... }

            // If the parser matched them, they are part of the span.

            // Let's see what the range is.
            // println!("Range: {:?}", range);

            // Assuming we want the range to be from '{' to '}'
            // Line 2: {
            // Line 3:   key value
            // Line 4: }

            assert_eq!(range.start.line, 2);
            assert_eq!(range.start.character, 1);
            assert_eq!(range.end.line, 4);
            assert_eq!(range.end.character, 0);
        } else {
            panic!("Expected Container");
        }
    }
}
