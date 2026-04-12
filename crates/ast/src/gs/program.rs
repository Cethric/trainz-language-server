use crate::find::HasRange;
use crate::gs::{ClassDef, Include};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Program {
    pub includes: Vec<Include>,
    pub classes: HashMap<String, ClassDef>,
    pub range: crate::Range,
}

impl HasRange for Program {
    fn range(&self) -> crate::Range {
        self.range
    }
}
