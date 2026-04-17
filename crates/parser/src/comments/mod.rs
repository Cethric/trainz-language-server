mod error;
pub mod grammar;

use error::ParseError;
use grammar::acs_text::{AcsTextCommentsParser, Rule as AcsTextRule};
use grammar::gs::{GsCommentsParser, Rule as GsRule};
use pest::Parser;
use pest::iterators::Pairs;
use shadow_rs::shadow;

shadow!(build);

#[tracing::instrument(skip(src))]
pub fn parse_gs_comments(src: &'_ str) -> Result<Pairs<'_, GsRule>, ParseError> {
    match GsCommentsParser::parse(GsRule::comment_program, src) {
        Ok(pairs) => Ok(pairs),
        Err(e) => Err(e.into()),
    }
}

#[tracing::instrument(skip(src))]
pub fn parse_acs_text_comments(src: &'_ str) -> Result<Pairs<'_, AcsTextRule>, ParseError> {
    match AcsTextCommentsParser::parse(AcsTextRule::comment_program, src) {
        Ok(pairs) => Ok(pairs),
        Err(e) => Err(e.into()),
    }
}
