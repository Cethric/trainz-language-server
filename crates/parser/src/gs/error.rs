use crate::gs::grammar::Rule;
use pest::error::Error as PestError;
use thiserror::Error;

/// Represents errors that can occur during the parsing of Game Script code.
#[derive(Debug, Error)]
pub enum ParseError {
    /// Syntax error occurred during parsing.
    #[error("syntax error: {0}")]
    Syntax(#[from] PestError<Rule>),
}
