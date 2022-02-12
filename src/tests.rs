use super::*;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::fmt;
use std::hash::{Hash, Hasher};

fn hash_val<H: Hasher>(val: &Value, state: &mut H) {
    match val {
        Value::Null => state.write_u8(0),
        Value::Bool(b) => {
            state.write_u8(1);
            state.write_u8(*b as u8);
        }
        Value::Number(n) => {
            state.write_u8(2);
            state.write(&n.as_f64().unwrap().to_ne_bytes());
        }
        Value::String(s) => {
            state.write_u8(3);
            state.write(s.as_bytes());
        }
        Value::Array(a) => {
            state.write_u8(4);
            state.write_usize(a.len());
            for v in a {
                hash_val(v, state);
            }
        }
        Value::Object(m) => {
            state.write_u8(5);
            state.write_usize(m.len());
            for (key, val) in m {
                state.write(key.as_bytes());
                hash_val(val, state);
            }
        }
    }
}

#[derive(PartialEq, Eq)]
pub struct ValueKey(Value);

impl fmt::Debug for ValueKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Value as fmt::Debug>::fmt(&self.0, f)
    }
}

impl Hash for ValueKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        hash_val(&self.0, state)
    }
}

impl From<Value> for ValueKey {
    fn from(val: Value) -> Self {
        ValueKey(val)
    }
}

#[test]
fn test_replace() {
    let json = json!({"list": ["red", "green", "blue"]});
    let path = JsonPath::compile("$.list[*]").unwrap();
    let result = path.replace(&json, |_| json!("black"));

    assert_eq!(result, json!({"list": ["black", "black", "black"]}));
}

#[test]
fn test_delete() {
    let json =
        json!({"inner": {"list": ["one", "two", "three"]}, "outer": ["one", "two", "three"]});
    let path = JsonPath::compile("$.inner.list[1]").unwrap();
    let result = path.delete(&json);

    assert_eq!(
        result,
        json!({"inner": {"list": ["one", "three"]}, "outer": ["one", "two", "three"]})
    );
}

#[test]
fn test_delete_array() {
    let json = json!({"list": ["one", "two", "three", "four"]});
    let result = JsonPath::compile("$.list[*]").unwrap().delete(&json);

    assert_eq!(result, json!({"list": []}));
}

#[test]
fn test_replace_in_try_replace() {
    let json = json!({"list": ["BLUE", "ORANGE", "GREEN", "RED"]});
    let result = JsonPath::compile("$.list[*]")
        .unwrap()
        .try_replace(&json, |_| Some(Value::Null));

    assert_eq!(result, json!({"list": [null, null, null, null]}));
}

#[test]
fn test_delete_in_try_replace() {
    let json = json!({"list": ["BLUE", "ORANGE", "GREEN", "RED"]});
    let result = JsonPath::compile("$.list[*]")
        .unwrap()
        .try_replace(&json, |_| None);

    assert_eq!(result, json!({"list": []}));
}

#[test]
fn dot_notation_after_recursive_descent() {
    let json = json!({
        "a": {"list": [1, 2, 3], "null": null, "id": []},
        "b": [{"id": 1, "name": "foo"}, {"id": 2, "name": "bar"}],
        "c": 1,
        "d": false,
    });
    let result = find("$..id", &json)
        .unwrap()
        .into_iter()
        .cloned()
        .map(ValueKey::from)
        .collect::<HashSet<ValueKey>>();

    assert_eq!(
        result,
        HashSet::from([json!([]), json!(1), json!(2)].map(ValueKey::from))
    );
}

#[test]
fn bracket_notation_after_recursive_descent() {
    let json = json!({
        "a": {"list": [1, 2, 3], "null": null, "id": []},
        "b": [{"id": 1, "name": "foo"}, {"id": 2, "name": "bar"}],
        "c": 1,
        "d": false,
    });
    let result = find("$..['id']", &json)
        .unwrap()
        .into_iter()
        .cloned()
        .map(ValueKey::from)
        .collect::<HashSet<ValueKey>>();

    assert_eq!(
        result,
        HashSet::from([json!([]), json!(1), json!(2)].map(ValueKey::from))
    );
}

#[test]
fn parent_after_dot_notation() {
    let json = json!({"a": {"b": true}});
    let result = find("$.a.b.^", &json).unwrap();

    let expected = vec![&json.as_object().unwrap()["a"]];

    assert_eq!(result, expected);
}

#[test]
fn parent_after_recursive_descent() {
    let json = json!({
        "a": {"list": [1, 2, 3], "null": null},
        "b": [{"id": 1, "name": "foo"}, {"id": 2, "name": "bar"}],
        "c": 1,
        "d": false,
    });
    let result = find("$..^", &json)
        .unwrap()
        .into_iter()
        .cloned()
        .map(ValueKey::from)
        .collect::<HashSet<ValueKey>>();

    assert_eq!(
        result,
        HashSet::from(
            [
                json!([1, 2, 3]),
                json!({"list": [1, 2, 3], "null": null}),
                json!({"id": 1, "name": "foo"}),
                json!({"id": 2, "name": "bar"}),
                json!([{"id": 1, "name": "foo"}, {"id": 2, "name": "bar"}]),
                json!({
                    "a": {"list": [1, 2, 3], "null": null},
                    "b": [{"id": 1, "name": "foo"}, {"id": 2, "name": "bar"}],
                    "c": 1,
                    "d": false,
                }),
            ]
            .map(ValueKey::from)
        )
    );
}

#[test]
fn array_slice_on_non_overlapping_array() {
    let json = json!(["first", "second", "third"]);
    let result = find("$[7:10]", &json).unwrap();

    assert_eq!(result, &[] as &[&Value]);
}

#[test]
fn array_slice_on_partially_overlapping_array() {
    let json = json!(["first", "second", "third"]);
    let result = find("$[1:10]", &json).unwrap();

    let expected = vec![&json.as_array().unwrap()[1], &json.as_array().unwrap()[2]];

    assert_eq!(result, expected);
}

#[test]
fn array_slice_with_large_end_number() {
    let json = json!(["first", "second", "third", "forth", "fifth"]);
    let result = find("$[2:113667776004]", &json).unwrap();

    let expected = vec![
        &json.as_array().unwrap()[2],
        &json.as_array().unwrap()[3],
        &json.as_array().unwrap()[4],
    ];

    assert_eq!(result, expected);
}

#[test]
fn array_slice_with_large_number_start() {
    let json = json!(["first", "second", "third", "forth", "fifth"]);
    let result = find("$[-113667776004:2]", &json).unwrap();

    let expected = vec![&json.as_array().unwrap()[0], &json.as_array().unwrap()[1]];

    assert_eq!(result, expected);
}

#[test]
fn array_slice_with_negative_step_only() {
    let json = json!(["first", "second", "third", "forth", "fifth"]);
    let result = find("$[::-2]", &json).unwrap();

    let expected = vec![
        &json.as_array().unwrap()[4],
        &json.as_array().unwrap()[2],
        &json.as_array().unwrap()[0],
    ];

    assert_eq!(result, expected);
}

#[test]
fn bracket_notation_with_negative_number_on_short_array() {
    let json = json!(["one element"]);
    let result = find("$[-2]", &json).unwrap();

    assert_eq!(result, &[] as &[&Value]);
}

#[test]
fn bracket_notation_with_number_on_object() {
    let json = json!({"0": "value"});
    let result = find("$[0]", &json).unwrap();

    assert_eq!(result, &[] as &[&Value]);
}

#[test]
fn bracket_notation_with_spaces() {
    let json = json!({" a": 1, "a": 2, " a ": 3, "a ": 4, " 'a' ": 5, " 'a": 6, "a' ": 7, " \"a\" ": 8, "\"a\"": 9});
    let result = find("$[ 'a' ]", &json).unwrap();

    let expected = vec![&json.as_object().unwrap()["a"]];

    assert_eq!(result, expected);
}

#[test]
fn dot_notation_after_filter_expression() {
    let json = json!([{"id": 42, "name": "forty-two"}, {"id": 1, "name": "one"}]);
    let result = find("$[?(@.id==42)].name", &json).unwrap();

    let expected = vec![&json.as_array().unwrap()[0].as_object().unwrap()["name"]];

    assert_eq!(result, expected);
}

#[test]
#[should_panic]
fn dot_notation_with_empty_path() {
    let json = json!({"key": 42, "": 9001, "''": "nice"});
    let _result = find("$.", &json).unwrap();
}
