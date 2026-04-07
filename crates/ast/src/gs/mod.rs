pub mod class;
pub mod expr;
pub mod find;
pub mod include;
pub mod literal;
pub mod process;
pub mod stmt;
pub mod types;

pub use class::*;
pub use expr::*;
pub use include::*;
pub use literal::*;
pub use stmt::*;
pub use types::*;

use crate::find::HasRange;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Program {
    pub includes: Vec<Include>,
    pub classes: Vec<ClassDef>,
    pub range: crate::Range,
}

impl HasRange for Program {
    fn range(&self) -> crate::Range {
        self.range
    }
}
