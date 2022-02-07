//! Errors returned by fallible methods

use std::error::Error;
use std::{error, fmt};

use chumsky::error::Simple;

/// Error returned by a failure to parse a provided JSON Path
#[derive(Debug)]
pub struct ParseError {
    src: String,
    errs: Vec<Simple<char>>,
}

impl ParseError {
    pub(crate) fn new(src: &str, errs: Vec<Simple<char>>) -> ParseError {
        ParseError {
            src: src.to_string(),
            errs,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Error Parsing JSON Path:")?;
        writeln!(f, "{}", self.src)?;
        for err in &self.errs {
            writeln!(f, "{}", err)?;
        }
        Ok(())
    }
}

impl error::Error for ParseError {}

/// Enum for an error that might be either a failure to parse a JSON path, or failure to deserialize
/// JSON data
#[derive(Debug)]
pub enum ParseOrJsonError {
    /// Error was a failure to parse JSON Path
    Parse(ParseError),
    /// Error was a failure to deserialize JSON data
    Json(serde_json::Error),
}

impl fmt::Display for ParseOrJsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseOrJsonError::Parse(err) => write!(f, "{}", err),
            ParseOrJsonError::Json(err) => write!(f, "{}", err),
        }
    }
}

impl error::Error for ParseOrJsonError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ParseOrJsonError::Parse(p) => Some(p),
            ParseOrJsonError::Json(j) => Some(j),
        }
    }
}

impl From<ParseError> for ParseOrJsonError {
    fn from(err: ParseError) -> Self {
        ParseOrJsonError::Parse(err)
    }
}

impl From<serde_json::Error> for ParseOrJsonError {
    fn from(err: serde_json::Error) -> Self {
        ParseOrJsonError::Json(err)
    }
}
