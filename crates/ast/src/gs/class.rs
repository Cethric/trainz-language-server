use crate::find::HasRange;
use crate::gs::expr::Expr;
use crate::gs::literal::Identifier;
use crate::gs::stmt::Block;
use crate::gs::types::{Type, TypeOrVoid};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Display;

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
    pub scope_id: usize,
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

impl Display for ClassModifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClassModifier::Final => write!(f, "final"),
            ClassModifier::Game => write!(f, "game"),
            ClassModifier::Static => write!(f, "static"),
            ClassModifier::Secured => write!(f, "secured"),
            ClassModifier::Obsolete(v) => {
                if let Some(v) = v {
                    write!(f, "obsolete({})", v)
                } else {
                    write!(f, "obsolete")
                }
            }
        }
    }
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

impl Display for FieldModifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldModifier::Static => write!(f, "static"),
            FieldModifier::Public => write!(f, "public"),
            FieldModifier::Define => write!(f, "define"),
            FieldModifier::Obsolete(v) => {
                if let Some(v) = v {
                    write!(f, "obsolete({})", v)
                } else {
                    write!(f, "obsolete")
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodDef {
    pub modifiers: Vec<(MethodModifier, crate::Range)>,
    pub return_type: TypeOrVoid,
    pub name: Identifier,
    pub params: Vec<Param>,
    pub body: Option<Block>,
    pub scope_id: usize,
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

impl Display for MethodModifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MethodModifier::Static => write!(f, "static"),
            MethodModifier::Public => write!(f, "public"),
            MethodModifier::Thread => write!(f, "thread"),
            MethodModifier::LegacyCompatibility => write!(f, "legacy_compatibility"),
            MethodModifier::Mandatory => write!(f, "mandatory"),
            MethodModifier::Obsolete(v) => {
                if let Some(v) = v {
                    write!(f, "obsolete({})", v)
                } else {
                    write!(f, "obsolete")
                }
            }
            MethodModifier::Native => write!(f, "native"),
        }
    }
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
