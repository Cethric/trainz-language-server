use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Value {
    Array(Vec<NumericValue>, crate::Range),
    Numeric(NumericValue, crate::Range),
    String(String, crate::Range),
    Variable(String, crate::Range),
    Kuid(crate::soup::kuid::Kuid, crate::Range),
    Container(Vec<crate::soup::KeyValuePair>, crate::Range, crate::Range),
}

impl Value {
    pub fn range(&self) -> crate::Range {
        match self {
            Value::Array(_, range) => *range,
            Value::Numeric(_, range) => *range,
            Value::String(_, range) => *range,
            Value::Variable(_, range) => *range,
            Value::Kuid(_, range) => *range,
            Value::Container(_, content_range, _) => *content_range,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NumericValue {
    Float(f64),
    Hex(u64),
    Int(i64),
}

impl Display for NumericValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            NumericValue::Float(val) => val.to_string(),
            NumericValue::Hex(val) => val.to_string(),
            NumericValue::Int(val) => val.to_string(),
        };
        write!(f, "{}", str)
    }
}
