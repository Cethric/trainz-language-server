use pest::error::Error as PestError;
use thiserror::Error;

use crate::comments::grammar::acs_text::Rule as AcsTextRule;
use crate::comments::grammar::gs::Rule as GsRule;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("syntax error: {0}")]
    Syntax(String),

    #[error("internal error: {0}")]
    Internal(String),
}

impl From<PestError<GsRule>> for ParseError {
    fn from(e: PestError<GsRule>) -> Self {
        ParseError::Syntax(e.to_string())
    }
}

impl From<PestError<AcsTextRule>> for ParseError {
    fn from(e: PestError<AcsTextRule>) -> Self {
        ParseError::Syntax(e.to_string())
    }
}
