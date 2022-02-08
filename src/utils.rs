use crate::idx::IdxPath;
use crate::Json;
use crate::json::{JsonArray, JsonMut, JsonObject};

pub fn delete_paths<T: Json>(mut paths: Vec<IdxPath>, out: &mut T) {
    // Ensure we always resolve paths longest to shortest, so if we match paths that are children
    // of other paths, they get resolved first and don't cause panics
    paths.sort_unstable_by(IdxPath::sort_specific_last);
    for path in paths {
        let delete_on = path
            .remove(1)
            .resolve_on_mut(out)
            .expect("Could resolve path");
        let last_idx = &path.raw_path()[path.len() - 1];
        match delete_on.as_mut() {
            JsonMut::Array(v) => {
                v.remove(last_idx.as_array().expect("Provided path should resolve"));
            }
            JsonMut::Object(m) => {
                m.remove(last_idx.as_object().expect("Provided path should resolve"));
            }
            _ => unreachable!(),
        }
    }
}

pub fn replace_paths<T: Json>(mut paths: Vec<IdxPath>, out: &mut T, mut f: impl FnMut(&T) -> T) {
    // Ensure we always resolve paths longest to shortest, so if we match paths that are children
    // of other paths, they get resolved first and don't cause panics
    paths.sort_unstable_by(IdxPath::sort_specific_last);
    for path in paths {
        let replace_on = path
            .remove(1)
            .resolve_on_mut(out)
            .expect("Could resolve path");
        let last_idx = &path.raw_path()[path.len() - 1];
        match replace_on.as_mut() {
            JsonMut::Array(v) => {
                let last_idx = last_idx.as_array().expect("Provided path should resolve");
                let new = f(&v.get(last_idx).unwrap());
                *v.get_mut(last_idx).unwrap() = new;
            }
            JsonMut::Object(m) => {
                let last_idx = last_idx.as_object().expect("Provided path should resolve");
                let new = f(&m.get(last_idx).unwrap());
                *m.get_mut(last_idx).unwrap() = new;
            }
            _ => unreachable!(),
        }
    }
}

pub fn try_replace_paths<T: Json>(
    mut paths: Vec<IdxPath>,
    out: &mut T,
    mut f: impl FnMut(&T) -> Option<T>,
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
        match replace_on.as_mut() {
            JsonMut::Array(v) => {
                let last_idx = last_idx.as_array().expect("Provided path should resolve");
                let new = f(v.get(last_idx).unwrap());
                match new {
                    Some(new) => *v.get_mut(last_idx).unwrap() = new,
                    None => {
                        v.remove(last_idx);
                    }
                }
            }
            JsonMut::Object(m) => {
                let last_idx = last_idx.as_object().expect("Provided path should resolve");
                let new = f(m.get(last_idx).unwrap());
                match new {
                    Some(new) => *m.get_mut(last_idx).unwrap() = new,
                    None => {
                        m.remove(last_idx);
                    }
                };
            }
            _ => unreachable!(),
        }
    }
}
