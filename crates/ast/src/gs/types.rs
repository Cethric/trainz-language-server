use crate::find::HasRange;
use crate::gs::Identifier;
use serde::{Deserialize, Serialize};

use std::fmt::Display;

/// Represents a GS type.
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

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Bool(_) => write!(f, "bool"),
            Type::Int(_) => write!(f, "int"),
            Type::Float(_) => write!(f, "float"),
            Type::Object(_) => write!(f, "object"),
            Type::String(_) => write!(f, "string"),
            Type::Named(id) => write!(f, "{}", id.name),
            Type::Array(inner, _) => write!(f, "{}[]", inner),
        }
    }
}

impl Type {
    /// Sets the range of the type.
    ///
    /// # Arguments
    /// * `range` - The new range to set.
    ///
    /// # Examples
    ///
    /// ```rust
    /// // type_obj.set_range(new_range);
    /// ```
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

impl Default for TypeOrVoid {
    fn default() -> Self {
        Self::Void(Default::default())
    }
}

impl Display for TypeOrVoid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeOrVoid::Void(_) => write!(f, "void"),
            TypeOrVoid::Type(t) => write!(f, "{}", t),
        }
    }
}

impl HasRange for TypeOrVoid {
    fn range(&self) -> crate::Range {
        match self {
            TypeOrVoid::Void(r) => *r,
            TypeOrVoid::Type(t) => t.range(),
        }
    }
}
