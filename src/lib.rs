//! Implementation of the `JSONPath` spec, Proposal A with extensions.

#![forbid(unsafe_code)]
#![warn(
    missing_docs,
    elided_lifetimes_in_paths,
    explicit_outlives_requirements,
    missing_abi,
    noop_method_call,
    pointer_structural_match,
    semicolon_in_expressions_from_macros,
    unused_import_braces,
    unused_lifetimes,
    clippy::cargo,
    clippy::missing_panics_doc,
    clippy::doc_markdown,
    clippy::ptr_as_ptr,
    clippy::cloned_instead_of_copied,
    clippy::unreadable_literal,
    clippy::must_use_candidate,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

use serde_json::Value;

use ast::Span;
use ast::eval::Eval;
use error::{ParseError, ParseOrJsonError};
use eval::EvalCtx;
use idx::{Idx, IdxPath};
use utils::{delete_paths, replace_paths, try_replace_paths};

pub mod ast;
pub mod error;
mod eval;
pub mod idx;
mod utils;

#[doc(inline)]
pub use ast::Path as JsonPath;

/// Find a pattern in the provided JSON value. Recompiles the pattern every call, if the same
/// pattern is used a lot should instead try using [`JsonPath::compile`].
///
/// # Errors
///
/// - If the provided pattern fails to parse as a valid JSON path
pub fn find<'a>(pattern: &str, value: &'a Value) -> Result<Vec<&'a Value>, ParseError> {
    Ok(JsonPath::compile(pattern)?.find(value))
}

/// Find a pattern in the provided JSON string. Recompiles the pattern every call, if the same
/// pattern is used a lot should instead try using [`JsonPath::compile`].
///
/// # Errors
///
/// - If the provided pattern fails to parse as a valid JSON path
/// - If the provided value fails to deserialize
pub fn find_str(pattern: &str, value: &str) -> Result<Vec<Value>, ParseOrJsonError> {
    Ok(JsonPath::compile(pattern)?.find_str(value)?)
}

impl JsonPath {
    /// Compile a JSON path, which can be used to match items multiple times.
    ///
    /// # Errors
    ///
    /// - If the provided pattern fails to parse as a valid JSON path
    pub fn compile(pattern: &str) -> Result<JsonPath, ParseError> {
        use chumsky::{Parser, Stream};

        let len = pattern.chars().count();
        let stream = Stream::from_iter(
            Span::from(len..len),
            Box::new(
                pattern
                    .chars()
                    .enumerate()
                    .map(|(i, c)| (c, Span::from(i..i + 1))),
            ),
        );

        Self::parser()
            .parse(stream)
            .map_err(|e| ParseError::new(pattern, e))
    }

    /// Find this pattern in the provided JSON value
    #[must_use = "this does not modify the path or provided value"]
    pub fn find<'a>(&self, value: &'a Value) -> Vec<&'a Value> {
        let mut ctx = EvalCtx::new(value);
        if self.has_parent() {
            ctx.prepopulate_parents();
        }
        self.eval(&mut ctx).unwrap();
        ctx.into_matched()
    }

    /// Find this pattern in the provided JSON value, and return the shortest paths to all found
    /// values as a chain of indices
    #[must_use = "this does not modify the path or provided value"]
    pub fn find_paths(&self, value: &Value) -> Vec<IdxPath> {
        let mut ctx = EvalCtx::new(value);
        ctx.prepopulate_parents();
        self.eval(&mut ctx).unwrap();
        ctx.paths_matched()
    }

    /// Delete all items matched by this pattern on the provided JSON value, and return the
    /// resulting object
    #[must_use = "this returns the new value, without modifying the original. To work in-place, \
                  use `delete_on`"]
    pub fn delete(&self, value: &Value) -> Value {
        let paths = self.find_paths(value);
        let mut out = value.clone();
        delete_paths(paths, &mut out);
        out
    }

    /// Delete all items matched by this pattern on the provided JSON value, operating in-place
    pub fn delete_on(&self, value: &mut Value) {
        let paths = self.find_paths(value);
        delete_paths(paths, value);
    }

    /// Replace items matched by this pattern on the provided JSON value, filling them with the
    /// value returned by the provided function, then return the resulting object
    #[must_use = "this returns the new value, without modifying the original. To work in-place, \
                  use `replace_on`"]
    pub fn replace(&self, value: &Value, f: impl FnMut(&Value) -> Value) -> Value {
        let paths = self.find_paths(value);
        let mut out = value.clone();
        replace_paths(paths, &mut out, f);
        out
    }

    /// Replace items matched by this pattern on the provided JSON value, filling them the value
    /// returned by the provided function, operating in-place
    pub fn replace_on(&self, value: &mut Value, f: impl FnMut(&Value) -> Value) {
        let paths = self.find_paths(value);
        replace_paths(paths, value, f);
    }

    /// Replace or delete items matched by this pattern on the provided JSON value. Replaces if the
    /// provided method returns `Some`, deletes if the provided method returns `None`. This method
    /// then returns the resulting object
    #[must_use = "this returns the new value, without modifying the original. To work in-place, \
                  use `try_replace_on`"]
    pub fn try_replace(&self, value: &Value, f: impl FnMut(&Value) -> Option<Value>) -> Value {
        let paths = self.find_paths(value);
        let mut out = value.clone();
        try_replace_paths(paths, &mut out, f);
        out
    }

    /// Replace or delete items matched by this pattern on the provided JSON value. Replaces if the
    /// provided method returns `Some`, deletes if the provided method returns `None`. This method
    /// operates in-place on the provided value
    pub fn try_replace_on(&self, value: &mut Value, f: impl FnMut(&Value) -> Option<Value>) {
        let paths = self.find_paths(value);
        try_replace_paths(paths, value, f);
    }

    /// Find this pattern in the provided JSON string
    ///
    /// # Errors
    ///
    /// - If the provided value fails to deserialize
    pub fn find_str(&self, str: &str) -> Result<Vec<Value>, serde_json::Error> {
        let val = serde_json::from_str(str)?;
        Ok(self.find(&val).into_iter().cloned().collect())
    }

    /// Delete items matching this pattern in the provided JSON string
    ///
    /// # Errors
    ///
    /// - If the provided value fails to deserialize
    pub fn delete_str(&self, str: &str) -> Result<Value, serde_json::Error> {
        let val = serde_json::from_str(str)?;
        Ok(self.delete(&val))
    }

    /// Replace items matching this pattern in the provided JSON string
    ///
    /// # Errors
    ///
    /// - If the provided value fails to deserialize
    pub fn replace_str(
        &self,
        str: &str,
        f: impl FnMut(&Value) -> Value,
    ) -> Result<Value, serde_json::Error> {
        let val = serde_json::from_str(str)?;
        Ok(self.replace(&val, f))
    }

    /// Replace or delete items matching this pattern in the provided JSON string
    ///
    /// # Errors
    ///
    /// - If the provided value fails to deserialize
    pub fn try_replace_str(
        &self,
        str: &str,
        f: impl FnMut(&Value) -> Option<Value>,
    ) -> Result<Value, serde_json::Error> {
        let val = serde_json::from_str(str)?;
        Ok(self.try_replace(&val, f))
    }
}

#[cfg(test)]
mod tests;
