use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use crate::error::{JsonTy, ResolveError};
use serde_json::Value;

#[derive(Clone)]
pub struct RefKey<'a, T>(&'a T);

impl<'a, T> PartialEq for RefKey<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0, other.0)
    }
}

impl<'a, T> Eq for RefKey<'a, T> {}

impl<'a, T> Hash for RefKey<'a, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(self.0 as *const T as usize)
    }
}

#[derive(Clone, Debug)]
pub enum Idx {
    Array(usize),
    Object(String),
}

impl Idx {
    pub fn is_array(&self) -> bool {
        matches!(self, Idx::Array(_))
    }

    pub fn is_object(&self) -> bool {
        matches!(self, Idx::Object(_))
    }

    pub fn as_array(&self) -> Option<usize> {
        match self {
            Idx::Array(u) => Some(*u),
            _ => None,
        }
    }

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

pub struct IdxPath(Vec<Idx>);

impl IdxPath {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn raw_path(&self) -> &[Idx] {
        &self.0
    }

    pub fn remove(&self, n: usize) -> IdxPath {
        if n > self.len() {
            panic!("Cannot remove {} items from path, path is only {} items long", n, self.len())
        }
        IdxPath(self.0[..self.len() - n].to_owned())
    }

    pub fn resolve_on<'a>(&self, value: &'a Value) -> Result<&'a Value, ResolveError> {
        let mut cur = value;

        for idx in &self.0 {
            match idx {
                Idx::Array(i) => {
                    cur = cur.as_array()
                        .ok_or_else(|| ResolveError::mismatched(JsonTy::Array, cur))?
                        .get(*i)
                        .ok_or_else(|| ResolveError::MissingIdx(idx.clone()))?
                }
                Idx::Object(i) => {
                    cur = cur.as_object()
                        .ok_or_else(|| ResolveError::mismatched(JsonTy::Object, cur))?
                        .get(i)
                        .ok_or_else(|| ResolveError::MissingIdx(idx.clone()))?
                }
            }
        }

        Ok(cur)
    }

    pub fn resolve_on_mut<'a>(&self, value: &'a mut Value) -> Result<&'a mut Value, ResolveError> {
        let mut cur = value;

        for idx in &self.0 {
            match idx {
                Idx::Array(i) => {
                    let json_ty = JsonTy::from(&*cur);
                    cur = cur.as_array_mut()
                        .ok_or(ResolveError::MismatchedTy { expected: JsonTy::Array, actual: json_ty })?
                        .get_mut(*i)
                        .ok_or_else(|| ResolveError::MissingIdx(idx.clone()))?
                }
                Idx::Object(i) => {
                    let json_ty = JsonTy::from(&*cur);
                    cur = cur.as_object_mut()
                        .ok_or(ResolveError::MismatchedTy { expected: JsonTy::Array, actual: json_ty })?
                        .get_mut(i)
                        .ok_or_else(|| ResolveError::MissingIdx(idx.clone()))?
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

pub(crate) struct EvalCtx<'a> {
    root: &'a Value,
    cur_matched: Vec<&'a Value>,
    parents: HashMap<RefKey<'a, Value>, &'a Value>,
}

impl<'a> EvalCtx<'a> {
    pub fn new(root: &'a Value) -> EvalCtx<'a> {
        EvalCtx {
            root,
            cur_matched: vec![root],
            parents: HashMap::new(),
        }
    }

    pub fn new_parents(
        root: &'a Value,
        parents: HashMap<RefKey<'a, Value>, &'a Value>,
    ) -> EvalCtx<'a> {
        EvalCtx {
            root,
            cur_matched: vec![root],
            parents,
        }
    }

    pub fn child_ctx(&self) -> EvalCtx<'a> {
        EvalCtx {
            root: self.root,
            cur_matched: self.cur_matched.clone(),
            parents: self.parents.clone(),
        }
    }

    pub fn root(&self) -> &'a Value {
        self.root
    }

    pub fn all_parents(&self) -> &HashMap<RefKey<'a, Value>, &'a Value> {
        &self.parents
    }

    pub fn idx_of(&self, val: &'a Value) -> Option<Idx> {
        let parent = self.parent_of(val)?;
        match parent {
            Value::Array(v) => v
                .iter()
                .enumerate()
                .find(|&(_, p)| std::ptr::eq(p, val))
                .map(|(idx, _)| Idx::Array(idx)),
            Value::Object(m) => m
                .iter()
                .find(|&(_, p)| std::ptr::eq(p, val))
                .map(|(idx, _)| Idx::Object(idx.to_string())),
            _ => None,
        }
    }

    pub fn parent_of(&self, val: &'a Value) -> Option<&'a Value> {
        self.parents.get(&RefKey(val)).copied()
    }

    pub fn set_matched(&mut self, matched: Vec<&'a Value>) {
        self.cur_matched = matched;
    }

    pub fn apply_matched(&mut self, f: impl Fn(&Self, &'a Value) -> Vec<&'a Value>) {
        let cur_matched = std::mem::take(&mut self.cur_matched);
        self.cur_matched = cur_matched
            .into_iter()
            .flat_map(|i| {
                let results = f(self, i);
                results.iter().for_each(|a| {
                    self.parents.insert(RefKey(*a), i);
                });
                results
            })
            .collect();
    }

    pub fn paths_matched(&self) -> Vec<IdxPath> {
        self.cur_matched
            .iter()
            .copied()
            .map(|a| {
                let mut cur = a;
                let mut out = Vec::new();
                while let Some(p) = self.parent_of(cur) {
                    out.push(self.idx_of(cur).unwrap());
                    cur = p;
                }
                out.reverse();
                IdxPath(out)
            })
            .collect()
    }

    pub fn into_matched(self) -> Vec<&'a Value> {
        self.cur_matched
    }
}
