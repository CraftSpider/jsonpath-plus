use super::*;
use serde_json::{json, Value};

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
    let result = JsonPath::compile("$.list[*]").unwrap().try_replace(&json, |_| Some(Value::Null));

    assert_eq!(result, json!({"list": [null, null, null, null]}));
}

#[test]
fn test_delete_in_try_replace() {
    let json = json!({"list": ["BLUE", "ORANGE", "GREEN", "RED"]});
    let result = JsonPath::compile("$.list[*]").unwrap().try_replace(&json, |_| None);

    assert_eq!(result, json!({"list": []}));
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
