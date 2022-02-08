use std::marker::PhantomData;
use super::{Json, JsonRef, JsonMut, JsonNumber, JsonObject};

use json::{JsonValue, Array};
use json::number::Number;
use json::object::Object;
use crate::json::{ObjectIter, ObjectValues};

impl JsonNumber for Number {
    fn as_u64(&self) -> Option<u64> {
        if self.as_parts().2 != 0 {
            None
        } else {
            self.as_fixed_point_u64(0)
        }
    }

    fn as_i64(&self) -> Option<i64> {
        if self.as_parts().2 != 0 {
            None
        } else {
            self.as_fixed_point_i64(0)
        }
    }

    fn as_f64(&self) -> Option<f64> {
        Some(self.clone().into())
    }
}

impl JsonObject<JsonValue> for Object {
    fn get(&self, key: &str) -> Option<&JsonValue> {
        self.get(key)
    }

    fn get_mut(&mut self, key: &str) -> Option<&mut JsonValue> {
        self.get_mut(key)
    }

    fn remove(&mut self, key: &str) {
        self.remove(key);
    }

    fn iter(&self) -> ObjectIter<'_, Self, JsonValue> {
        ObjectIter(
            self,
            self.iter().map(|(key, _)| key).collect::<Vec<_>>().into_iter(),
            PhantomData
        )
    }

    fn values(&self) -> ObjectValues<'_, Self, JsonValue> {
        ObjectValues(
            self,
            self.iter().map(|(key, _)| key).collect::<Vec<_>>().into_iter(),
            PhantomData
        )
    }
}

impl Json for JsonValue {
    type Number = Number;
    type Array = Array;
    type Object = Object;

    fn null() -> Self {
        JsonValue::Null
    }

    fn from_bool(val: bool) -> Self {
        JsonValue::from(val)
    }

    fn from_i64(val: i64) -> Self {
        JsonValue::from(val)
    }

    fn from_f64(val: f64) -> Self {
        JsonValue::from(val)
    }

    fn from_str(val: String) -> Self {
        JsonValue::from(val)
    }

    fn as_ref(&self) -> JsonRef<'_, Self> {
        match self {
            JsonValue::Null => JsonRef::Null,
            JsonValue::Short(s) => JsonRef::String(s.as_str()),
            JsonValue::String(s) => JsonRef::String(s.as_str()),
            JsonValue::Number(n) => JsonRef::Number(n),
            JsonValue::Boolean(b) => JsonRef::Bool(*b),
            JsonValue::Object(o) => JsonRef::Object(o),
            JsonValue::Array(a) => JsonRef::Array(a),
        }
    }

    fn as_mut(&mut self) -> JsonMut<'_, Self> {
        match self {
            JsonValue::Null => JsonMut::Null,
            JsonValue::Boolean(b) => JsonMut::Bool(b),
            JsonValue::Number(n) => JsonMut::Number(n),
            // HACK: Dirty little lie because we never care about this case
            JsonValue::Short(_) => JsonMut::Null,
            JsonValue::String(s) => JsonMut::String(s),
            JsonValue::Array(a) => JsonMut::Array(a),
            JsonValue::Object(o) => JsonMut::Object(o),
        }
    }
}
