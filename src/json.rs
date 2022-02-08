use std::collections::HashMap;
use std::marker::PhantomData;
use std::vec::IntoIter;
use crate::Idx;

mod serde_impl;
mod json_impl;
mod json_minimal_impl;

pub enum JsonRef<'a, T: Json> {
    Null,
    Bool(bool),
    Number(&'a T::Number),
    String(&'a str),
    Array(&'a T::Array),
    Object(&'a T::Object),
}

pub enum JsonMut<'a, T: Json> {
    Null,
    Bool(&'a mut bool),
    Number(&'a mut T::Number),
    String(&'a mut String),
    Array(&'a mut T::Array),
    Object(&'a mut T::Object),
}

pub trait JsonNumber: ToString {
    fn as_u64(&self) -> Option<u64>;
    fn as_i64(&self) -> Option<i64>;
    fn as_f64(&self) -> Option<f64>;
}

pub struct ArrayIter<'a, T: ?Sized + JsonArray<V>, V>(&'a T, usize, PhantomData<V>);

impl<'a, T: JsonArray<V>, V: Json + 'a> Iterator for ArrayIter<'a, T, V> {
    type Item = &'a V;

    fn next(&mut self) -> Option<Self::Item> {
        let out = self.0.get(self.1)?;
        self.1 += 1;
        Some(out)
    }
}

pub trait JsonArray<Value> {
    fn len(&self) -> usize;
    fn get(&self, idx: usize) -> Option<&Value>;
    fn get_mut(&mut self, idx: usize) -> Option<&mut Value>;
    fn remove(&mut self, idx: usize);
    fn iter(&self) -> ArrayIter<'_, Self, Value>;
}

impl<T: Json> JsonArray<T> for Vec<T> {
    fn len(&self) -> usize {
        self.len()
    }

    fn get(&self, idx: usize) -> Option<&T> {
        <[T]>::get(self, idx)
    }

    fn get_mut(&mut self, idx: usize) -> Option<&mut T> {
        <[T]>::get_mut(self, idx)
    }

    fn remove(&mut self, idx: usize) {
        self.remove(idx);
    }

    fn iter(&self) -> ArrayIter<'_, Self, T> {
        ArrayIter(self, 0, PhantomData)
    }
}

pub struct ObjectIter<'a, T: ?Sized + JsonObject<V>, V>(&'a T, IntoIter<&'a str>, PhantomData<V>);

impl<'a, T: JsonObject<V>, V: 'a> Iterator for ObjectIter<'a, T, V> {
    type Item = (&'a str, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        let key = self.1.next()?;
        Some((key, self.0.get(key)?))
    }
}

pub struct ObjectValues<'a, T: ?Sized + JsonObject<V>, V>(&'a T, IntoIter<&'a str>, PhantomData<V>);

impl<'a, T: JsonObject<V>, V: 'a> Iterator for ObjectValues<'a, T, V> {
    type Item = &'a V;

    fn next(&mut self) -> Option<Self::Item> {
        let key = self.1.next()?;
        Some(self.0.get(key)?)
    }
}

pub trait JsonObject<Value> {
    fn get(&self, key: &str) -> Option<&Value>;
    fn get_mut(&mut self, key: &str) -> Option<&mut Value>;
    fn remove(&mut self, key: &str);
    fn iter(&self) -> ObjectIter<'_, Self, Value>;
    fn values(&self) -> ObjectValues<'_, Self, Value>;
}

impl<T: Json> JsonObject<T> for HashMap<String, T> {
    fn get(&self, key: &str) -> Option<&T> {
        self.get(key)
    }

    fn get_mut(&mut self, key: &str) -> Option<&mut T> {
        self.get_mut(key)
    }

    fn remove(&mut self, key: &str) {
        self.remove(key);
    }

    fn iter(&self) -> ObjectIter<'_, Self, T> {
        ObjectIter(self, self.keys().map(|a| &**a).collect::<Vec<_>>().into_iter(), PhantomData)
    }

    fn values(&self) -> ObjectValues<'_, Self, T> {
        ObjectValues(self, self.keys().map(|a| &**a).collect::<Vec<_>>().into_iter(), PhantomData)
    }
}

pub trait Json: Clone + PartialEq {
    type Number: JsonNumber;
    type Array: JsonArray<Self>;
    type Object: JsonObject<Self>;

    fn null() -> Self;
    fn from_bool(val: bool) -> Self;
    fn from_i64(val: i64) -> Self;
    fn from_f64(val: f64) -> Self;
    fn from_str(val: String) -> Self;

    fn from_idx(idx: Idx) -> Self {
        match idx {
            Idx::Array(i) => Self::from_i64(i as i64),
            Idx::Object(s) => Self::from_str(s),
        }
    }

    fn as_ref(&self) -> JsonRef<'_, Self>;
    fn as_mut(&mut self) -> JsonMut<'_, Self>;

    fn is_bool(&self) -> bool {
        matches!(self.as_ref(), JsonRef::Bool(_))
    }

    fn is_f64(&self) -> bool {
        self.as_f64().is_some()
    }

    fn is_string(&self) -> bool {
        matches!(self.as_ref(), JsonRef::String(_))
    }

    fn as_bool(&self) -> Option<bool> {
        if let JsonRef::Bool(b) = self.as_ref() {
            Some(b)
        } else {
            None
        }
    }

    fn as_i64(&self) -> Option<i64> {
        if let JsonRef::Number(n) = self.as_ref() {
            n.as_i64()
        } else {
            None
        }
    }

    fn as_f64(&self) -> Option<f64> {
        if let JsonRef::Number(n) = self.as_ref() {
            n.as_f64()
        } else {
            None
        }
    }

    fn as_str(&self) -> Option<&str> {
        if let JsonRef::String(s) = self.as_ref() {
            Some(s)
        } else {
            None
        }
    }

    fn as_array_mut(&mut self) -> Option<&mut Self::Array> {
        if let JsonMut::Array(a) = self.as_mut() {
            Some(a)
        } else {
            None
        }
    }

    fn as_object_mut(&mut self) -> Option<&mut Self::Object> {
        if let JsonMut::Object(a) = self.as_mut() {
            Some(a)
        } else {
            None
        }
    }
}
