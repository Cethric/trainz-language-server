use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct SoupValidator {
    pub key_to_check: String,
    pub allowed_values: HashMap<String, String>, // value -> human readable name
}

#[derive(Debug, Clone, Default)]
pub struct ContainerRule {
    pub key: String,
    pub type_name: Option<String>,
    pub kind: Option<String>,
    pub default_value: Option<String>,
    pub description: Option<String>,
    pub validation: Option<String>,
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
    pub array_element: Option<String>, // container type for elements
    pub validation: Option<String>,
    pub inherit: Vec<String>,
    pub top_level: bool,
    pub subpossibilities: Vec<ContainerRule>,
    pub tag_array: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Validators {
    pub simple: Vec<SoupValidator>,
    pub containers: Vec<ContainerValidator>,
    pub category_classes: HashMap<String, String>,
    pub category_regions: HashMap<String, String>,
    pub category_eras: HashMap<String, String>,
}
