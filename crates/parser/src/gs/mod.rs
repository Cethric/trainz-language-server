//! Game Script (GS) language parser implementation.
//!
//! This module provides functionality for parsing GS source code into an Abstract Syntax Tree (AST).
//! It uses [pest] as the underlying parser engine.

pub mod error;
pub mod grammar;
pub mod pratt;
#[cfg(test)]
mod tests;

pub use error::ParseError;
use grammar::{GameScriptParser, Rule};
use pest::Parser;
use pest::iterators::Pairs;

/// Parses the given Game Script source code into a set of parse tree pairs.
///
/// # Arguments
///
/// * `src` - A string slice containing the Game Script source code.
///
/// # Returns
///
/// * `Ok(Pairs<'_, Rule>)` - If parsing is successful, returns the parse tree pairs.
/// * `Err(ParseError)` - If parsing fails (e.g., syntax error), returns a `ParseError`.
///
/// # Examples
///
/// ```rust
/// use trainz_parser::gs::parse;
///
/// // Valid class definition
/// let source = "class MyClass { };";
/// let result = parse(source);
/// assert!(result.is_ok());
/// ```
///
/// ```rust
/// use trainz_parser::gs::parse;
///
/// // Missing closing brace - should fail
/// let source = "class MyClass {";
/// let result = parse(source);
/// assert!(result.is_err());
/// ```
#[tracing::instrument(skip(src))]
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
