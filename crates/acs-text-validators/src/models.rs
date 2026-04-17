use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum ArrayElementType {
    Array(String, String), // key, type_name
    Tuple(Vec<(String, String)>),
    Inline(Box<ContainerValidator>),
    Rule(Box<ContainerRule>),
}

impl fmt::Display for ArrayElementType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArrayElementType::Array(k, t) => write!(f, "Array<{}: {}>", k, t),
            ArrayElementType::Tuple(v) => write!(
                f,
                "Tuple<{}>",
                v.iter()
                    .map(|(k, t)| format!("{}: {}", k, t))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            ArrayElementType::Inline(_) => write!(f, "InlineContainer"),
            ArrayElementType::Rule(r) => write!(f, "Rule({})", r.key),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Validation {
    Named(String),
    IntRange(i64, i64),
    HexRange(u64, u64),
    FloatRange(f64, f64),
    NeedCollateMeshes(Vec<String>),
    NotOwnParent,
    MustBePaired(Vec<String>),
    FilepathTableFilesExist,
}

#[derive(Debug, Clone, Default, PartialEq)]
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
    pub obsolete_version: Option<f64>,
    pub obsolete_message: Option<String>,
    pub minimum_version: Option<f64>,
    pub source: Option<String>,
    pub child_validator: Option<Box<ContainerValidator>>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ContainerValidator {
    pub container_name: String,
    pub rules: Vec<ContainerRule>,
    pub array_element: Option<ArrayElementType>, // Updated: can be an array of one type or a tuple of multiple types
    pub validation: Option<Vec<Validation>>,
    pub inherit: Vec<String>,
    pub top_level: bool,
    pub sub_possibilities: Vec<ContainerRule>,
    pub tag_array: Option<ArrayElementType>,
    pub allow_any_key: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Validators {
    pub simple: HashMap<String, HashMap<String, Option<String>>>,
    pub containers: Vec<ContainerValidator>,
    pub container_map: HashMap<String, ContainerValidator>,
}

impl fmt::Display for Validation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Validation::IntRange(a, b) => write!(f, "Range: {}-{}", a, b),
            Validation::HexRange(a, b) => write!(f, "HexRange: {}-{}", a, b),
            Validation::FloatRange(a, b) => write!(f, "FloatRange: {}-{}", a, b),
            Validation::NeedCollateMeshes(v) => write!(f, "NeedCollateMeshes: {}", v.join(", ")),
            Validation::NotOwnParent => write!(f, "NotOwnParent"),
            Validation::Named(s) => write!(f, "{}", s),
            Validation::MustBePaired(keys) => write!(f, "MustBePaired({})", keys.join(", ")),
            Validation::FilepathTableFilesExist => write!(f, "FilepathTableFilesExist"),
        }
    }
}
