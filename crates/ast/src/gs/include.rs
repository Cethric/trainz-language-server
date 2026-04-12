use crate::find::HasRange;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Include {
    pub path: Option<PathBuf>,
    pub path_range: Option<crate::Range>,
    pub name: String,
    pub range: crate::Range,
    pub keyword_include_range: crate::Range,
}

impl HasRange for Include {
    fn range(&self) -> crate::Range {
        self.range
    }
}
