mod error;
pub mod grammar;

use error::ParseError;
use grammar::{AuranConfigAcsTextParser, Rule};
use pest::Parser;
use pest::iterators::Pairs;
use shadow_rs::shadow;

shadow!(build);

#[tracing::instrument(skip(src))]
pub fn parse_acs_text(src: &'_ str) -> Result<Pairs<'_, Rule>, ParseError> {
    match AuranConfigAcsTextParser::parse(Rule::acs_text, src) {
        Ok(pairs) => {
            // trace!("Parsing successful: {:#?}", pairs);
            Ok(pairs)
        }
        Err(e) => {
            // error!("Parsing error: {:#?}", e);
            Err(e.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_string_table_issue() {
        let input = r#"string-table
{
  description                           "Unload train vehicles at current industry location."
  msg_error_industry_not_found          "Issue a 'Drive To' command prior to the 'Unload' command."
  driver_command_unload                 "Unload"
  tt_unload_at                          "Unload at $0"
}"#;

        let result = parse_acs_text(input);
        assert!(
            result.is_ok(),
            "Failed to parse acs_text: {:?}",
            result.err()
        );
    }

    fn assert_parse_acs_text_input(input: &str) {
        let result = parse_acs_text(input);
        assert!(
            result.is_ok(),
            "Failed to parse acs_text input: {:?}",
            input
        );
        let pairs: Vec<_> = result.unwrap().collect();
        assert!(
            pairs
                .iter()
                .any(|pair| pair.as_rule() == Rule::key_value_pair),
            "Expected at least one key_value_pair for input: {:?}, got: {:?}",
            input,
            pairs
        );
    }

    #[test]
    fn test_parse_acs_text_string_value() {
        assert_parse_acs_text_input("name value");
    }

    #[test]
    fn test_parse_acs_text_numeric_value() {
        assert_parse_acs_text_input("age 123");
    }

    #[test]
    fn test_parse_acs_text_float_value() {
        assert_parse_acs_text_input("pi 3.14");
    }

    #[test]
    fn test_parse_acs_text_hex_value() {
        assert_parse_acs_text_input("color 0xFF00FF");
    }

    #[test]
    fn test_parse_acs_text_quoted_string_value() {
        assert_parse_acs_text_input("msg \"hello\"");
    }

    #[test]
    fn test_acs_text_error_recovery() {
        let input = r#"
            name "value"
            !!! garbage !!!
            age 123
            container {
                !!! garbage in container !!!
                sub "item"
            }
            !!! more garbage !!!
            kuid <KUID:-3:1011>
        "#;
        let result = parse_acs_text(input);
        assert!(
            result.is_ok(),
            "Failed to parse acs_text with error recovery: {:?}",
            result.err()
        );
        let pairs: Vec<_> = result.unwrap().collect();

        let keys: Vec<_> = pairs
            .iter()
            .filter(|p| p.as_rule() == Rule::key_value_pair)
            .map(|p| p.clone().into_inner().next().unwrap().as_str())
            .collect();

        assert!(keys.contains(&"name"));
        assert!(keys.contains(&"age"));
        assert!(keys.contains(&"container"));
        assert!(keys.contains(&"kuid"));
    }
}

// todo partial parse
