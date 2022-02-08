
use super::{Json, JsonNumber, JsonObject, JsonRef, JsonMut, ObjectIter, ObjectValues};

use std::marker::PhantomData;
use serde_json::{Value, Map, Number};

impl JsonNumber for Number {
    fn as_u64(&self) -> Option<u64> {
        self.as_u64()
    }

    fn as_i64(&self) -> Option<i64> {
        self.as_i64()
    }

    fn as_f64(&self) -> Option<f64> {
        self.as_f64()
    }
}

impl JsonObject<Value> for Map<String, Value> {
    fn get(&self, key: &str) -> Option<&Value> {
        self.get(key)
    }

    fn get_mut(&mut self, key: &str) -> Option<&mut Value> {
        self.get_mut(key)
    }

    fn remove(&mut self, key: &str) {
        self.remove(key);
    }

    fn iter(&self) -> ObjectIter<'_, Self, Value> {
        ObjectIter(self, self.keys().map(|a| &**a).collect::<Vec<_>>().into_iter(), PhantomData)
    }

    fn values(&self) -> ObjectValues<'_, Self, Value> {
        ObjectValues(self, self.keys().map(|a| &**a).collect::<Vec<_>>().into_iter(), PhantomData)
    }
}

impl Json for Value {
    type Number = Number;
    type Array = Vec<Value>;
    type Object = Map<String, Value>;

    fn null() -> Self {
        Value::Null
    }

    fn from_bool(val: bool) -> Self {
        Value::from(val)
    }

    fn from_i64(val: i64) -> Self {
        Value::from(val)
    }

    fn from_f64(val: f64) -> Self {
        Value::from(val)
    }

    fn from_str(val: String) -> Self {
        Value::from(val)
    }

    fn as_ref(&self) -> JsonRef<'_, Self> {
        match self {
            Value::Null => JsonRef::Null,
            Value::Bool(b) => JsonRef::Bool(*b),
            Value::Number(n) => JsonRef::Number(n),
            Value::String(s) => JsonRef::String(s),
            Value::Array(a) => JsonRef::Array(a),
            Value::Object(o) => JsonRef::Object(o),
        }
    }

    fn as_mut(&mut self) -> JsonMut<'_, Self> {
        match self {
            Value::Null => JsonMut::Null,
            Value::Bool(b) => JsonMut::Bool(b),
            Value::Number(n) => JsonMut::Number(n),
            Value::String(s) => JsonMut::String(s),
            Value::Array(a) => JsonMut::Array(a),
            Value::Object(o) => JsonMut::Object(o),
        }
    }
}
