mod error;
pub mod grammar;

use error::ParseError;
use grammar::{AuranConfigSoupParser, Rule};
use pest::iterators::Pairs;
use pest::Parser;
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

    #[test]
    fn test_parse_soup_multi_char_value() {
        let inputs = vec![
            "name value",
            "age 123",
            "pi 3.14",
            "color 0xFF00FF",
            "msg \"hello\"",
        ];
        for input in inputs {
            println!("Testing input: {:?}", input);
            let result = parse_soup(input).unwrap();
            let pairs: Vec<_> = result.collect();
            for pair in &pairs {
                if pair.as_rule() == Rule::key_value_pair {
                    println!("Rule: {:?}, Content: {:?}", pair.as_rule(), pair.as_str());
                    let mut inner = pair.clone().into_inner();
                    let key = inner.next().unwrap();
                    println!("  Key: {:?}, Content: {:?}", key.as_rule(), key.as_str());
                    if let Some(value) = inner.next() {
                        println!(
                            "  Value: {:?}, Content: {:?}",
                            value.as_rule(),
                            value.as_str()
                        );
                        let mut value_inner = value.into_inner();
                        if let Some(v) = value_inner.next() {
                            println!("    V: {:?}, Content: {:?}", v.as_rule(), v.as_str());
                        }
                    }
                }
            }
            println!("---");
        }
    }
}

// todo partial parse
