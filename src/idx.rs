//! Items related to shortest-path indexing of JSON objects

use crate::error::{JsonTy, ResolveError};
use serde_json::Value;

/// An index on a JSON object, either an integer index on an array or a string index on an object
#[derive(Clone, Debug)]
pub enum Idx {
    /// An array index
    Array(usize),
    /// An object index
    Object(String),
}

impl Idx {

    /// Whether this is an array index
    #[must_use]
    pub fn is_array(&self) -> bool {
        matches!(self, Idx::Array(_))
    }

    /// Whether this is an object index
    #[must_use]
    pub fn is_object(&self) -> bool {
        matches!(self, Idx::Object(_))
    }

    /// Get this index as an array index, or None
    #[must_use]
    pub fn as_array(&self) -> Option<usize> {
        match self {
            Idx::Array(u) => Some(*u),
            _ => None,
        }
    }

    /// Get this index as an object index, or None
    #[must_use]
    pub fn as_object(&self) -> Option<&str> {
        match self {
            Idx::Object(s) => Some(s),
            _ => None,
        }
    }
}

impl From<Idx> for Value {
    fn from(idx: Idx) -> Self {
        match idx {
            Idx::Array(i) => Value::from(i),
            Idx::Object(str) => Value::from(str),
        }
    }
}

/// A shortest-path set of indices on a JSON object
pub struct IdxPath(Vec<Idx>);

impl IdxPath {
    pub(crate) const fn new(indices: Vec<Idx>) -> IdxPath {
        IdxPath(indices)
    }

    /// The length of this path
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether this path is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Reference this path as a raw slice of indices
    #[must_use]
    pub fn raw_path(&self) -> &[Idx] {
        &self.0
    }

    /// Remove the last `n` items from this path
    ///
    /// # Panics
    ///
    /// - If `n` is greater than the length of this path
    #[must_use]
    pub fn remove(&self, n: usize) -> IdxPath {
        assert!(
            n <= self.len(),
            "Cannot remove {} items from path, path is only {} items long", n, self.len()
        );
        IdxPath(self.0[..self.len() - n].to_owned())
    }

    /// Resolve this path on a value, returning a reference to the result or an error indicating
    /// why the path couldn't be resolved
    ///
    /// # Errors
    ///
    /// - If the path cannot be resolved
    pub fn resolve_on<'a>(&self, value: &'a Value) -> Result<&'a Value, ResolveError> {
        let mut cur = value;

        for idx in &self.0 {
            match idx {
                Idx::Array(i) => {
                    cur = cur.as_array()
                        .ok_or_else(|| ResolveError::mismatched(JsonTy::Array, cur))?
                        .get(*i)
                        .ok_or_else(|| ResolveError::MissingIdx(idx.clone()))?;
                }
                Idx::Object(i) => {
                    cur = cur.as_object()
                        .ok_or_else(|| ResolveError::mismatched(JsonTy::Object, cur))?
                        .get(i)
                        .ok_or_else(|| ResolveError::MissingIdx(idx.clone()))?;
                }
            }
        }

        Ok(cur)
    }

    /// Resolve this path on a value, returning a mutable reference to the result or an error
    /// indicating why the path couldn't be resolved
    ///
    /// # Errors
    ///
    /// - If the path cannot be resolved
    pub fn resolve_on_mut<'a>(&self, value: &'a mut Value) -> Result<&'a mut Value, ResolveError> {
        let mut cur = value;

        for idx in &self.0 {
            match idx {
                Idx::Array(i) => {
                    let json_ty = JsonTy::from(&*cur);
                    cur = cur.as_array_mut()
                        .ok_or(ResolveError::MismatchedTy { expected: JsonTy::Array, actual: json_ty })?
                        .get_mut(*i)
                        .ok_or_else(|| ResolveError::MissingIdx(idx.clone()))?;
                }
                Idx::Object(i) => {
                    let json_ty = JsonTy::from(&*cur);
                    cur = cur.as_object_mut()
                        .ok_or(ResolveError::MismatchedTy { expected: JsonTy::Array, actual: json_ty })?
                        .get_mut(i)
                        .ok_or_else(|| ResolveError::MissingIdx(idx.clone()))?;
                }
            }
        }

        Ok(cur)
    }
}

impl From<Vec<Idx>> for IdxPath {
    fn from(path: Vec<Idx>) -> Self {
        IdxPath(path)
    }
}
