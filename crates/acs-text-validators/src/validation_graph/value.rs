use crate::util::{
    parse_as_array_option, parse_as_bool, parse_as_numeric, parse_as_string, parse_file,
    parse_source,
};
use anyhow::Result;
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::Path;
use tracing::{trace, warn};
use trainz_ast::acs_text::{KeyValuePair, Value};

#[derive(Debug, Clone)]
pub enum RuleNodeValue {
    String(RuleNodeValueString),
    Float(RuleNodeValueFloat),
    Integer(RuleNodeValueInteger),
    Bool(RuleNodeValueBool),
    Rgb(RuleNodeValueRgb),
    ComboBox(RuleNodeValueComboBox),
    IntComboBox(RuleNodeValueIntComboBox),
    FloatComboBox(RuleNodeValueFloatComboBox),
    ListBox(RuleNodeValueListBox),
    Kuid(RuleNodeValueKuid),
    KuidBrowser(RuleNodeValueKuidBrowser),
    FilePath(RuleNodeValueFilePath),
    FloatList(RuleNodeValueFloatList),
    Vector2(RuleNodeValueVector2),
    Vector3(RuleNodeValueVector3),
    Vector4(RuleNodeValueVector4),
    Vector5(RuleNodeValueVector5),
    Vector6(RuleNodeValueVector6),
}

impl RuleNodeValue {
    #[tracing::instrument(skip(key, entries))]
    pub(crate) fn new(key: &str, entries: &Vec<KeyValuePair>) -> Option<Self> {
        if key.eq_ignore_ascii_case("trainz-build") {
            return Some(RuleNodeValue::FloatComboBox(
                RuleNodeValueFloatComboBox::new(
                    entries,
                    parse_source(include_str!("../custom-validators/trainz-build.txt"))
                        .ok()
                        .map(|processed| processed.key_value_pairs),
                ),
            ));
        }

        for entry in entries {
            if entry.key.eq_ignore_ascii_case("type") {
                let value_type = parse_as_string(entry);
                if let Some(value_type) = value_type {
                    if value_type.eq_ignore_ascii_case("string")
                        || value_type.eq_ignore_ascii_case("string-token")
                        || value_type.eq_ignore_ascii_case("doublestring")
                    {
                        return Some(RuleNodeValue::String(RuleNodeValueString::new(entries)));
                    }
                    if value_type.eq_ignore_ascii_case("float") {
                        return Some(RuleNodeValue::Float(RuleNodeValueFloat::new(entries)));
                    }
                    if value_type.eq_ignore_ascii_case("int") {
                        return Some(RuleNodeValue::Integer(RuleNodeValueInteger::new(entries)));
                    }
                    if value_type.eq_ignore_ascii_case("bool") {
                        return Some(RuleNodeValue::Bool(RuleNodeValueBool::new(entries)));
                    }
                    if value_type.eq_ignore_ascii_case("rgb") {
                        return Some(RuleNodeValue::Rgb(RuleNodeValueRgb::new(entries)));
                    }
                    if value_type.eq_ignore_ascii_case("combobox") {
                        return Some(RuleNodeValue::ComboBox(RuleNodeValueComboBox::new(
                            entries, None,
                        )));
                    }
                    if value_type.eq_ignore_ascii_case("intcombobox") {
                        return Some(RuleNodeValue::IntComboBox(RuleNodeValueIntComboBox::new(
                            entries, None,
                        )));
                    }
                    if value_type.eq_ignore_ascii_case("listbox") {
                        return Some(RuleNodeValue::ListBox(RuleNodeValueListBox::new(
                            entries, None,
                        )));
                    }
                    if value_type.eq_ignore_ascii_case("kuid") {
                        return Some(RuleNodeValue::Kuid(RuleNodeValueKuid::new(entries)));
                    }
                    if value_type.eq_ignore_ascii_case("kuidbrowser")
                        || value_type.eq_ignore_ascii_case("stringkuidbrowser")
                    {
                        return Some(RuleNodeValue::KuidBrowser(RuleNodeValueKuidBrowser::new(
                            entries,
                        )));
                    }
                    if value_type.eq_ignore_ascii_case("filepath")
                        || value_type.eq_ignore_ascii_case("filepathedit")
                        || value_type.eq_ignore_ascii_case("filepath-token")
                    {
                        return Some(RuleNodeValue::FilePath(RuleNodeValueFilePath::new(entries)));
                    }
                    if value_type.eq_ignore_ascii_case("floatlist") {
                        return Some(RuleNodeValue::FloatList(RuleNodeValueFloatList::new(
                            entries,
                        )));
                    }
                    if value_type.eq_ignore_ascii_case("vector2") {
                        return Some(RuleNodeValue::Vector2(RuleNodeValueVector2::new(entries)));
                    }
                    if value_type.eq_ignore_ascii_case("vector3") {
                        return Some(RuleNodeValue::Vector3(RuleNodeValueVector3::new(entries)));
                    }
                    if value_type.eq_ignore_ascii_case("vector4") {
                        return Some(RuleNodeValue::Vector4(RuleNodeValueVector4::new(entries)));
                    }
                    if value_type.eq_ignore_ascii_case("vector5") {
                        return Some(RuleNodeValue::Vector5(RuleNodeValueVector5::new(entries)));
                    }
                    if value_type.eq_ignore_ascii_case("vector6") {
                        return Some(RuleNodeValue::Vector6(RuleNodeValueVector6::new(entries)));
                    }
                }
            }
        }

        warn!(
            "Unknown type for RuleNodeValue {{ entries: {:?} }}",
            entries
        );

        None
    }
}

impl RuleNodeValue {
    #[tracing::instrument(skip(self, validation_path))]
    pub(crate) async fn update_sources(&mut self, validation_path: &Path) -> Result<()> {
        match self {
            RuleNodeValue::String(_) => {}
            RuleNodeValue::Float(_) => {}
            RuleNodeValue::Integer(_) => {}
            RuleNodeValue::Bool(_) => {}
            RuleNodeValue::Rgb(_) => {}
            RuleNodeValue::ComboBox(combobox) => combobox.update_sources(validation_path).await?,
            RuleNodeValue::IntComboBox(int_combobox) => {
                int_combobox.update_sources(validation_path).await?
            }
            RuleNodeValue::FloatComboBox(float_combobox) => {
                float_combobox.update_sources(validation_path).await?
            }
            RuleNodeValue::ListBox(listbox) => listbox.update_sources(validation_path).await?,
            RuleNodeValue::Kuid(_) => {}
            RuleNodeValue::KuidBrowser(_) => {}
            RuleNodeValue::FilePath(_) => {}
            RuleNodeValue::FloatList(_) => {}
            RuleNodeValue::Vector2(_) => {}
            RuleNodeValue::Vector3(_) => {}
            RuleNodeValue::Vector4(_) => {}
            RuleNodeValue::Vector5(_) => {}
            RuleNodeValue::Vector6(_) => {}
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct RuleNodeValueString {
    default: Option<String>,
}

impl RuleNodeValueString {
    #[tracing::instrument(skip(entries))]
    pub(crate) fn new(entries: &Vec<KeyValuePair>) -> Self {
        let mut default: Option<String> = None;
        for entry in entries {
            if entry.key.eq_ignore_ascii_case("default") {
                default = parse_as_string(entry);
            }
        }
        Self { default }
    }
}

#[derive(Debug, Clone)]
pub struct RuleNodeValueFloat {
    default: Option<f64>,
}

impl RuleNodeValueFloat {
    #[tracing::instrument(skip(entries))]
    fn new(entries: &Vec<KeyValuePair>) -> Self {
        let mut default: Option<f64> = None;
        for entry in entries {
            if entry.key.eq_ignore_ascii_case("default") {
                default = parse_as_numeric(entry);
            }
        }
        Self { default }
    }
}

#[derive(Debug, Clone)]
pub struct RuleNodeValueInteger {
    default: Option<i64>,
}

impl RuleNodeValueInteger {
    #[tracing::instrument(skip(entries))]
    fn new(entries: &Vec<KeyValuePair>) -> Self {
        let mut default: Option<i64> = None;
        for entry in entries {
            if entry.key.eq_ignore_ascii_case("default") {
                default = parse_as_numeric(entry);
            }
        }
        Self { default }
    }
}

#[derive(Debug, Clone)]
pub struct RuleNodeValueBool {
    default: Option<bool>,
}

impl RuleNodeValueBool {
    #[tracing::instrument(skip(entries))]
    fn new(entries: &Vec<KeyValuePair>) -> Self {
        let mut default: Option<bool> = None;
        for entry in entries {
            if entry.key.eq_ignore_ascii_case("default") {
                default = Some(parse_as_bool(entry));
            }
        }
        Self { default }
    }
}

#[derive(Debug, Clone)]
pub struct RuleNodeValueRgb {
    default: Option<(u8, u8, u8)>,
}

impl RuleNodeValueRgb {
    #[tracing::instrument(skip(entries))]
    pub(crate) fn new(entries: &Vec<KeyValuePair>) -> Self {
        let mut default: Option<(u8, u8, u8)> = None;
        for entry in entries {
            if entry.key.eq_ignore_ascii_case("default")
                && let Some(parsed) = parse_as_array_option::<u8>(entry)
                && parsed.len() == 3
            {
                default = Some((parsed[0], parsed[1], parsed[2]));
            }
        }
        Self { default }
    }
}

#[derive(Debug, Clone)]
pub struct RuleNodeValueComboBox {
    default: Option<String>,
    source: Option<String>,
    options: HashMap<String, Option<String>>,
}

impl RuleNodeValueComboBox {
    #[tracing::instrument(skip(self, validation_path))]
    pub(crate) async fn update_sources(&mut self, validation_path: &Path) -> Result<()> {
        if let Some(source) = &self.source {
            let source_path = validation_path.join(source).with_extension("txt");
            trace!("Updating source: {:?}", source_path);
            if source_path.exists() && source_path.is_file() {
                let source = parse_file(&source_path).await;

                if let Ok(source) = source {
                    let options = RuleNodeValueComboBox::build_options(source.key_value_pairs);
                    self.options.par_extend(options.clone());
                    trace!("Added options: {:?}", options);
                } else if let Err(source) = source {
                    warn!("Failed to parse source file: {:?}", source);
                    trace!("Failed to parse source file: {:?}", source);
                }
            }
        }

        Ok(())
    }

    #[tracing::instrument(skip(self, key))]
    pub fn has_value(&self, key: &String) -> bool {
        self.options.contains_key(key)
    }

    #[tracing::instrument(skip(self))]
    pub fn display_options(&self) -> String {
        self.options
            .par_iter()
            .map(|(key, value)| {
                if let Some(value) = value {
                    format!("- {}: {}", key, value)
                } else {
                    format!("- {}", key)
                }
            })
            .collect::<Vec<String>>()
            .join("\n")
    }

    #[tracing::instrument(skip(self))]
    pub fn display_options_inline(&self) -> String {
        self.options
            .par_iter()
            .map(|(key, _)| key.to_string())
            .collect::<Vec<String>>()
            .join(", ")
    }
}

impl RuleNodeValueComboBox {
    #[tracing::instrument(skip(key_value_pairs))]
    fn build_options(key_value_pairs: Vec<KeyValuePair>) -> HashMap<String, Option<String>> {
        let mut options: HashMap<String, Option<String>> = HashMap::new();
        for entry in key_value_pairs {
            let description = if let Some(value) = parse_as_string(&entry) {
                Some(value)
            } else {
                None
            };
            options.insert(entry.key.clone(), description);
        }

        options
    }

    #[tracing::instrument(skip(entries, initial_options))]
    fn new(entries: &Vec<KeyValuePair>, initial_options: Option<Vec<KeyValuePair>>) -> Self {
        let mut default: Option<String> = None;
        let mut source: Option<String> = None;
        let options: HashMap<String, Option<String>> =
            if let Some(initial_options) = initial_options {
                RuleNodeValueComboBox::build_options(initial_options)
            } else {
                HashMap::new()
            };

        for entry in entries {
            if entry.key.eq_ignore_ascii_case("source") {
                source = parse_as_string(entry);
            }
            if entry.key.eq_ignore_ascii_case("default") {
                default = parse_as_string(entry);
            }
        }

        Self {
            default,
            source,
            options,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuleNodeValueIntComboBox {
    default: Option<u64>,
    source: Option<String>,
    options: HashMap<u64, Option<String>>,
}

impl RuleNodeValueIntComboBox {
    #[tracing::instrument(skip(self, validation_path))]
    pub(crate) async fn update_sources(&mut self, validation_path: &Path) -> Result<()> {
        if let Some(source) = &self.source {
            let source_path = validation_path.join(source).with_extension("txt");
            trace!("Updating source: {:?}", source_path);
            if source_path.exists() && source_path.is_file() {
                let source = parse_file(&source_path).await;
                if let Ok(source) = source {
                    let options = RuleNodeValueIntComboBox::build_options(source.key_value_pairs);
                    self.options.par_extend(options.clone());
                    trace!("Added options: {:?}", options);
                } else if let Err(source) = source {
                    warn!("Failed to parse source file: {:?}", source);
                    trace!("Failed to parse source file: {:?}", source);
                }
            }
        }

        Ok(())
    }

    #[tracing::instrument(skip(self, key))]
    pub fn has_value(&self, key: &u64) -> bool {
        self.options.contains_key(key)
    }

    #[tracing::instrument(skip(self))]
    pub fn display_options(&self) -> String {
        self.options
            .par_iter()
            .map(|(key, value)| {
                if let Some(value) = value {
                    format!("- {}: {}", key, value)
                } else {
                    format!("- {}", key)
                }
            })
            .collect::<Vec<String>>()
            .join("\n")
    }

    #[tracing::instrument(skip(self))]
    pub fn display_options_inline(&self) -> String {
        self.options
            .par_iter()
            .map(|(key, _)| key.to_string())
            .collect::<Vec<String>>()
            .join(", ")
    }
}

impl RuleNodeValueIntComboBox {
    #[tracing::instrument(skip(key_value_pairs))]
    fn build_options(key_value_pairs: Vec<KeyValuePair>) -> HashMap<u64, Option<String>> {
        let mut options: HashMap<u64, Option<String>> = HashMap::new();
        for entry in key_value_pairs {
            let description = if let Some(value) = parse_as_string(&entry) {
                Some(value)
            } else {
                None
            };
            if let Ok(value) = entry.key.parse::<u64>() {
                options.insert(value, description);
            }
        }

        options
    }

    #[tracing::instrument(skip(entries, initial_options))]
    fn new(entries: &Vec<KeyValuePair>, initial_options: Option<Vec<KeyValuePair>>) -> Self {
        let mut default: Option<u64> = None;
        let mut source: Option<String> = None;
        let options: HashMap<u64, Option<String>> = if let Some(initial_options) = initial_options {
            RuleNodeValueIntComboBox::build_options(initial_options)
        } else {
            HashMap::new()
        };

        for entry in entries {
            if entry.key.eq_ignore_ascii_case("source") {
                source = parse_as_string(entry);
            }
            if entry.key.eq_ignore_ascii_case("default") {
                default = parse_as_numeric(entry);
            }
        }

        Self {
            default,
            source,
            options,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuleNodeValueFloatComboBox {
    default: Option<f64>,
    source: Option<String>,
    options: HashMap<String, Option<String>>,
}

impl RuleNodeValueFloatComboBox {
    #[tracing::instrument(skip(self, validation_path))]
    pub(crate) async fn update_sources(&mut self, validation_path: &Path) -> Result<()> {
        if let Some(source) = &self.source {
            let source_path = validation_path.join(source).with_extension("txt");
            trace!("Updating source: {:?}", source_path);
            if source_path.exists() && source_path.is_file() {
                let source = parse_file(&source_path).await;

                if let Ok(source) = source {
                    let options = RuleNodeValueFloatComboBox::build_options(source.key_value_pairs);
                    self.options.par_extend(options.clone());
                    trace!("Added options: {:?}", options);
                } else if let Err(source) = source {
                    warn!("Failed to parse source file: {:?}", source);
                    trace!("Failed to parse source file: {:?}", source);
                }
            }
        }

        Ok(())
    }

    #[tracing::instrument(skip(self, key))]
    pub fn has_value(&self, key: &str) -> bool {
        self.options.contains_key(key)
    }

    #[tracing::instrument(skip(self))]
    pub fn display_options(&self) -> String {
        self.options
            .par_iter()
            .map(|(key, value)| {
                if let Some(value) = value {
                    format!("- {}: {}", key, value)
                } else {
                    format!("- {}", key)
                }
            })
            .collect::<Vec<String>>()
            .join("\n")
    }

    #[tracing::instrument(skip(self))]
    pub fn display_options_inline(&self) -> String {
        self.options
            .par_iter()
            .map(|(key, _)| key.to_string())
            .collect::<Vec<String>>()
            .join(", ")
    }
}

impl RuleNodeValueFloatComboBox {
    #[tracing::instrument(skip(key_value_pairs))]
    fn build_options(key_value_pairs: Vec<KeyValuePair>) -> HashMap<String, Option<String>> {
        let mut options: HashMap<String, Option<String>> = HashMap::new();
        for entry in key_value_pairs {
            let description = if let Some(value) = parse_as_string(&entry) {
                Some(value)
            } else {
                None
            };
            options.insert(entry.key.clone(), description);
        }

        options
    }

    #[tracing::instrument(skip(entries, initial_options))]
    fn new(entries: &Vec<KeyValuePair>, initial_options: Option<Vec<KeyValuePair>>) -> Self {
        let mut default: Option<f64> = None;
        let mut source: Option<String> = None;
        let options: HashMap<String, Option<String>> =
            if let Some(initial_options) = initial_options {
                RuleNodeValueFloatComboBox::build_options(initial_options)
            } else {
                HashMap::new()
            };

        for entry in entries {
            if entry.key.eq_ignore_ascii_case("source") {
                source = parse_as_string(entry);
            }
            if entry.key.eq_ignore_ascii_case("default") {
                default = parse_as_numeric(entry);
            }
        }

        Self {
            default,
            source,
            options,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuleNodeValueListBox {
    default: Option<String>,
    source: Option<String>,
    options: HashMap<String, Option<String>>,
}

impl RuleNodeValueListBox {
    #[tracing::instrument(skip(self, validation_path))]
    pub(crate) async fn update_sources(&mut self, validation_path: &Path) -> Result<()> {
        if let Some(source) = &self.source {
            let source_path = validation_path.join(source).with_extension("txt");
            trace!("Updating source: {:?}", source_path);
            if source_path.exists() && source_path.is_file() {
                let source = parse_file(&source_path).await;

                if let Ok(source) = source {
                    let options = RuleNodeValueListBox::build_options(source.key_value_pairs);
                    self.options.par_extend(options.clone());
                    trace!("Added options: {:?}", options);
                } else if let Err(source) = source {
                    warn!("Failed to parse source file: {:?}", source);
                    trace!("Failed to parse source file: {:?}", source);
                }
            }
        }

        Ok(())
    }

    #[tracing::instrument(skip(self, key))]
    pub fn has_value(&self, key: &str) -> bool {
        self.options.contains_key(key)
    }

    #[tracing::instrument(skip(self))]
    pub fn display_options(&self) -> String {
        self.options
            .par_iter()
            .map(|(key, value)| {
                if let Some(value) = value {
                    format!("- {}: {}", key, value)
                } else {
                    format!("- {}", key)
                }
            })
            .collect::<Vec<String>>()
            .join("\n")
    }

    #[tracing::instrument(skip(self))]
    pub fn display_options_inline(&self) -> String {
        self.options
            .par_iter()
            .map(|(key, _)| key.to_string())
            .collect::<Vec<String>>()
            .join(", ")
    }
}

impl RuleNodeValueListBox {
    #[tracing::instrument(skip(key_value_pairs))]
    fn build_options(key_value_pairs: Vec<KeyValuePair>) -> HashMap<String, Option<String>> {
        let mut options: HashMap<String, Option<String>> = HashMap::new();
        for entry in key_value_pairs {
            let description = parse_as_string(&entry);
            options.insert(entry.key.clone(), description);
        }

        options
    }
    #[tracing::instrument(skip(entries))]
    fn new(entries: &Vec<KeyValuePair>, initial_options: Option<Vec<KeyValuePair>>) -> Self {
        let mut default: Option<String> = None;
        let mut source: Option<String> = None;
        let options: HashMap<String, Option<String>> =
            if let Some(initial_options) = initial_options {
                RuleNodeValueListBox::build_options(initial_options)
            } else {
                HashMap::new()
            };

        for entry in entries {
            if entry.key.eq_ignore_ascii_case("source") {
                source = parse_as_string(entry);
            }
            if entry.key.eq_ignore_ascii_case("default") {
                default = parse_as_string(entry);
            }
        }

        Self {
            default,
            source,
            options,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuleNodeValueKuid {}

impl RuleNodeValueKuid {
    #[tracing::instrument(skip())]
    fn new(_: &Vec<KeyValuePair>) -> Self {
        Self {}
    }
}

#[derive(Debug, Clone)]
pub struct RuleNodeValueKuidBrowser {
    default: Option<(i32, i32, Option<u8>)>,
    filter: Option<String>,
    categories: Option<Vec<String>>,
}

impl RuleNodeValueKuidBrowser {
    #[tracing::instrument(skip(entries))]
    fn new(entries: &Vec<KeyValuePair>) -> Self {
        let mut default: Option<(i32, i32, Option<u8>)> = None;
        let mut filter: Option<String> = None;
        let mut categories: Option<Vec<String>> = None;

        for entry in entries {
            if entry.key.eq_ignore_ascii_case("default")
                && let Some(Value::Kuid(kuid, _)) = &entry.value
            {
                default = Some((kuid.user_id, kuid.content_id, kuid.version))
            }
            if entry.key.eq_ignore_ascii_case("filter") {
                filter = parse_as_string(entry);
            }
            if entry.key.eq_ignore_ascii_case("default")
                && let Some(categories_packed) = parse_as_string(entry)
            {
                categories = Some(
                    categories_packed
                        .split(";")
                        .map(|val| val.to_string())
                        .collect::<Vec<String>>(),
                )
            }
        }

        Self {
            default,
            filter,
            categories,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuleNodeValueFilePath {
    default: Option<String>,
    data_type: Option<String>,
}

impl RuleNodeValueFilePath {
    #[tracing::instrument(skip(entries))]
    fn new(entries: &Vec<KeyValuePair>) -> Self {
        let mut default: Option<String> = None;
        let mut data_type: Option<String> = None;

        for entry in entries {
            if entry.key.eq_ignore_ascii_case("default") {
                default = parse_as_string(entry);
            }
            if entry.key.eq_ignore_ascii_case("datatype") {
                data_type = parse_as_string(entry);
            }
        }

        Self { default, data_type }
    }
}

#[derive(Debug, Clone)]
pub struct RuleNodeValueFloatList {
    default: Option<Vec<f64>>,
}

impl RuleNodeValueFloatList {
    #[tracing::instrument(skip(entries))]
    fn new(entries: &Vec<KeyValuePair>) -> Self {
        let mut default: Option<Vec<f64>> = None;

        for entry in entries {
            if entry.key.eq_ignore_ascii_case("default") {
                default = parse_as_array_option::<f64>(entry);
            }
        }

        Self { default }
    }
}

#[derive(Debug, Clone)]
pub struct RuleNodeValueVector2 {
    default: Option<(f64, f64)>,
}

impl RuleNodeValueVector2 {
    #[tracing::instrument(skip(entries))]
    fn new(entries: &Vec<KeyValuePair>) -> Self {
        let mut default: Option<(f64, f64)> = None;

        for entry in entries {
            if entry.key.eq_ignore_ascii_case("default") {
                if let Some(parsed) = parse_as_array_option::<f64>(entry)
                    && parsed.len() == 2
                {
                    default = Some((parsed[0], parsed[1]))
                } else {
                    warn!("Invalid default value for vector2: {:?}", entry.value);
                    trace!("Invalid default value for vector2: {:?}", entry.value);
                }
            }
        }

        Self { default }
    }
}

#[derive(Debug, Clone)]
pub struct RuleNodeValueVector3 {
    default: Option<(f64, f64, f64)>,
}

impl RuleNodeValueVector3 {
    #[tracing::instrument(skip(entries))]
    fn new(entries: &Vec<KeyValuePair>) -> Self {
        let mut default: Option<(f64, f64, f64)> = None;

        for entry in entries {
            if entry.key.eq_ignore_ascii_case("default") {
                if let Some(parsed) = parse_as_array_option::<f64>(entry)
                    && parsed.len() == 3
                {
                    default = Some((parsed[0], parsed[1], parsed[2]))
                } else {
                    warn!("Invalid default value for vector3: {:?}", entry.value);
                    trace!("Invalid default value for vector3: {:?}", entry.value);
                }
            }
        }

        Self { default }
    }
}

#[derive(Debug, Clone)]
pub struct RuleNodeValueVector4 {
    default: Option<(f64, f64, f64, f64)>,
}

impl RuleNodeValueVector4 {
    #[tracing::instrument(skip(entries))]
    fn new(entries: &Vec<KeyValuePair>) -> Self {
        let mut default: Option<(f64, f64, f64, f64)> = None;

        for entry in entries {
            if entry.key.eq_ignore_ascii_case("default") {
                if let Some(parsed) = parse_as_array_option::<f64>(entry)
                    && parsed.len() == 4
                {
                    default = Some((parsed[0], parsed[1], parsed[2], parsed[3]))
                } else {
                    warn!("Invalid default value for vector4: {:?}", entry.value);
                    trace!("Invalid default value for vector4: {:?}", entry.value);
                }
            }
        }

        Self { default }
    }
}

#[derive(Debug, Clone)]
pub struct RuleNodeValueVector5 {
    default: Option<(f64, f64, f64, f64, f64)>,
}

impl RuleNodeValueVector5 {
    #[tracing::instrument(skip(entries))]
    fn new(entries: &Vec<KeyValuePair>) -> Self {
        let mut default: Option<(f64, f64, f64, f64, f64)> = None;

        for entry in entries {
            if entry.key.eq_ignore_ascii_case("default") {
                if let Some(parsed) = parse_as_array_option::<f64>(entry)
                    && parsed.len() == 5
                {
                    default = Some((parsed[0], parsed[1], parsed[2], parsed[3], parsed[4]))
                } else {
                    warn!("Invalid default value for vector5: {:?}", entry.value);
                    trace!("Invalid default value for vector5: {:?}", entry.value);
                }
            }
        }

        Self { default }
    }
}

#[derive(Debug, Clone)]
pub struct RuleNodeValueVector6 {
    default: Option<(f64, f64, f64, f64, f64, f64)>,
}

impl RuleNodeValueVector6 {
    #[tracing::instrument(skip(entries))]
    fn new(entries: &Vec<KeyValuePair>) -> Self {
        let mut default: Option<(f64, f64, f64, f64, f64, f64)> = None;

        for entry in entries {
            if entry.key.eq_ignore_ascii_case("default") {
                if let Some(parsed) = parse_as_array_option::<f64>(entry)
                    && parsed.len() == 6
                {
                    default = Some((
                        parsed[0], parsed[1], parsed[2], parsed[3], parsed[4], parsed[5],
                    ))
                } else {
                    warn!("Invalid default value for vector6: {:?}", entry.value);
                    trace!("Invalid default value for vector6: {:?}", entry.value);
                }
            }
        }

        Self { default }
    }
}
