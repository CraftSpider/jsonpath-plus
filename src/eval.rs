use core::hash::{Hash, Hasher};
use std::collections::HashMap;

use crate::idx::{Idx, IdxPath};
use serde_json::Value;

#[derive(Clone)]
pub struct RefKey<'a, T>(&'a T);

impl<'a, T> PartialEq for RefKey<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        core::ptr::eq(self.0, other.0)
    }
}

impl<'a, T> Eq for RefKey<'a, T> {}

impl<'a, T> Hash for RefKey<'a, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(self.0 as *const T as usize);
    }
}

pub struct EvalCtx<'a> {
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
            Value::Array(v) => v.iter().enumerate().find_map(|(idx, p)| {
                if core::ptr::eq(p, val) {
                    Some(Idx::Array(idx))
                } else {
                    None
                }
            }),
            Value::Object(m) => m.iter().find_map(|(idx, p)| {
                if core::ptr::eq(p, val) {
                    Some(Idx::Object(idx.to_string()))
                } else {
                    None
                }
            }),
            _ => None,
        }
    }

    pub fn parent_of(&self, val: &'a Value) -> Option<&'a Value> {
        self.parents.get(&RefKey(val)).copied()
    }

    fn parents_recur(parents: &mut HashMap<RefKey<'a, Value>, &'a Value>, parent: &'a Value) {
        match parent {
            Value::Array(v) => {
                for child in v {
                    parents.insert(RefKey(child), parent);
                    EvalCtx::parents_recur(parents, child);
                }
            }
            Value::Object(m) => {
                for (_, child) in m {
                    parents.insert(RefKey(child), parent);
                    EvalCtx::parents_recur(parents, child);
                }
            }
            _ => (),
        }
    }

    pub fn prepopulate_parents(&mut self) {
        Self::parents_recur(&mut self.parents, self.root);
    }

    pub fn set_matched(&mut self, matched: Vec<&'a Value>) {
        self.cur_matched = matched;
    }

    pub fn apply_matched<T>(&mut self, f: impl Fn(&Self, &'a Value) -> T)
    where
        T: IntoIterator<Item = &'a Value>,
    {
        let cur_matched = core::mem::take(&mut self.cur_matched);
        self.cur_matched = cur_matched.into_iter().flat_map(|i| f(self, i)).collect();
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
                IdxPath::new(out)
            })
            .collect()
    }

    pub fn into_matched(self) -> Vec<&'a Value> {
        self.cur_matched
    }
}
