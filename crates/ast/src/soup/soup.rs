use crate::soup::key_value_pair::KeyValuePair;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Soup {
    pub key_value_pairs: Vec<KeyValuePair>,
    pub range: crate::Range,
    pub src: String,
}
