use std::collections::HashMap;
use std::hash::{Hash, Hasher};

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

#[derive(Debug)]
pub enum Idx {
    Int(usize),
    Name(String),
}

impl Idx {
    pub fn as_int(&self) -> usize {
        match self {
            Idx::Int(u) => *u,
            _ => panic!("Wrong Idx Type: expected int"),
        }
    }

    pub fn as_string(&self) -> &str {
        match self {
            Idx::Name(s) => s,
            _ => panic!("Wrong Idx Type: expected string"),
        }
    }
}

impl From<Idx> for Value {
    fn from(idx: Idx) -> Self {
        match idx {
            Idx::Int(i) => Value::from(i),
            Idx::Name(str) => Value::from(str),
        }
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
                .map(|(idx, _)| Idx::Int(idx)),
            Value::Object(m) => m
                .iter()
                .find(|&(_, p)| std::ptr::eq(p, val))
                .map(|(idx, _)| Idx::Name(idx.to_string())),
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

    pub fn paths_matched(&self) -> Vec<Vec<Idx>> {
        pub struct CurRef<'a, T>(&'a T);

        self.cur_matched
            .iter()
            .copied()
            .map(|a| {
                let mut cur = CurRef(a);
                let mut out = Vec::new();
                while let Some(p) = self.parent_of(cur.0) {
                    out.push(self.idx_of(cur.0).unwrap());
                    cur = CurRef(p);
                }
                out.reverse();
                out
            })
            .collect()
    }

    pub fn into_matched(self) -> Vec<&'a Value> {
        self.cur_matched
    }
}
