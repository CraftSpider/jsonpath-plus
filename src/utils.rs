use crate::idx::IdxPath;
use crate::Idx;
use serde_json::Value;
use std::iter::FusedIterator;

pub enum ValueIter<'a> {
    Array(std::slice::Iter<'a, Value>),
    Object(serde_json::map::Values<'a>),
    Other,
}

impl<'a> ValueIter<'a> {
    pub fn new(val: &'a Value) -> ValueIter<'a> {
        match val {
            Value::Array(v) => ValueIter::Array(v.iter()),
            Value::Object(m) => ValueIter::Object(m.values()),
            _ => ValueIter::Other,
        }
    }
}

impl<'a> Iterator for ValueIter<'a> {
    type Item = &'a Value;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            ValueIter::Array(iter) => iter.next(),
            ValueIter::Object(iter) => iter.next(),
            ValueIter::Other => None,
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl<'a> FusedIterator for ValueIter<'a>
where
    std::slice::Iter<'a, Value>: FusedIterator,
    serde_json::map::Values<'a>: FusedIterator,
{
}

impl DoubleEndedIterator for ValueIter<'_> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        match self {
            ValueIter::Array(iter) => iter.next_back(),
            ValueIter::Object(iter) => iter.next_back(),
            ValueIter::Other => None,
        }
    }
}

impl ExactSizeIterator for ValueIter<'_> {
    #[inline]
    fn len(&self) -> usize {
        match self {
            ValueIter::Array(iter) => iter.len(),
            ValueIter::Object(iter) => iter.len(),
            ValueIter::Other => 0,
        }
    }
}

pub trait ValueExt {
    fn iter(&self) -> ValueIter<'_>;
    fn remove(&mut self, key: &Idx) -> Option<Value>;
}

impl ValueExt for Value {
    #[inline]
    fn iter(&self) -> ValueIter<'_> {
        ValueIter::new(self)
    }

    #[inline]
    fn remove(&mut self, key: &Idx) -> Option<Value> {
        match (self, key) {
            (Value::Array(v), Idx::Array(idx)) => {
                if v.len() > *idx {
                    Some(v.remove(*idx))
                } else {
                    None
                }
            }
            (Value::Object(m), Idx::Object(idx)) => m.remove(idx),
            _ => None,
        }
    }
}

pub fn delete_paths(mut paths: Vec<IdxPath>, out: &mut Value) {
    // Ensure we always resolve paths longest to shortest, so if we match paths that are children
    // of other paths, they get resolved first and don't cause panics
    paths.sort_unstable_by(IdxPath::sort_specific_last);
    for path in paths {
        let delete_on = path
            .remove(1)
            .resolve_on_mut(out)
            .expect("Could resolve path");
        let last_idx = &path.raw_path()[path.len() - 1];
        delete_on
            .remove(last_idx)
            .expect("Provided path should resolve");
    }
}

pub fn replace_paths(mut paths: Vec<IdxPath>, out: &mut Value, mut f: impl FnMut(&Value) -> Value) {
    // Ensure we always resolve paths longest to shortest, so if we match paths that are children
    // of other paths, they get resolved first and don't cause panics
    paths.sort_unstable_by(IdxPath::sort_specific_last);
    for path in paths {
        let replace_on = path
            .remove(1)
            .resolve_on_mut(out)
            .expect("Could resolve path");
        let last_idx = &path.raw_path()[path.len() - 1];
        let new = f(&replace_on[last_idx]);
        replace_on[last_idx] = new;
    }
}

pub fn try_replace_paths(
    mut paths: Vec<IdxPath>,
    out: &mut Value,
    mut f: impl FnMut(&Value) -> Option<Value>,
) {
    // Ensure we always resolve paths longest to shortest, so if we match paths that are children
    // of other paths, they get resolved first and don't cause panics
    paths.sort_unstable_by(IdxPath::sort_specific_last);
    for path in paths {
        let replace_on = path
            .remove(1)
            .resolve_on_mut(out)
            .expect("Could resolve path");
        let last_idx = &path.raw_path()[path.len() - 1];

        let new = f(&replace_on[last_idx]);
        match new {
            Some(new) => replace_on[last_idx] = new,
            None => {
                replace_on
                    .remove(last_idx)
                    .expect("Provided path should resolve");
            }
        }
    }
}
