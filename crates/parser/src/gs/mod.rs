mod error;
pub mod grammar;
mod pratt;

use error::ParseError;
use grammar::{GameScriptParser, Rule};
use pest::iterators::Pairs;
use pest::Parser;

pub fn parse(src: &'_ str) -> Result<Pairs<'_, Rule>, ParseError> {
    match GameScriptParser::parse(Rule::program, src) {
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
