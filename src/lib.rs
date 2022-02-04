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
    clippy::unreadable_literal
)]

use error::{ParseError, ParseOrJsonError};
use eval::EvalCtx;

mod ast;
mod eval;
pub mod error;

pub use ast::Path as JsonPath;

fn resolve_path<'a>(path: &[eval::Idx], val: &'a mut serde_json::Value) -> &'a mut serde_json::Value {
    use serde_json::Value;
    pub struct CurRef<'a, T>(&'a mut T);

    let mut cur = CurRef(val);

    for p in path {
        match cur.0 {
            Value::Array(v) => cur = CurRef(&mut v[p.as_int()]),
            Value::Object(m) => cur = CurRef(&mut m[p.as_string()]),
            _ => unreachable!()
        }
    }

    cur.0
}

/// Find a pattern in the provided JSON value. Recompiles the pattern every call, if the same
/// pattern is used a lot should instead try using [`JsonPath::compile`].
///
/// # Errors
///
/// - If the provided pattern fails to parse as a valid JSON path
pub fn find<'a>(pattern: &str, value: &'a serde_json::Value) -> Result<Vec<&'a serde_json::Value>, ParseError> {
    Ok(JsonPath::compile(pattern)?.find(value))
}

/// Find a pattern in the provided JSON string. Recompiles the pattern every call, if the same
/// pattern is used a lot should instead try using [`JsonPath::compile`].
///
/// # Errors
///
/// - If the provided pattern fails to parse as a valid JSON path
/// - If the provided value fails to deserialize
pub fn find_str(pattern: &str, value: &str) -> Result<Vec<serde_json::Value>, ParseOrJsonError> {
    Ok(JsonPath::compile(pattern)?.find_str(value)?)
}

impl JsonPath {
    /// Compile a JSON path, which can be used to match items multiple times.
    ///
    /// # Errors
    ///
    /// - If the provided pattern fails to parse as a valid JSON path
    pub fn compile(pattern: &str) -> Result<JsonPath, ParseError> {
        use chumsky::Parser;

        Self::parser()
            .parse(pattern)
            .map_err(|e| ParseError::new(pattern, e))
    }

    /// Find this pattern in the provided JSON value
    pub fn find<'a>(&self, value: &'a serde_json::Value) -> Vec<&'a serde_json::Value> {
        let mut ctx = EvalCtx::new(value);

        self.eval(&mut ctx);

        ctx.into_matched()
    }

    /// Delete all items matched by this pattern on the provided JSON value, and return the
    /// resulting object
    pub fn delete(&self, value: &serde_json::Value) -> serde_json::Value {
        use serde_json::Value;

        let mut ctx = EvalCtx::new(value);
        self.eval(&mut ctx);

        let paths: Vec<_> = ctx.paths_matched();

        let mut out = value.clone();

        for p in paths {
            let delete_on = resolve_path(&p[..p.len() - 1], &mut out);
            let last_idx = p.last().expect("Idx should match found item");
            match delete_on {
                Value::Array(v) => {
                    v.remove(last_idx.as_int());
                }
                Value::Object(m) => {
                    m.remove(last_idx.as_string());
                }
                _ => unreachable!(),
            }
        }

        out
    }

    /// Replace items matched by this pattern on the provided JSON value, filling them with the
    /// value returned by the provided function, then return the resulting object
    pub fn replace(&self, value: &serde_json::Value, mut f: impl FnMut(&serde_json::Value) -> serde_json::Value) -> serde_json::Value {
        use serde_json::Value;

        let mut ctx = EvalCtx::new(value);
        self.eval(&mut ctx);

        let paths: Vec<_> = ctx.paths_matched();

        let mut out = value.clone();

        for p in paths {
            let replace_on = resolve_path(&p[..p.len() - 1], &mut out);
            let last_idx = p.last().unwrap();
            match replace_on {
                Value::Array(v) => {
                    let last_idx = last_idx.as_int();
                    let new = f(&v[last_idx]);
                    v[last_idx] = new;
                }
                Value::Object(m) => {
                    let last_idx = last_idx.as_string();
                    let new = f(&m[last_idx]);
                    m[last_idx] = new;
                }
                _ => unreachable!(),
            }
        }

        out
    }

    /// Find this pattern in the provided JSON string
    ///
    /// # Errors
    ///
    /// - If the provided value fails to deserialize
    pub fn find_str(&self, str: &str) -> Result<Vec<serde_json::Value>, serde_json::Error> {
        let val = serde_json::from_str(str)?;
        Ok(self.find(&val).into_iter().cloned().collect())
    }

    /// Delete items matching this pattern in the provided JSON string
    ///
    /// # Errors
    ///
    /// - If the provided value fails to deserialize
    pub fn delete_str(&self, str: &str) -> Result<serde_json::Value, serde_json::Error> {
        let val = serde_json::from_str(str)?;
        Ok(self.delete(&val))
    }

    /// Replace items matching this pattern in the provided JSON string
    ///
    /// # Errors
    ///
    /// - If the provided value fails to deserialize
    pub fn replace_str(&self, str: &str, f: impl FnMut(&serde_json::Value) -> serde_json::Value) -> Result<serde_json::Value, serde_json::Error> {
        let val = serde_json::from_str(str)?;
        Ok(self.replace(&val, f))
    }
}

#[cfg(test)]
mod tests;
