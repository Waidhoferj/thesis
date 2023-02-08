use serde::{Deserialize, Serialize};
use serde_json::{self, json, Value as JSON};

use std::clone::Clone;
use std::cmp::Ordering;
use std::fmt::Debug;
use std::fmt::Display;

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub enum Value {
    String(String),
    Int(isize),
    Float(f32),
    Bool(bool),
    Array(Vec<Value>),
    Null,
}

impl Value {
    #[inline(always)]
    fn type_rank(value: &Value) -> u8 {
        match value {
            Value::Array(_) => 5,
            Value::String(_) => 4,
            Value::Int(_) => 3,
            Value::Float(_) => 2,
            Value::Bool(_) => 1,
            Value::Null => 0,
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let repr = match self {
            Value::String(s) => format!("\"{}\"", s),
            Value::Int(i) => format!("{}", i),
            Value::Float(f) => format!("{}", f),
            Value::Bool(b) => format!("{}", b),
            Value::Array(a) => format!("{:?}", a),
            Value::Null => "null".to_string(),
        };
        write!(f, "{repr}")
    }
}

impl Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return std::fmt::Display::fmt(&self, f);
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match Value::type_rank(self).cmp(&Value::type_rank(other)) {
            Ordering::Equal => match (self, other) {
                (Value::Bool(b1), Value::Bool(b2)) => b1.partial_cmp(b2),
                (Value::Int(v1), Value::Int(v2)) => v1.partial_cmp(v2),
                (Value::Float(v1), Value::Float(v2)) => v1.partial_cmp(v2),
                (Value::String(v1), Value::String(v2)) => v1.partial_cmp(v2),
                (Value::Array(v1), Value::Array(v2)) => v1.partial_cmp(v2),
                (Value::Null, Value::Null) => Some(Ordering::Equal),
                _ => unreachable!("If type ranks match, they must be the same type."),
            },
            ord => Some(ord),
        }
    }
}

impl TryFrom<JSON> for Value {
    type Error = String;
    fn try_from(json: JSON) -> Result<Self, Self::Error> {
        match json {
            JSON::Bool(b) => Ok(Value::Bool(b)),
            JSON::Number(n) if n.is_i64() => Ok(Value::Int(n.as_i64().unwrap() as isize)),
            JSON::Number(n) if n.is_f64() => Ok(Value::Float(n.as_f64().unwrap() as f32)),
            JSON::String(s) => Ok(Value::String(s)),
            JSON::Array(a) => {
                let array: Result<Vec<Value>, String> =
                    a.into_iter().map(|json_val| json_val.try_into()).collect();
                Ok(Value::Array(array?))
            }
            JSON::Null => Ok(Value::Null),
            _ => Err("Should not be building with null or object values.".to_string()),
        }
    }
}

impl From<Value> for JSON {
    fn from(value: Value) -> Self {
        match value {
            Value::String(s) => json!(s),
            Value::Int(i) => json!(i),
            Value::Float(f) => json!(f),
            Value::Bool(b) => json!(b),
            Value::Array(a) => {
                let arr: Vec<JSON> = a.into_iter().map(JSON::from).collect();
                json!(arr)
            }
            Value::Null => JSON::Null,
        }
    }
}

impl From<isize> for Value {
    fn from(value: isize) -> Self {
        Value::Int(value)
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Value::String(value)
    }
}

impl From<f32> for Value {
    fn from(value: f32) -> Self {
        Value::Float(value)
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::Bool(value)
    }
}

impl From<Vec<Value>> for Value {
    fn from(value: Vec<Value>) -> Self {
        Value::Array(value)
    }
}
