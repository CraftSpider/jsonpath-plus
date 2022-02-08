//! Errors returned by fallible methods

use core::fmt;
use std::error;
use std::error::Error;

use crate::{Idx, Json};
use chumsky::error::Simple;
use serde_json::Value;
use crate::json::JsonRef;

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

/// Type of a JSON Value for error info
#[derive(Copy, Clone, Debug)]
pub enum JsonTy {
    /// `null`
    Null,
    /// `true` or `false`
    Bool,
    /// `1.5` or similar
    Number,
    /// `"foo"` or similar
    String,
    /// `[1, 2, 3]` or similar
    Array,
    /// `{"a": false}` or similar
    Object,
}

impl fmt::Display for JsonTy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JsonTy::Null => write!(f, "null"),
            JsonTy::Bool => write!(f, "bool"),
            JsonTy::Number => write!(f, "number"),
            JsonTy::String => write!(f, "string"),
            JsonTy::Array => write!(f, "array"),
            JsonTy::Object => write!(f, "object"),
        }
    }
}

impl<T: Json> From<&T> for JsonTy {
    fn from(val: &T) -> Self {
        match val.as_ref() {
            JsonRef::Null => JsonTy::Null,
            JsonRef::Bool(_) => JsonTy::Bool,
            JsonRef::Number(_) => JsonTy::Number,
            JsonRef::String(_) => JsonTy::String,
            JsonRef::Array(_) => JsonTy::Array,
            JsonRef::Object(_) => JsonTy::Object,
        }
    }
}

/// Error returned by a failure to resolve a path of indices
#[derive(Debug)]
pub enum ResolveError {
    /// Expected next item in the path to be a specific type, but it wasn't
    MismatchedTy {
        /// Type that was expected
        expected: JsonTy,
        /// Type that was found
        actual: JsonTy,
    },
    /// Expected an index to exist, but it didn't
    MissingIdx(Idx),
}

impl ResolveError {
    pub(crate) fn mismatched(expected: JsonTy, got: &Value) -> ResolveError {
        ResolveError::MismatchedTy {
            expected,
            actual: got.into(),
        }
    }
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResolveError::MismatchedTy { expected, actual } => {
                write!(
                    f,
                    "Resolved path expected type {}, instead got type {}",
                    expected, actual
                )
            }
            ResolveError::MissingIdx(idx) => {
                let idx = match idx {
                    Idx::Array(i) => i as &dyn fmt::Debug,
                    Idx::Object(i) => i as &dyn fmt::Debug,
                };
                write!(
                    f,
                    "Resolved path expected an index {:?}, but it didn't exist",
                    idx
                )
            }
        }
    }
}
