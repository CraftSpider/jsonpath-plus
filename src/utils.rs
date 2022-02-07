use crate::idx::IdxPath;
use serde_json::Value;
use std::cmp::Reverse;

pub fn delete_paths(mut paths: Vec<IdxPath>, out: &mut Value) {
    // Ensure we always resolve paths longest to shortest, so if we match paths that are children
    // of other paths, they get resolved first and don't cause panics
    paths.sort_unstable_by_key(|idx| Reverse(idx.len()));
    for path in paths {
        let delete_on = path
            .remove(1)
            .resolve_on_mut(out)
            .expect("Could resolve path");
        let last_idx = &path.raw_path()[path.len() - 1];
        match delete_on {
            Value::Array(v) => {
                v.remove(last_idx.as_array().expect("Provided path should resolve"));
            }
            Value::Object(m) => {
                m.remove(last_idx.as_object().expect("Provided path should resolve"));
            }
            _ => unreachable!(),
        }
    }
}

pub fn replace_paths(mut paths: Vec<IdxPath>, out: &mut Value, mut f: impl FnMut(&Value) -> Value) {
    // Ensure we always resolve paths longest to shortest, so if we match paths that are children
    // of other paths, they get resolved first and don't cause panics
    paths.sort_unstable_by_key(|idx| Reverse(idx.len()));
    for path in paths {
        let replace_on = path
            .remove(1)
            .resolve_on_mut(out)
            .expect("Could resolve path");
        let last_idx = &path.raw_path()[path.len() - 1];
        match replace_on {
            Value::Array(v) => {
                let last_idx = last_idx.as_array().expect("Provided path should resolve");
                let new = f(&v[last_idx]);
                v[last_idx] = new;
            }
            Value::Object(m) => {
                let last_idx = last_idx.as_object().expect("Provided path should resolve");
                let new = f(&m[last_idx]);
                m[last_idx] = new;
            }
            _ => unreachable!(),
        }
    }
}

pub fn try_replace_paths(
    mut paths: Vec<IdxPath>,
    out: &mut Value,
    mut f: impl FnMut(&Value) -> Option<Value>,
) {
    // Ensure we always resolve paths longest to shortest, so if we match paths that are children
    // of other paths, they get resolved first and don't cause panics
    paths.sort_unstable_by_key(|idx| Reverse(idx.len()));
    for path in paths {
        let replace_on = path
            .remove(1)
            .resolve_on_mut(out)
            .expect("Could resolve path");
        let last_idx = &path.raw_path()[path.len() - 1];
        match replace_on {
            Value::Array(v) => {
                let last_idx = last_idx.as_array().expect("Provided path should resolve");
                let new = f(&v[last_idx]);
                match new {
                    Some(new) => v[last_idx] = new,
                    None => {
                        v.remove(last_idx);
                    }
                }
            }
            Value::Object(m) => {
                let last_idx = last_idx.as_object().expect("Provided path should resolve");
                let new = f(&m[last_idx]);
                match new {
                    Some(new) => m[last_idx] = new,
                    None => {
                        m.remove(last_idx);
                    }
                };
            }
            _ => unreachable!(),
        }
    }
}
