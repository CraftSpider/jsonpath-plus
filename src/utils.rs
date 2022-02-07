use crate::eval::IdxPath;
use serde_json::Value;

pub fn delete_paths(paths: Vec<IdxPath>, out: &mut Value) {
    for path in paths {
        let delete_on = path.remove(1).resolve_on_mut(out)
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

pub fn replace_paths(paths: Vec<IdxPath>, out: &mut Value, mut f: impl FnMut(&Value) -> Value) {
    for path in paths {
        let replace_on = path.remove(1).resolve_on_mut(out)
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
