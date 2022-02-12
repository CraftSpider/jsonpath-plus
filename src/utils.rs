use crate::idx::IdxPath;
use serde_json::Value;

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
            None => match replace_on {
                Value::Array(v) => {
                    v.remove(last_idx.as_array().expect("Provided path should resolve"));
                },
                Value::Object(m) => {
                    m.remove(last_idx.as_object().expect("Provided path should resolve"));
                },
                _ => unreachable!(),
            },
        }
    }
}
