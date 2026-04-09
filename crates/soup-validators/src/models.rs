use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum ArrayElementType {
    Array(String),
    Tuple(Vec<String>),
}

impl fmt::Display for ArrayElementType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArrayElementType::Array(s) => write!(f, "Array<{}>", s),
            ArrayElementType::Tuple(v) => write!(f, "Tuple<{}>", v.join(", ")),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Validation {
    IntRange(i64, i64),
    HexRange(u64, u64),
    FloatRange(f64, f64),
    NeedCollateMeshes(Vec<String>),
    NotOwnParent,
    Named(String),
}

#[derive(Debug, Clone, Default)]
pub struct ContainerRule {
    pub key: String,
    pub type_name: Option<String>,
    pub kind: Option<String>,
    pub default_value: Option<String>,
    pub description: Option<String>,
    pub validation: Option<Vec<Validation>>,
    pub compulsory: Option<f64>,
    pub filter: Option<String>,
    pub disabled: Option<bool>,
    pub obsolete_tag: Option<bool>,
    pub array_element: Option<Box<ContainerValidator>>,
}

#[derive(Debug, Clone, Default)]
pub struct ContainerValidator {
    pub container_name: String,
    pub rules: Vec<ContainerRule>,
    pub array_element: Option<ArrayElementType>, // Updated: can be an array of one type or a tuple of multiple types
    pub validation: Option<Vec<Validation>>,
    pub inherit: Vec<String>,
    pub top_level: bool,
    pub sub_possibilities: Vec<ContainerRule>,
    pub tag_array: Option<ContainerRule>,
    pub allow_any_key: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Validators {
    pub simple: HashMap<String, HashMap<String, Option<String>>>,
    pub containers: Vec<ContainerValidator>,
    pub container_map: HashMap<String, ContainerValidator>,
}
