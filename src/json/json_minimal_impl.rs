
use super::{Json, JsonRef, JsonMut, JsonObject};

use json_minimal::Json as Value;
use crate::json::{ObjectIter, ObjectValues};

impl JsonObject<Value> for Vec<Value> {
    fn get(&self, key: &str) -> Option<&Value> {
        todo!()
    }

    fn get_mut(&mut self, key: &str) -> Option<&mut Value> {
        todo!()
    }

    fn remove(&mut self, key: &str) {
        todo!()
    }

    fn iter(&self) -> ObjectIter<'_, Self, Value> {
        todo!()
    }

    fn values(&self) -> ObjectValues<'_, Self, Value> {
        todo!()
    }
}

impl Json for Value {
    type Number = f64;
    type Array = Vec<Value>;
    type Object = Vec<Value>;

    fn null() -> Self {
        Value::NULL
    }

    fn from_bool(val: bool) -> Self {
        Value::BOOL(val)
    }

    fn from_i64(val: i64) -> Self {
        Value::NUMBER(val.into())
    }

    fn from_f64(val: f64) -> Self {
        Value::NUMBER(val)
    }

    fn from_str(val: String) -> Self {
        Value::STRING(val)
    }

    fn as_ref(&self) -> JsonRef<'_, Self> {
        match self {
            Value::NULL => JsonRef::Null,
            Value::OBJECT { .. } => unreachable!(),
            Value::JSON(o) => JsonRef::Object(o),
            Value::ARRAY(a) => JsonRef::Array(a),
            Value::STRING(s) => JsonRef::String(s),
            Value::NUMBER(n) => JsonRef::Number(n),
            Value::BOOL(b) => JsonRef::Bool(b),
        }
    }

    fn as_mut(&mut self) -> JsonMut<'_, Self> {
        match self {
            Value::NULL => JsonMut::Null,
            Value::OBJECT { .. } => unreachable!(),
            Value::JSON(o) => JsonMut::Object(o),
            Value::ARRAY(a) => JsonMut::Array(a),
            Value::STRING(s) => JsonMut::String(s),
            Value::NUMBER(n) => JsonMut::Number(n),
            Value::BOOL(b) => JsonMut::Bool(b),
        }
    }
}
