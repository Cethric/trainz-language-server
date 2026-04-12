use crate::Range;
use crate::gs::{Identifier, Type};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Scope {
    pub id: usize,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub variables: Vec<(Type, Identifier)>,
    pub range: Range,
}
