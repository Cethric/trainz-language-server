use crate::find::HasRange;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Literal {
    String(StringLiteral),
    Char(char, crate::Range),
    Float(f64, crate::Range),
    Int(i64, crate::Range),
    Hex(u64, crate::Range),
    Bool(bool, crate::Range),
    Null(crate::Range),
}

impl HasRange for Literal {
    fn range(&self) -> crate::Range {
        match self {
            Literal::String(lit) => lit.range(),
            Literal::Char(_, range) => *range,
            Literal::Float(_, range) => *range,
            Literal::Int(_, range) => *range,
            Literal::Hex(_, range) => *range,
            Literal::Bool(_, range) => *range,
            Literal::Null(range) => *range,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StringLiteral {
    pub value: String,
    pub range: crate::Range,
}

impl HasRange for StringLiteral {
    fn range(&self) -> crate::Range {
        self.range
    }
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identifier {
    pub name: String,
    pub range: crate::Range,
}

impl HasRange for Identifier {
    fn range(&self) -> crate::Range {
        self.range
    }
}
