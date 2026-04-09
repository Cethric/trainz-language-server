mod error;
pub mod grammar;

use error::ParseError;
use grammar::gs::{GsCommentsParser, Rule as GsRule};
use grammar::soup::{Rule as SoupRule, SoupCommentsParser};
use pest::iterators::Pairs;
use pest::Parser;
use shadow_rs::shadow;

shadow!(build);

pub fn parse_gs_comments(src: &'_ str) -> Result<Pairs<'_, GsRule>, ParseError> {
    match GsCommentsParser::parse(GsRule::comment_program, src) {
        Ok(pairs) => Ok(pairs),
        Err(e) => Err(e.into()),
    }
}

pub fn parse_soup_comments(src: &'_ str) -> Result<Pairs<'_, SoupRule>, ParseError> {
    match SoupCommentsParser::parse(SoupRule::comment_program, src) {
        Ok(pairs) => Ok(pairs),
        Err(e) => Err(e.into()),
    }
}
