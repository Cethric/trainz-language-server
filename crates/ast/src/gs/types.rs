use crate::find::HasRange;
use crate::gs::Identifier;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Type {
    Bool(crate::Range),
    Int(crate::Range),
    Float(crate::Range),
    Object(crate::Range),
    String(crate::Range),
    Named(Identifier), // type_identifier
    Array(Box<Type>, crate::Range),
}

impl Type {
    pub fn set_range(&mut self, range: crate::Range) {
        match self {
            Type::Bool(r) => *r = range,
            Type::Int(r) => *r = range,
            Type::Float(r) => *r = range,
            Type::Object(r) => *r = range,
            Type::String(r) => *r = range,
            Type::Named(id) => id.range = range,
            Type::Array(_, r) => *r = range,
        }
    }
}

impl HasRange for Type {
    fn range(&self) -> crate::Range {
        match self {
            Type::Bool(r) => *r,
            Type::Int(r) => *r,
            Type::Float(r) => *r,
            Type::Object(r) => *r,
            Type::String(r) => *r,
            Type::Named(id) => id.range,
            Type::Array(_, r) => *r,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypeOrVoid {
    Void(crate::Range),
    Type(Type),
}

impl HasRange for TypeOrVoid {
    fn range(&self) -> crate::Range {
        match self {
            TypeOrVoid::Void(r) => *r,
            TypeOrVoid::Type(t) => t.range(),
        }
    }
}
