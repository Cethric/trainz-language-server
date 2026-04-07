use crate::soup::process::kuid::process_kuid;
use crate::soup::process::process_key_value_pair;
use crate::soup::{NumericValue, Value};
use gs_parser::soup::grammar::Rule;
use gs_util::range::pair_to_range;
use pest::iterators::Pair;

pub fn process_value(pair: Pair<Rule>) -> Value {
    let range = pair_to_range(&pair);
    let inner = pair
        .into_inner()
        .find(|p| p.as_rule() != Rule::double_quote)
        .unwrap();
    process_value_inner(inner, range)
}

fn process_value_inner(inner: Pair<Rule>, range: tower_lsp_server::ls_types::Range) -> Value {
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
            process_value_inner(num_inner, range)
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
            let pairs = inner
                .into_inner()
                .filter(|p| p.as_rule() == Rule::key_value_pair)
                .map(process_key_value_pair)
                .collect();
            Value::Container(pairs, range)
        }
        _ => unreachable!("Unexpected rule in value: {:?}", inner.as_rule()),
    }
}

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
    use gs_parser::soup::grammar::{AuranConfigSoupParser, Rule};
    use pest::Parser;

    #[test]
    fn test_process_numeric_value_hex() {
        let hex_strs = ["0x1A", "0XFF", "0x0", "0X1234abcd"];
        for s in hex_strs {
            let pair = AuranConfigSoupParser::parse(Rule::hex, s)
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
            let pair = AuranConfigSoupParser::parse(Rule::float, s)
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
}
