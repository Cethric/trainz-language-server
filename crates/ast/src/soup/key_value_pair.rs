use crate::soup::Value;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyValuePair {
    pub key: String,
    pub key_range: crate::Range,
    pub value: Option<Value>,
    pub range: crate::Range,
}
