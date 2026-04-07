pub mod kuid;
pub mod process;
pub mod value;

pub use kuid::Kuid;
pub use value::{NumericValue, Value};

#[derive(Debug, Clone)]
pub struct Soup {
    pub key_value_pairs: Vec<KeyValuePair>,
    pub range: crate::Range,
    pub src: String,
}

#[derive(Debug, Clone)]
pub struct KeyValuePair {
    pub key: String,
    pub key_range: crate::Range,
    pub value: Option<Value>,
    pub range: crate::Range,
}
