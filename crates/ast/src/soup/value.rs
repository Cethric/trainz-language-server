#[derive(Debug, Clone)]
pub enum Value {
    Array(Vec<NumericValue>, crate::Range),
    Numeric(NumericValue, crate::Range),
    String(String, crate::Range),
    Variable(String, crate::Range),
    Kuid(crate::soup::kuid::Kuid, crate::Range),
    Container(Vec<crate::soup::KeyValuePair>, crate::Range),
}

impl Value {
    pub fn range(&self) -> crate::Range {
        match self {
            Value::Array(_, range) => *range,
            Value::Numeric(_, range) => *range,
            Value::String(_, range) => *range,
            Value::Variable(_, range) => *range,
            Value::Kuid(_, range) => *range,
            Value::Container(_, range) => *range,
        }
    }
}

#[derive(Debug, Clone)]
pub enum NumericValue {
    Float(f64),
    Hex(u64),
    Int(i64),
}
