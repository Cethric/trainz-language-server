use pest::error::Error as PestError;
use thiserror::Error;

use crate::soup::grammar::Rule;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("syntax error: {0}")]
    Syntax(String),

    #[error("internal error: {0}")]
    Internal(String),
}

impl From<PestError<Rule>> for ParseError {
    fn from(e: PestError<Rule>) -> Self {
        ParseError::Syntax(e.to_string())
    }
}
