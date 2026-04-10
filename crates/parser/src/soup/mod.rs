mod error;
pub mod grammar;

use error::ParseError;
use grammar::{AuranConfigSoupParser, Rule};
use pest::Parser;
use pest::iterators::Pairs;
use shadow_rs::shadow;

shadow!(build);

pub fn parse_soup(src: &'_ str) -> Result<Pairs<'_, Rule>, ParseError> {
    match AuranConfigSoupParser::parse(Rule::soup, src) {
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

        let result = parse_soup(input);
        assert!(result.is_ok(), "Failed to parse soup: {:?}", result.err());
    }

    fn assert_parse_soup_input(input: &str) {
        let result = parse_soup(input);
        assert!(result.is_ok(), "Failed to parse soup input: {:?}", input);
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
    fn test_parse_soup_string_value() {
        assert_parse_soup_input("name value");
    }

    #[test]
    fn test_parse_soup_numeric_value() {
        assert_parse_soup_input("age 123");
    }

    #[test]
    fn test_parse_soup_float_value() {
        assert_parse_soup_input("pi 3.14");
    }

    #[test]
    fn test_parse_soup_hex_value() {
        assert_parse_soup_input("color 0xFF00FF");
    }

    #[test]
    fn test_parse_soup_quoted_string_value() {
        assert_parse_soup_input("msg \"hello\"");
    }
}

// todo partial parse
