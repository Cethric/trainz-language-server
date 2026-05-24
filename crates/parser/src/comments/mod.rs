mod error;
pub mod grammar;

use error::ParseError;
use grammar::acs_text::{AcsTextCommentsParser, Rule as AcsTextRule};
use grammar::gs::{GsCommentsParser, Rule as GsRule};
use pest::Parser;
use pest::iterators::Pairs;
use shadow_rs::shadow;

shadow!(build);

/// Parses Game Script comments.
///
/// # Examples
///
/// ```rust
/// use trainz_parser::comments::parse_gs_comments;
///
/// let input = "// This is a comment";
/// let result = parse_gs_comments(input);
/// assert!(result.is_ok());
/// ```
#[tracing::instrument(skip(src))]
pub fn parse_gs_comments(src: &'_ str) -> Result<Pairs<'_, GsRule>, ParseError> {
    match GsCommentsParser::parse(GsRule::comment_program, src) {
        Ok(pairs) => Ok(pairs),
        Err(e) => Err(e.into()),
    }
}

/// Parses AcsText comments.
///
/// # Examples
///
/// ```rust
/// use trainz_parser::comments::parse_acs_text_comments;
///
/// let input = "// This is an AcsText comment";
/// let result = parse_acs_text_comments(input);
/// assert!(result.is_ok());
/// ```
#[tracing::instrument(skip(src))]
pub fn parse_acs_text_comments(src: &'_ str) -> Result<Pairs<'_, AcsTextRule>, ParseError> {
    match AcsTextCommentsParser::parse(AcsTextRule::comment_program, src) {
        Ok(pairs) => Ok(pairs),
        Err(e) => Err(e.into()),
    }
}
