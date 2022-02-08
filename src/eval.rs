use core::hash::{Hash, Hasher};
use std::collections::HashMap;

use crate::idx::{Idx, IdxPath};
use crate::json::{Json, JsonArray, JsonObject, JsonRef};

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

pub struct EvalCtx<'a, T: Json> {
    root: &'a T,
    cur_matched: Vec<&'a T>,
    parents: HashMap<RefKey<'a, T>, &'a T>,
}

impl<'a, T: Json> EvalCtx<'a, T> {
    pub fn new(root: &'a T) -> EvalCtx<'a, T> {
        EvalCtx {
            root,
            cur_matched: vec![root],
            parents: HashMap::new(),
        }
    }

    pub fn new_parents(
        root: &'a T,
        parents: HashMap<RefKey<'a, T>, &'a T>,
    ) -> EvalCtx<'a, T> {
        EvalCtx {
            root,
            cur_matched: vec![root],
            parents,
        }
    }

    pub fn child_ctx(&self) -> EvalCtx<'a, T> {
        EvalCtx {
            root: self.root,
            cur_matched: self.cur_matched.clone(),
            parents: self.parents.clone(),
        }
    }

    pub fn root(&self) -> &'a T {
        self.root
    }

    pub fn all_parents(&self) -> &HashMap<RefKey<'a, T>, &'a T> {
        &self.parents
    }

    pub fn idx_of(&self, val: &'a T) -> Option<Idx> {
        let parent = self.parent_of(val)?;
        match parent.as_ref() {
            JsonRef::Array(v) => v
                .iter()
                .enumerate()
                .find(|&(_, p)| core::ptr::eq(p, val))
                .map(|(idx, _)| Idx::Array(idx)),
            JsonRef::Object(m) => m
                .iter()
                .find(|&(_, p)| core::ptr::eq(p, val))
                .map(|(idx, _)| Idx::Object(idx.to_string())),
            _ => None,
        }
    }

    pub fn parent_of(&self, val: &'a T) -> Option<&'a T> {
        self.parents.get(&RefKey(val)).copied()
    }

    fn parents_recur(&mut self, value: &'a T) {
        match value.as_ref() {
            JsonRef::Array(v) => {
                for child in v.iter() {
                    self.parents.entry(RefKey(child)).or_insert(value);
                    self.parents_recur(child);
                }
            }
            JsonRef::Object(m) => {
                for child in m.values() {
                    self.parents.entry(RefKey(child)).or_insert(value);
                    self.parents_recur(child);
                }
            }
            _ => (),
        }
    }

    pub fn prepopulate_parents(&mut self) {
        self.cur_matched
            .clone()
            .into_iter()
            .for_each(|v| self.parents_recur(v));
    }

    pub fn set_matched(&mut self, matched: Vec<&'a T>) {
        self.cur_matched = matched;
    }

    pub fn apply_matched(&mut self, f: impl Fn(&Self, &'a T) -> Vec<&'a T>) {
        let cur_matched = core::mem::take(&mut self.cur_matched);
        self.cur_matched = cur_matched
            .into_iter()
            .flat_map(|i| {
                let results = f(self, i);
                for &a in &results {
                    self.parents.entry(RefKey(a)).or_insert(i);
                }
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
                IdxPath::new(out)
            })
            .collect()
    }

    pub fn into_matched(self) -> Vec<&'a T> {
        self.cur_matched
    }
}
