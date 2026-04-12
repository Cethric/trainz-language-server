use crate::find::HasRange;
use crate::gs::expr::Expr;
use crate::gs::literal::Identifier;
use crate::gs::stmt::Block;
use crate::gs::types::{Type, TypeOrVoid};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassDef {
    pub modifiers: Vec<(ClassModifier, crate::Range)>,
    pub keyword_class_range: crate::Range,
    pub name: Identifier,
    pub keyword_is_class_range: Option<crate::Range>,
    pub superclasses: Vec<Identifier>,
    pub fields: HashMap<String, FieldDef>,
    pub methods: HashMap<String, Vec<MethodDef>>,
    pub body_range: crate::Range,
    pub range: crate::Range,
}

impl HasRange for ClassDef {
    fn range(&self) -> crate::Range {
        self.range
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClassModifier {
    Final,
    Game,
    Static,
    Secured,
    Obsolete(Option<i64>), // obsolete(123)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDef {
    pub modifiers: Vec<(FieldModifier, crate::Range)>,
    pub ty: Type,
    pub name: Identifier,
    pub initializer: Option<Expr>,
    pub range: crate::Range,
}

impl HasRange for FieldDef {
    fn range(&self) -> crate::Range {
        self.range
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FieldModifier {
    Static,
    Public,
    Define,
    Obsolete(Option<i64>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodDef {
    pub modifiers: Vec<(MethodModifier, crate::Range)>,
    pub return_type: TypeOrVoid,
    pub name: Identifier,
    pub params: Vec<Param>,
    pub body: Option<Block>,
    pub range: crate::Range,
}

impl HasRange for MethodDef {
    fn range(&self) -> crate::Range {
        self.range
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MethodModifier {
    Static,
    Public,
    Thread,
    LegacyCompatibility,
    Mandatory,
    Obsolete(Option<i64>),
    Native, // for native methods
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Param {
    pub ty: Type,
    pub name: Identifier,
    pub range: crate::Range,
}

impl HasRange for Param {
    fn range(&self) -> crate::Range {
        self.range
    }
}
