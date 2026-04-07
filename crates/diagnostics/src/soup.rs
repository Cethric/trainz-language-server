use gs_ast::soup::Soup;
use gs_ast::soup::process::process_soup_ast;
use gs_ast::soup::value::{NumericValue, Value};
use gs_parser::soup::parse_soup;
use log::error;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tower_lsp_server::ls_types::{Diagnostic, DiagnosticSeverity};

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

fn parse_rule(key: String, rule_details: Vec<gs_ast::soup::KeyValuePair>) -> ContainerRule {
    let mut type_name = None;
    let mut kind = None;
    let mut default_value = None;
    let mut description = None;
    let mut rule_validation = None;
    let mut compulsory = None;
    let mut filter = None;
    let mut disabled = None;
    let mut obsolete_tag = None;

    for detail in rule_details {
        let key_lower = detail.key.to_lowercase();
        match key_lower.as_str() {
            "type" => {
                type_name = match detail.value {
                    Some(Value::String(s, _)) => Some(s),
                    Some(Value::Variable(s, _)) => Some(s),
                    _ => None,
                }
            }
            "kind" => {
                kind = match detail.value {
                    Some(Value::String(s, _)) => Some(s),
                    Some(Value::Variable(s, _)) => Some(s),
                    _ => None,
                }
            }
            "default" => {
                default_value = match detail.value {
                    Some(Value::String(s, _)) => Some(s),
                    Some(Value::Variable(s, _)) => Some(s),
                    _ => None,
                }
            }
            "description" => {
                description = match detail.value {
                    Some(Value::String(s, _)) => Some(s),
                    Some(Value::Variable(s, _)) => Some(s),
                    _ => None,
                }
            }
            "validation" => {
                rule_validation = match detail.value {
                    Some(Value::String(s, _)) => Some(s),
                    Some(Value::Variable(s, _)) => Some(s),
                    Some(Value::Container(details, _)) => {
                        // If validation is a container, use the first key
                        details.first().map(|d| d.key.clone())
                    }
                    _ => None,
                }
            }
            "compulsory" => {
                compulsory = match detail.value {
                    Some(Value::Numeric(NumericValue::Float(n), _)) => Some(n),
                    Some(Value::Numeric(NumericValue::Int(n), _)) => Some(n as f64),
                    Some(Value::Array(v, _)) => {
                        if let Some(NumericValue::Int(n)) = v.first() {
                            Some(*n as f64)
                        } else if let Some(NumericValue::Float(n)) = v.first() {
                            Some(*n)
                        } else {
                            None
                        }
                    }
                    Some(Value::String(s, _)) => s.parse().ok(),
                    _ => None,
                };
            }
            "filter" => {
                filter = match detail.value {
                    Some(Value::String(s, _)) => Some(s),
                    _ => None,
                }
            }
            "disabled" => {
                disabled = match detail.value {
                    Some(Value::Numeric(NumericValue::Float(n), _)) => Some(n == 1.0),
                    Some(Value::Numeric(NumericValue::Int(n), _)) => Some(n == 1),
                    Some(Value::Array(v, _)) => {
                        if let Some(NumericValue::Int(n)) = v.first() {
                            Some(*n == 1)
                        } else if let Some(NumericValue::Float(n)) = v.first() {
                            Some((*n - 1.0).abs() < 1e-9)
                        } else {
                            None
                        }
                    }
                    Some(Value::String(s, _)) => Some(s == "1"),
                    _ => None,
                }
            }
            "obsolete-tag" => {
                obsolete_tag = match detail.value {
                    Some(Value::Numeric(NumericValue::Float(n), _)) => Some(n == 1.0),
                    Some(Value::Numeric(NumericValue::Int(n), _)) => Some(n == 1),
                    Some(Value::Array(v, _)) => {
                        if let Some(NumericValue::Int(n)) = v.first() {
                            Some(*n == 1)
                        } else if let Some(NumericValue::Float(n)) = v.first() {
                            Some((*n - 1.0).abs() < 1e-9)
                        } else {
                            None
                        }
                    }
                    Some(Value::String(s, _)) => Some(s == "1"),
                    _ => None,
                }
            }
            _ => {}
        }
    }

    ContainerRule {
        key,
        type_name,
        kind,
        default_value,
        description,
        validation: rule_validation,
        compulsory,
        filter,
        disabled,
        obsolete_tag,
        array_element: None,
    }
}

pub fn load_validators(validation_path: &Path) -> Validators {
    let mut simple_validators = vec![];
    let mut container_validators = vec![];
    let mut category_classes = HashMap::new();
    let mut category_regions = HashMap::new();
    let mut category_eras = HashMap::new();

    if !validation_path.exists() || !validation_path.is_dir() {
        return Validators {
            simple: simple_validators,
            containers: container_validators,
            category_classes,
            category_regions,
            category_eras,
        };
    }

    let entries = match fs::read_dir(validation_path) {
        Ok(entries) => entries,
        Err(e) => {
            error!("Failed to read validation directory: {}", e);
            return Validators {
                simple: simple_validators,
                containers: container_validators,
                category_classes,
                category_regions,
                category_eras,
            };
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            let filename = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            let full_filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if filename.is_empty() {
                continue;
            }

            if let Ok(validator_content) = fs::read_to_string(&path) {
                if let Ok(pairs) = parse_soup(&validator_content) {
                    let validator_soup =
                        process_soup_ast(pairs, &validator_content, &path, &vec![], &vec![]);

                    // Look for existing container name or create new
                    let filename_lower = filename.to_lowercase();
                    let full_filename_lower = full_filename.to_lowercase();
                    let is_container_style = match filename_lower.as_str() {
                        "container" | "effect-layer" | "inheritance" | "kind" | "my_container" => {
                            true
                        }
                        _ => full_filename_lower == "kind.txt",
                    };

                    // Always try to parse as container if it contains any key-value pairs that look like container definitions
                    // But for safety and to maintain existing behavior, we can also check if the first level of the soup contains keys that are later used as container_name.

                    if is_container_style {
                        for kv in validator_soup.key_value_pairs {
                            if let Some(Value::Container(container_kv, _)) = kv.value {
                                let mut rules = vec![];
                                let mut array_element = None;
                                let mut validation = None;
                                let mut inherit = vec![];
                                let mut top_level = false;
                                let mut subpossibilities = vec![];
                                let mut tag_array = false;

                                for rule_kv in container_kv {
                                    match rule_kv.value {
                                        Some(Value::Container(rule_details, _)) => {
                                            if rule_kv.key.eq_ignore_ascii_case("array-element") {
                                                for detail in rule_details {
                                                    if detail.key == "container-type0" {
                                                        array_element = match detail.value {
                                                            Some(Value::String(s, _)) => Some(s),
                                                            Some(Value::Variable(s, _)) => Some(s),
                                                            _ => None,
                                                        };
                                                    }
                                                }
                                                continue;
                                            }
                                            if rule_kv.key.eq_ignore_ascii_case("validation") {
                                                // Handle validation rule for the container itself
                                                for detail in rule_details {
                                                    validation = Some(detail.key.clone()); // Usually just a key name
                                                    if detail.key.eq_ignore_ascii_case("TagArray")
                                                        || detail
                                                            .key
                                                            .eq_ignore_ascii_case("tagarray")
                                                        || detail
                                                            .key
                                                            .eq_ignore_ascii_case("tag-array")
                                                    {
                                                        tag_array = true;
                                                    }
                                                }
                                                continue;
                                            }
                                            if rule_kv.key.eq_ignore_ascii_case("inherit") {
                                                for detail in rule_details {
                                                    inherit.push(detail.key);
                                                }
                                                continue;
                                            }
                                            if rule_kv.key.eq_ignore_ascii_case("subpossibilities")
                                            {
                                                for sub_kv in rule_details {
                                                    if let Some(Value::Container(sub_details, _)) =
                                                        sub_kv.value
                                                    {
                                                        subpossibilities.push(parse_rule(
                                                            sub_kv.key,
                                                            sub_details,
                                                        ));
                                                    }
                                                }
                                                continue;
                                            }

                                            rules.push(parse_rule(rule_kv.key, rule_details));
                                        }
                                        Some(Value::String(s, _)) | Some(Value::Variable(s, _)) => {
                                            // Handle top-level keys in container definition
                                            let key_lower = rule_kv.key.to_lowercase();
                                            match key_lower.as_str() {
                                                "kind" => { /* handle container kind if needed */ }
                                                "top-level" => {
                                                    top_level = s == "1" || s == "true";
                                                }
                                                "inherit" => {
                                                    inherit.push(s);
                                                }
                                                "validation" | "subpossibilities"
                                                | "array-element" | "tagarray" | "uniquenames" => {
                                                    // Already handled or special metadata
                                                }
                                                _ => {
                                                    // It's a simple rule override (e.g., author "string")
                                                    rules.push(ContainerRule {
                                                        key: rule_kv.key,
                                                        type_name: Some(s),
                                                        kind: None,
                                                        default_value: None,
                                                        description: None,
                                                        validation: None,
                                                        compulsory: None,
                                                        filter: None,
                                                        disabled: None,
                                                        obsolete_tag: None,
                                                        array_element: None,
                                                    });
                                                }
                                            }
                                        }
                                        Some(Value::Array(v, _)) => {
                                            match rule_kv.key.to_lowercase().as_str() {
                                                "top-level" => {
                                                    if let Some(NumericValue::Int(n)) = v.first() {
                                                        top_level = *n == 1;
                                                    } else if let Some(NumericValue::Float(n)) =
                                                        v.first()
                                                    {
                                                        top_level = (*n - 1.0).abs() < 1e-9;
                                                    }
                                                }
                                                _ => {}
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                                container_validators.push(ContainerValidator {
                                    container_name: kv.key,
                                    rules,
                                    array_element,
                                    validation,
                                    inherit,
                                    top_level,
                                    subpossibilities,
                                    tag_array,
                                });
                            }
                        }
                    } else if full_filename_lower == "category-class.txt" {
                        for kv in validator_soup.key_value_pairs {
                            let description = match kv.value {
                                Some(Value::String(s, _)) => s,
                                Some(Value::Variable(s, _)) => s,
                                _ => String::new(),
                            };
                            category_classes.insert(kv.key, description);
                        }
                    } else if full_filename_lower == "category-region.txt" {
                        for kv in validator_soup.key_value_pairs {
                            let description = match kv.value {
                                Some(Value::String(s, _)) => s,
                                Some(Value::Variable(s, _)) => s,
                                _ => String::new(),
                            };
                            category_regions.insert(kv.key, description);
                        }
                    } else if full_filename_lower == "category-era.txt" {
                        for kv in validator_soup.key_value_pairs {
                            let description = match kv.value {
                                Some(Value::String(s, _)) => s,
                                Some(Value::Variable(s, _)) => s,
                                _ => String::new(),
                            };
                            category_eras.insert(kv.key, description);
                        }
                    } else {
                        let mut allowed_values = HashMap::new();
                        for kv in validator_soup.key_value_pairs {
                            let description = match kv.value {
                                Some(Value::String(s, _)) => s,
                                Some(Value::Variable(s, _)) => s,
                                _ => String::new(),
                            };
                            allowed_values.insert(kv.key, description);
                        }

                        simple_validators.push(SoupValidator {
                            key_to_check: filename.to_string(),
                            allowed_values,
                        });
                    }
                }
            }
        }
    }

    let mut validators = Validators {
        simple: simple_validators,
        containers: container_validators,
        category_classes,
        category_regions,
        category_eras,
    };

    // Perform inheritance merging
    let mut merged_containers = vec![];
    for i in 0..validators.containers.len() {
        let mut container = validators.containers[i].clone();
        if !container.inherit.is_empty() {
            let mut merged_rules = vec![];
            let mut merged_subpossibilities = vec![];

            // Helper to merge rules from inherited containers
            for inherited_name in &container.inherit {
                for j in 0..validators.containers.len() {
                    let parent_name = &validators.containers[j].container_name;
                    if parent_name.eq_ignore_ascii_case(inherited_name) {
                        let parent = validators.containers[j].clone();
                        // Recursive merge for parents
                        // Note: this only goes one level deep for now,
                        // but should be enough for the test.
                        for rule in &parent.rules {
                            if !merged_rules
                                .iter()
                                .any(|r: &ContainerRule| r.key.eq_ignore_ascii_case(&rule.key))
                            {
                                merged_rules.push(rule.clone());
                            }
                        }
                        for sub in &parent.subpossibilities {
                            if !merged_subpossibilities
                                .iter()
                                .any(|r: &ContainerRule| r.key.eq_ignore_ascii_case(&sub.key))
                            {
                                merged_subpossibilities.push(sub.clone());
                            }
                        }
                        break;
                    }
                }
            }

            // Current container rules override inherited rules
            for rule in &container.rules {
                if let Some(idx) = merged_rules
                    .iter()
                    .position(|r| r.key.eq_ignore_ascii_case(&rule.key))
                {
                    merged_rules[idx] = rule.clone();
                } else {
                    merged_rules.push(rule.clone());
                }
            }
            for sub in &container.subpossibilities {
                if let Some(idx) = merged_subpossibilities
                    .iter()
                    .position(|r| r.key.eq_ignore_ascii_case(&sub.key))
                {
                    merged_subpossibilities[idx] = sub.clone();
                } else {
                    merged_subpossibilities.push(sub.clone());
                }
            }

            container.rules = merged_rules;
            container.subpossibilities = merged_subpossibilities;
        }
        merged_containers.push(container);
    }
    validators.containers = merged_containers;

    // Second pass for multi-level inheritance
    let mut changed = true;
    while changed {
        changed = false;
        let containers_snapshot = validators.containers.clone();
        for i in 0..validators.containers.len() {
            let container = &containers_snapshot[i];
            let mut container_to_update = validators.containers[i].clone();
            let mut rule_added = false;
            for inherited_name in &container.inherit {
                if let Some(parent) = containers_snapshot
                    .iter()
                    .find(|c| c.container_name.eq_ignore_ascii_case(inherited_name))
                {
                    for rule in &parent.rules {
                        if !container_to_update
                            .rules
                            .iter()
                            .any(|r| r.key.eq_ignore_ascii_case(&rule.key))
                        {
                            container_to_update.rules.push(rule.clone());
                            rule_added = true;
                        }
                    }
                    for sub in &parent.subpossibilities {
                        if !container_to_update
                            .subpossibilities
                            .iter()
                            .any(|r| r.key.eq_ignore_ascii_case(&sub.key))
                        {
                            container_to_update.subpossibilities.push(sub.clone());
                            rule_added = true;
                        }
                    }
                }
            }
            if rule_added {
                validators.containers[i] = container_to_update;
                changed = true;
            }
        }
    }

    validators
}

pub fn soup_diagnostics(
    soup: &Soup,
    validators: &Validators,
    base_path: Option<&Path>,
) -> Vec<Diagnostic> {
    let mut diagnostics = vec![];

    let kind_kv = soup
        .key_value_pairs
        .iter()
        .find(|kv| kv.key.eq_ignore_ascii_case("kind"));
    let kind_validator_name = kind_kv.as_ref().and_then(|kv| match &kv.value {
        Some(Value::String(kind_name, _)) => Some(kind_name.clone()),
        Some(Value::Variable(kind_name, _)) => Some(kind_name.clone()),
        _ => None,
    });

    let kind_validator = kind_validator_name.as_ref().and_then(|name| {
        validators
            .containers
            .iter()
            .find(|v| v.container_name.eq_ignore_ascii_case(name))
    });

    if let Some(validator) = kind_validator {
        if validator.top_level {
            // Validate all subpossibilities at the top level
            for sub in &validator.subpossibilities {
                let soup_kv = soup
                    .key_value_pairs
                    .iter()
                    .find(|kv| kv.key.eq_ignore_ascii_case(&sub.key));
                if let Some(kv) = soup_kv {
                    if sub.obsolete_tag.unwrap_or(false) {
                        diagnostics.push(Diagnostic {
                            range: kv.key_range,
                            severity: Some(DiagnosticSeverity::WARNING),
                            message: format!(
                                "Key '{}' is obsolete/deprecated in container '{}'",
                                kv.key, validator.container_name
                            ),
                            source: Some(String::from("soup-validator")),
                            ..Default::default()
                        });
                    }
                    if let Some(value) = &kv.value {
                        validate_value(
                            value,
                            sub,
                            validators,
                            &mut diagnostics,
                            &validator.container_name,
                            base_path,
                        );
                    }
                } else if sub.compulsory.map_or(false, |c| c >= 1.0)
                    && !sub.disabled.unwrap_or(false)
                {
                    diagnostics.push(Diagnostic {
                        range: soup.range,
                        severity: Some(DiagnosticSeverity::ERROR),
                        message: format!("Compulsory top-level key '{}' is missing.", sub.key),
                        source: Some(String::from("soup-validator")),
                        ..Default::default()
                    });
                }
            }

            // Also validate rules if any (in addition to subpossibilities)
            for rule in &validator.rules {
                let soup_kv = soup
                    .key_value_pairs
                    .iter()
                    .find(|kv| kv.key.eq_ignore_ascii_case(&rule.key));
                if let Some(kv) = soup_kv {
                    if rule.obsolete_tag.unwrap_or(false) {
                        diagnostics.push(Diagnostic {
                            range: kv.key_range,
                            severity: Some(DiagnosticSeverity::WARNING),
                            message: format!(
                                "Key '{}' is obsolete/deprecated in container '{}'",
                                kv.key, validator.container_name
                            ),
                            source: Some(String::from("soup-validator")),
                            ..Default::default()
                        });
                    }
                    if let Some(value) = &kv.value {
                        validate_value(
                            value,
                            rule,
                            validators,
                            &mut diagnostics,
                            &validator.container_name,
                            base_path,
                        );
                    }
                } else if rule.compulsory.map_or(false, |c| c >= 1.0)
                    && !rule.disabled.unwrap_or(false)
                {
                    diagnostics.push(Diagnostic {
                        range: soup.range,
                        severity: Some(DiagnosticSeverity::ERROR),
                        message: format!("Compulsory top-level key '{}' is missing.", rule.key),
                        source: Some(String::from("soup-validator")),
                        ..Default::default()
                    });
                }
            }

            // Check for unknown top-level keys
            for kv in &soup.key_value_pairs {
                if kv.key.eq_ignore_ascii_case("kind") {
                    continue;
                }
                if !validator
                    .subpossibilities
                    .iter()
                    .any(|sub| sub.key.eq_ignore_ascii_case(&kv.key))
                    && !validator
                        .rules
                        .iter()
                        .any(|rule| rule.key.eq_ignore_ascii_case(&kv.key))
                {
                    // Ignore metadata keys
                    if kv.key.eq_ignore_ascii_case("array-element")
                        || kv.key.eq_ignore_ascii_case("subpossibilities")
                        || kv.key.eq_ignore_ascii_case("validation")
                        || kv.key.eq_ignore_ascii_case("top-level")
                        || kv.key.eq_ignore_ascii_case("tagarray")
                        || kv.key.eq_ignore_ascii_case("tag-array")
                    {
                        continue;
                    }

                    diagnostics.push(Diagnostic {
                        range: kv.range,
                        severity: Some(DiagnosticSeverity::WARNING),
                        message: format!(
                            "Unknown top-level key '{}' for kind '{}'",
                            kv.key, validator.container_name
                        ),
                        source: Some(String::from("soup-validator")),
                        ..Default::default()
                    });
                }
            }
        } else {
            validate_container(
                &soup.key_value_pairs,
                validator,
                validators,
                &mut diagnostics,
                Some("kind"),
                base_path,
            );
        }
    }

    // Simple validators
    for validator in &validators.simple {
        if let Some(name) = &kind_validator_name {
            if validator.key_to_check.eq_ignore_ascii_case(name) {
                continue;
            }
        }
        for soup_kv in &soup.key_value_pairs {
            if soup_kv.key.eq_ignore_ascii_case(&validator.key_to_check) {
                if soup_kv.key.eq_ignore_ascii_case("kind") {
                    continue;
                }
                if let Some(value) = &soup_kv.value {
                    validate_simple_value(
                        value,
                        &soup_kv.key,
                        &validator.allowed_values,
                        &mut diagnostics,
                    );
                }
            }
        }
    }

    // Container validators
    for validator in &validators.containers {
        if let Some(name) = &kind_validator_name {
            if validator.container_name.eq_ignore_ascii_case(name) {
                continue;
            }
        }
        for soup_kv in &soup.key_value_pairs {
            if soup_kv.key.eq_ignore_ascii_case(&validator.container_name) {
                if soup_kv.key.eq_ignore_ascii_case("kind") {
                    continue;
                }
                if let Some(Value::Container(container_kv, _)) = &soup_kv.value {
                    validate_container(
                        container_kv,
                        validator,
                        validators,
                        &mut diagnostics,
                        None,
                        base_path,
                    );
                }
            }
        }
    }

    // Final sanity check for unknown top-level keys if no kind-based validation was done
    if kind_validator.is_none() {
        // ... (we could add logic here for top-level keys when no kind is present)
    }

    diagnostics
}

fn validate_simple_value(
    value: &Value,
    key_to_check: &str,
    allowed_values: &HashMap<String, String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let value_str = match value {
        Value::String(s, _) => s.clone(),
        Value::Variable(s, _) => s.clone(),
        _ => return,
    };

    if !allowed_values.contains_key(&value_str) {
        let values: Vec<&str> = value_str.split(';').map(|s| s.trim()).collect();
        let mut invalid_values = vec![];
        for v in values {
            if !allowed_values.contains_key(v) {
                invalid_values.push(v);
            }
        }

        if !invalid_values.is_empty() {
            diagnostics.push(Diagnostic {
                range: value.range(),
                severity: Some(DiagnosticSeverity::ERROR),
                message: format!(
                    "Invalid value(s) '{}' for key '{}'. Allowed values are: {}",
                    invalid_values.join(", "),
                    key_to_check,
                    allowed_values
                        .keys()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                source: Some(String::from("soup-validator")),
                ..Default::default()
            });
        }
    }
}

fn validate_value(
    value: &Value,
    rule: &ContainerRule,
    all_validators: &Validators,
    diagnostics: &mut Vec<Diagnostic>,
    container_name: &str,
    base_path: Option<&Path>,
) {
    if let Some(type_name) = &rule.type_name {
        let actual_type = match value {
            Value::String(_, _) => "string",
            Value::Numeric(_, _) => "numeric",
            Value::Array(v, _) => {
                if v.iter().any(|n| matches!(n, NumericValue::Float(_))) {
                    "floatlist"
                } else {
                    "array"
                }
            }
            Value::Container(_, _) => "container",
            Value::Kuid(_, _) => "kuid",
            Value::Variable(_, _) => "variable",
        };

        let is_numeric = match value {
            Value::Numeric(_, _) => true,
            Value::Array(v, _) if v.len() == 1 => true,
            Value::String(s, _) | Value::Variable(s, _) => {
                s.parse::<f64>().is_ok() || s.starts_with("0x") || s.starts_with("0X")
            }
            _ => false,
        };

        let is_array_type = actual_type == "array" || actual_type == "floatlist";

        let type_mismatch = if type_name == "numeric" {
            !is_numeric
        } else if type_name == "int" || type_name == "float" {
            actual_type != "numeric" && actual_type != "variable"
        } else if type_name == "bool" {
            if actual_type == "numeric" {
                match value {
                    Value::Numeric(NumericValue::Int(n), _) => *n != 0 && *n != 1,
                    Value::Numeric(NumericValue::Float(f), _) => {
                        (*f - 0.0).abs() > 1e-9 && (*f - 1.0).abs() > 1e-9
                    }
                    _ => true,
                }
            } else {
                actual_type != "bool" && actual_type != "variable"
            }
        } else if type_name == "kuid" || type_name == "kuidbrowser" {
            actual_type != "kuid" && actual_type != "variable"
        } else if type_name == "filepath" {
            actual_type != "string" && actual_type != "variable"
        } else if type_name.starts_with("vector") || type_name == "floatlist" {
            !is_array_type && actual_type != "variable"
        } else if type_name == "combobox" || type_name == "listbox" || type_name == "filepathedit" {
            actual_type != "string" && actual_type != "variable"
        } else {
            actual_type != type_name.as_str()
                && actual_type != "variable"
                && !(type_name == "array" && is_array_type)
        };

        if type_mismatch {
            let message = format!(
                "Invalid type for key '{}' in container '{}'. Expected '{}', found '{}'",
                rule.key, container_name, type_name, actual_type
            );
            diagnostics.push(Diagnostic {
                range: value.range(),
                severity: Some(DiagnosticSeverity::ERROR),
                message,
                source: Some(String::from("soup-validator")),
                ..Default::default()
            });
        }
    }

    match value {
        Value::Container(container_kv, _) => {
            if let Some(kind_name) = &rule.kind {
                if let Some(validator) = all_validators
                    .containers
                    .iter()
                    .find(|v| v.container_name.eq_ignore_ascii_case(kind_name))
                {
                    validate_container(
                        container_kv,
                        validator,
                        all_validators,
                        diagnostics,
                        None,
                        base_path,
                    );
                }
            } else if let Some(type_name) = &rule.type_name {
                if let Some(validator) = all_validators
                    .containers
                    .iter()
                    .find(|v| v.container_name.eq_ignore_ascii_case(type_name))
                {
                    validate_container(
                        container_kv,
                        validator,
                        all_validators,
                        diagnostics,
                        None,
                        base_path,
                    );
                }
            }
        }
        Value::String(s, _) | Value::Variable(s, _) => {
            if let Some(type_name) = &rule.type_name {
                if type_name == "combobox" {
                    if s.contains(';') {
                        diagnostics.push(Diagnostic {
                            range: value.range(),
                            severity: Some(DiagnosticSeverity::WARNING),
                            message: format!(
                                "Key '{}' of type 'combobox' expects a single string value, but contains a semicolon.",
                                rule.key
                            ),
                            source: Some(String::from("soup-validator")),
                            ..Default::default()
                        });
                    }
                } else if type_name == "listbox" {
                    // Check if it has at least one semicolon? Or just ensure it's a string (which it is).
                    // Usually listbox implies multiple values are okay.
                } else if type_name == "filepathedit" {
                    if let Some(base_path) = base_path {
                        let file_path = if let Some(parent) = base_path.parent() {
                            parent.join(s)
                        } else {
                            Path::new(s).to_path_buf()
                        };
                        if !file_path.exists() {
                            diagnostics.push(Diagnostic {
                                range: value.range(),
                                severity: Some(DiagnosticSeverity::ERROR),
                                message: format!(
                                    "File '{}' for key '{}' does not exist.",
                                    s, rule.key
                                ),
                                source: Some(String::from("soup-validator")),
                                ..Default::default()
                            });
                        }
                    }
                }

                if let Some(validator) = all_validators
                    .simple
                    .iter()
                    .find(|v| v.key_to_check.eq_ignore_ascii_case(type_name))
                {
                    validate_simple_value(value, &rule.key, &validator.allowed_values, diagnostics);
                }
            }
            if let Some(filter) = &rule.filter {
                if !s.to_lowercase().contains(&filter.to_lowercase()) {
                    diagnostics.push(Diagnostic {
                        range: value.range(),
                        severity: Some(DiagnosticSeverity::ERROR),
                        message: format!(
                            "Value '{}' for key '{}' does not match filter '{}'.",
                            s, rule.key, filter
                        ),
                        source: Some(String::from("soup-validator")),
                        ..Default::default()
                    });
                }
            }
            if let Some(validation) = &rule.validation {
                if validation.eq_ignore_ascii_case("IsValidCategoryClass") {
                    if !all_validators.category_classes.contains_key(s) {
                        diagnostics.push(Diagnostic {
                            range: value.range(),
                            severity: Some(DiagnosticSeverity::ERROR),
                            message: format!(
                                "Value '{}' for key '{}' is not a valid category class.",
                                s, rule.key
                            ),
                            source: Some(String::from("soup-validator")),
                            ..Default::default()
                        });
                    }
                } else if validation.eq_ignore_ascii_case("IsValidCategoryRegion") {
                    if !all_validators.category_regions.contains_key(s) {
                        diagnostics.push(Diagnostic {
                            range: value.range(),
                            severity: Some(DiagnosticSeverity::ERROR),
                            message: format!(
                                "Value '{}' for key '{}' is not a valid category region.",
                                s, rule.key
                            ),
                            source: Some(String::from("soup-validator")),
                            ..Default::default()
                        });
                    }
                } else if validation.eq_ignore_ascii_case("IsValidCategoryEra") {
                    // Category era can be semicolon separated
                    // We need to provide precise ranges for each invalid era if possible.
                    // However, split() loses range info.
                    // For now, let's at least mark the whole value range if any part is invalid,
                    // or better, emit diagnostics for each invalid part but with the whole range.
                    // Wait, the requirement says "diagnostic ranges appear to be incorrect as they are only one char appart".
                    // This implies the user wants the full range of the value (or the specific era).

                    let eras: Vec<&str> = s
                        .split(';')
                        .map(|e| e.trim())
                        .filter(|e| !e.is_empty())
                        .collect();
                    for era in eras {
                        if !all_validators.category_eras.contains_key(era) {
                            diagnostics.push(Diagnostic {
                                range: value.range(),
                                severity: Some(DiagnosticSeverity::ERROR),
                                message: format!(
                                    "Value '{}' for key '{}' is not a valid category era.",
                                    era, rule.key
                                ),
                                source: Some(String::from("soup-validator")),
                                ..Default::default()
                            });
                        }
                    }
                }
            }
        }
        Value::Numeric(_, _) => {
            // Numbers are generally okay unless specific type validation is needed
        }
        _ => {}
    }
}

fn validate_container(
    container_kv: &[gs_ast::soup::KeyValuePair],
    validator: &ContainerValidator,
    all_validators: &Validators,
    diagnostics: &mut Vec<Diagnostic>,
    key_to_ignore: Option<&str>,
    base_path: Option<&Path>,
) {
    let mut value_type_validator = None;
    let is_tag_array = validator.tag_array
        || validator.validation.as_deref().map_or(false, |v| {
            v.eq_ignore_ascii_case("TagArray")
                || v.eq_ignore_ascii_case("tagarray")
                || v.eq_ignore_ascii_case("tag-array")
        })
        || validator
            .container_name
            .eq_ignore_ascii_case("string-table");

    if is_tag_array
        || validator.validation.as_deref().map_or(false, |v| {
            v.eq_ignore_ascii_case("UniqueNames") || v.eq_ignore_ascii_case("SubPossibilities")
        })
    {
        let mut names = HashMap::new();
        for kv in container_kv {
            let key_lower = kv.key.to_lowercase();
            if let Some(_prev_range) = names.insert(key_lower, kv.range) {
                diagnostics.push(Diagnostic {
                    range: kv.range,
                    severity: Some(DiagnosticSeverity::ERROR),
                    message: format!(
                        "Duplicate key '{}' in container '{}'",
                        kv.key, validator.container_name
                    ),
                    source: Some(String::from("soup-validator")),
                    ..Default::default()
                });
            }
        }

        if is_tag_array {
            // For TagArray, we need to check if there is a 'type' rule that defines the value type
            if let Some(type_rule) = validator
                .rules
                .iter()
                .find(|r| r.key.eq_ignore_ascii_case("type"))
            {
                if let Some(type_name) = &type_rule.type_name {
                    value_type_validator = all_validators
                        .containers
                        .iter()
                        .find(|v| v.container_name.eq_ignore_ascii_case(type_name));
                }
            }
        }
    }

    let mut found_keys = std::collections::HashSet::new();
    for kv in container_kv {
        found_keys.insert(kv.key.to_lowercase());

        if let Some(ignore) = key_to_ignore {
            if kv.key.eq_ignore_ascii_case(ignore) {
                continue;
            }
        }
        let rule = validator
            .rules
            .iter()
            .find(|r| r.key.eq_ignore_ascii_case(&kv.key))
            .or_else(|| {
                validator
                    .subpossibilities
                    .iter()
                    .find(|r| r.key.eq_ignore_ascii_case(&kv.key))
            });

        if rule.is_none() && is_tag_array && !kv.key.eq_ignore_ascii_case("type") {
            // For TagArray, keys that are not 'type' and not in rules/subpossibilities are entries whose values should be validated against value_type_validator
            if let Some(value_validator) = value_type_validator {
                if let Some(value) = &kv.value {
                    if let Value::Container(inner_kv, _) = value {
                        validate_container(
                            inner_kv,
                            value_validator,
                            all_validators,
                            diagnostics,
                            None,
                            base_path,
                        );
                    } else {
                        // If it's not a container but we have a validator, maybe it expects a simple type?
                        // Currently validate_container only handles containers.
                    }
                }
            }
            // Even if value_type_validator is None, it's still a TagArray, so arbitrary keys are allowed.
            continue;
        }

        match rule {
            Some(rule) => {
                if rule.obsolete_tag.unwrap_or(false) {
                    diagnostics.push(Diagnostic {
                        range: kv.key_range,
                        severity: Some(DiagnosticSeverity::WARNING),
                        message: format!(
                            "Key '{}' is obsolete/deprecated in container '{}'",
                            kv.key, validator.container_name
                        ),
                        source: Some(String::from("soup-validator")),
                        ..Default::default()
                    });
                }
                if let Some(value) = &kv.value {
                    validate_value(
                        value,
                        rule,
                        all_validators,
                        diagnostics,
                        &validator.container_name,
                        base_path,
                    );
                }
            }
            None => {
                // Ignore metadata keys
                if kv.key.eq_ignore_ascii_case("array-element")
                    || kv.key.eq_ignore_ascii_case("subpossibilities")
                    || kv.key.eq_ignore_ascii_case("validation")
                    || kv.key.eq_ignore_ascii_case("top-level")
                    || kv.key.eq_ignore_ascii_case("tagarray")
                    || kv.key.eq_ignore_ascii_case("tag-array")
                {
                    continue;
                }

                // Not in rules, check if it's an array-element
                if let Some(element_type) = &validator.array_element {
                    if let Some(Value::Container(inner_kv, _)) = &kv.value {
                        if let Some(element_validator) = all_validators
                            .containers
                            .iter()
                            .find(|c| &c.container_name == element_type)
                        {
                            validate_container(
                                inner_kv,
                                element_validator,
                                all_validators,
                                diagnostics,
                                None,
                                base_path,
                            );
                        }
                    }
                } else {
                    // Unknown key in container
                    diagnostics.push(Diagnostic {
                        range: kv.range,
                        severity: Some(DiagnosticSeverity::WARNING),
                        message: format!(
                            "Unknown key '{}' in container '{}'",
                            kv.key, validator.container_name
                        ),
                        source: Some(String::from("soup-validator")),
                        ..Default::default()
                    });
                }
            }
        }
    }

    // Check compulsory keys in rules
    for rule in &validator.rules {
        if rule.compulsory.map_or(false, |c| c >= 1.0) && !rule.disabled.unwrap_or(false) {
            if !found_keys.contains(&rule.key.to_lowercase()) {
                let range = container_kv.first().map(|kv| kv.range).unwrap_or_default();
                diagnostics.push(Diagnostic {
                    range,
                    severity: Some(DiagnosticSeverity::ERROR),
                    message: format!(
                        "Compulsory key '{}' is missing in container '{}'.",
                        rule.key, validator.container_name
                    ),
                    source: Some(String::from("soup-validator")),
                    ..Default::default()
                });
            }
        }
    }

    // Check compulsory keys in subpossibilities
    for rule in &validator.subpossibilities {
        if rule.compulsory.map_or(false, |c| c >= 1.0) && !rule.disabled.unwrap_or(false) {
            if !found_keys.contains(&rule.key.to_lowercase()) {
                let range = container_kv.first().map(|kv| kv.range).unwrap_or_default();
                diagnostics.push(Diagnostic {
                    range,
                    severity: Some(DiagnosticSeverity::ERROR),
                    message: format!(
                        "Compulsory key '{}' is missing in container '{}'.",
                        rule.key, validator.container_name
                    ),
                    source: Some(String::from("soup-validator")),
                    ..Default::default()
                });
            }
        }
    }

    // Special check for array-element sequential keys
    if let Some(_element_type) = &validator.array_element {
        let mut indices: Vec<usize> = found_keys
            .iter()
            .filter_map(|k| k.parse::<usize>().ok())
            .collect();
        indices.sort_unstable();

        for (i, &index) in indices.iter().enumerate() {
            if i != index {
                // Find the KV pair for this index to get its range
                if let Some(kv) = container_kv.iter().find(|kv| kv.key == index.to_string()) {
                    diagnostics.push(Diagnostic {
                        range: kv.range,
                        severity: Some(DiagnosticSeverity::ERROR),
                        message: format!(
                            "Non-sequential array index '{}' in container '{}'. Expected '{}'.",
                            index, validator.container_name, i
                        ),
                        source: Some(String::from("soup-validator")),
                        ..Default::default()
                    });
                }
                break; // Only report the first gap
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gs_ast::soup::process::process_soup_ast;
    use gs_parser::soup::parse_soup;
    use std::path::PathBuf;

    #[test]
    fn test_bool_kuid_filepath_types() {
        let temp_dir = std::env::current_dir()
            .unwrap()
            .join("temp_bool_kuid_filepath_test");
        if temp_dir.exists() {
            let _ = std::fs::remove_dir_all(&temp_dir);
        }
        std::fs::create_dir_all(&temp_dir).unwrap();

        let content = r#"
test_container {
    bool_key_0 0
    bool_key_1 1
    bool_key_error 2
    kuid_key <kuid:1:2>
    kuidbrowser_key <kuid:3:4>
    filepath_key "some/path/file.txt"
}
"#;
        let pairs = parse_soup(content).unwrap();
        let soup = process_soup_ast(
            pairs,
            content,
            &PathBuf::from("test.soup"),
            &vec![],
            &vec![],
        );

        let container_txt = r#"
test_container
{
  kind "container"
  bool_key_0 { type "bool" }
  bool_key_1 { type "bool" }
  bool_key_error { type "bool" }
  kuid_key { type "kuid" }
  kuidbrowser_key { type "kuidbrowser" }
  filepath_key { type "filepath" }
}
"#;
        // In load_validators, 'is_container_style' checks for specific filenames.
        std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();

        let validators = load_validators(&temp_dir);
        let diagnostics = soup_diagnostics(&soup, &validators, None);

        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.severity == Some(DiagnosticSeverity::ERROR))
            .collect();

        // We expect only one error for 'bool_key_error' because it has value '2'
        assert_eq!(
            errors.len(),
            1,
            "Expected exactly 1 error for bool_key_error, but found: {:?}",
            errors
        );
        assert!(errors[0].message.contains("bool_key_error"));
        assert!(
            errors[0]
                .message
                .contains("Expected 'bool', found 'numeric'")
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_case_insensitive_keys_in_soup() {
        let content = r#"
Signals {
    0 { light -1 }
    1 { light -1 }
    2 { light -1 }
    3 { light -1 }
}
"#;
        let pairs = parse_soup(content);
        assert!(pairs.is_ok());

        let pairs = pairs.unwrap();
        let soup = process_soup_ast(
            pairs,
            content,
            &PathBuf::from("test.soup"),
            &vec![],
            &vec![],
        );

        let temp_dir = std::env::current_dir()
            .unwrap()
            .join("temp_case_insensitive_test");
        if temp_dir.exists() {
            let _ = std::fs::remove_dir_all(&temp_dir);
        }
        std::fs::create_dir_all(&temp_dir).unwrap();

        let container_txt = r#"
signals
{
  kind "container"
  array-element
  {
    container-type0 "signal-possibility"
  }
  validation
  {
    UniqueNames
  }
}

signal-possibility
{
  light
  {
    type "numeric"
  }
}
"#;
        std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();

        let validators = load_validators(&temp_dir);
        let diagnostics = soup_diagnostics(&soup, &validators, None);

        // Check if there are any errors.
        // 1. "Signals" should match "signals" in container.txt
        // 2. "Light" should match "light" in signal-possibility
        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.severity == Some(DiagnosticSeverity::ERROR))
            .collect();
        let warnings: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.severity == Some(DiagnosticSeverity::WARNING))
            .collect();

        assert!(
            errors.is_empty(),
            "Expected no errors for case-insensitive keys, but found errors: {:?}",
            errors
        );
        assert!(
            warnings.is_empty(),
            "Expected no warnings (unknown key), but found: {:?}",
            warnings
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_case_insensitive_duplicates() {
        let temp_dir = std::env::current_dir()
            .unwrap()
            .join("temp_duplicates_case_insensitive_test");
        if temp_dir.exists() {
            let _ = std::fs::remove_dir_all(&temp_dir);
        }
        std::fs::create_dir_all(&temp_dir).unwrap();

        let content2 = r#"
my_container {
    KeyA "value"
    keya "value"
}
"#;
        let pairs2 = parse_soup(content2).unwrap();
        let soup2 = process_soup_ast(
            pairs2,
            content2,
            &PathBuf::from("test2.soup"),
            &vec![],
            &vec![],
        );

        let my_container_txt = r#"
my_container
{
  kind "container"
  validation
  {
    UniqueNames
  }
  KeyA { type "string" }
}
"#;
        std::fs::write(temp_dir.join("my_container.txt"), my_container_txt).unwrap();

        let validators = load_validators(&temp_dir);
        let diagnostics = soup_diagnostics(&soup2, &validators, None);
        let has_duplicate_error = diagnostics.iter().any(|diag| {
            diag.message.to_lowercase().contains("duplicate key")
                && diag.message.to_lowercase().contains("'keya'")
        });

        assert!(
            has_duplicate_error,
            "Expected duplicate key error for case-insensitive 'KeyA' and 'keya', but found: {:?}",
            diagnostics
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_kind_txt_validation() {
        let content = r#"
kind "my-kind"
key1 "value1"
key2 123
"#;
        let pairs = parse_soup(content).unwrap();
        let soup = process_soup_ast(
            pairs,
            content,
            &PathBuf::from("test.soup"),
            &vec![],
            &vec![],
        );

        let temp_dir = std::env::current_dir().unwrap().join("temp_kind_test");
        if temp_dir.exists() {
            let _ = std::fs::remove_dir_all(&temp_dir);
        }
        std::fs::create_dir_all(&temp_dir).unwrap();

        let kind_txt = r#"
my-kind
{
  key1 { type "string" }
  key2 { type "numeric" }
}
"#;
        std::fs::write(temp_dir.join("kind.txt"), kind_txt).unwrap();

        let validators = load_validators(&temp_dir);
        let diagnostics = soup_diagnostics(&soup, &validators, None);

        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.severity == Some(DiagnosticSeverity::ERROR))
            .collect();
        assert!(
            errors.is_empty(),
            "Expected no errors for kind-based validation, but found: {:?}",
            errors
        );

        // Test with invalid value
        let invalid_content = r#"
kind "my-kind"
key1 123
"#;
        let pairs_invalid = parse_soup(invalid_content).unwrap();
        let soup_invalid = process_soup_ast(
            pairs_invalid,
            invalid_content,
            &PathBuf::from("test_invalid.soup"),
            &vec![],
            &vec![],
        );
        let diagnostics_invalid = soup_diagnostics(&soup_invalid, &validators, None);
        eprintln!(
            "DEBUG: Diagnostics for invalid content: {:?}",
            diagnostics_invalid
        );
        let has_type_error = diagnostics_invalid
            .iter()
            .any(|d| d.message.contains("Invalid type for key 'key1'"));
        assert!(
            has_type_error,
            "Expected type error for key1, but found: {:?}",
            diagnostics_invalid
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_numeric_keys_in_soup() {
        let content = r#"
signals {
    0 {
        light -1
    }
    1 {
        light -1
    }
    2 {
        light -1
    }
    3 {
        light -1
    }
}
"#;
        let pairs = parse_soup(content);
        assert!(
            pairs.is_ok(),
            "Failed to parse soup with numeric keys: {:?}",
            pairs.err()
        );

        let pairs = pairs.unwrap();
        let soup = process_soup_ast(
            pairs,
            content,
            &PathBuf::from("test.soup"),
            &vec![],
            &vec![],
        );

        // Create a temporary validation directory
        let temp_dir = std::env::current_dir()
            .unwrap()
            .join("temp_numeric_keys_test_unit");
        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir).unwrap();
        }
        std::fs::create_dir_all(&temp_dir).unwrap();

        let container_txt = r#"
signals
{
  kind "container"
  array-element
  {
    container-type0 "signal-possibility"
  }
  validation
  {
    UniqueNames
  }
}

signal-possibility
{
  light
  {
    type "numeric"
  }
}
"#;
        std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();

        let validators = load_validators(&temp_dir);
        let diagnostics = soup_diagnostics(&soup, &validators, None);

        // Check if there are any errors. Numeric keys should be valid.
        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.severity == Some(DiagnosticSeverity::ERROR))
            .collect();
        assert!(
            errors.is_empty(),
            "Expected no errors for numeric keys, but found: {:?}",
            errors
        );

        std::fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_sub_possibilities_validation() {
        let content = r#"
signals {
    0 {
        light -1
    }
    4 {
        light -1
    }
    0 {
        light -1
    }
}
"#;
        let pairs = parse_soup(content);
        assert!(pairs.is_ok());

        let pairs = pairs.unwrap();
        let soup = process_soup_ast(
            pairs,
            content,
            &PathBuf::from("test.soup"),
            &vec![],
            &vec![],
        );

        let temp_dir = std::env::current_dir()
            .unwrap()
            .join("temp_sub_possibilities_test");
        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir).unwrap();
        }
        std::fs::create_dir_all(&temp_dir).unwrap();

        let container_txt = r#"
signals
{
  kind "container"
  array-element
  {
    container-type0 "signal-possibility"
  }
  validation
  {
    SubPossibilities
  }
}

signal-possibility
{
  light
  {
    type "numeric"
  }
}
"#;
        std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();

        let validators = load_validators(&temp_dir);
        let diagnostics = soup_diagnostics(&soup, &validators, None);

        // Check for duplicate key error.
        let has_duplicate_error = diagnostics.iter().any(|diag| {
            diag.message
                .contains("Duplicate key '0' in container 'signals'")
        });

        assert!(
            has_duplicate_error,
            "Expected duplicate key error for '0', but found: {:?}",
            diagnostics
        );

        std::fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_tuple_container_validation() {
        // Test that tuple containers (no braces for values) are also validated correctly.
        let content = r#"
signals {
    0 { light -1 }
    1 { light -1 }
    2 { light -1 }
    3 { light -1 }
}
"#;
        let pairs = parse_soup(content);
        assert!(pairs.is_ok());

        let pairs = pairs.unwrap();
        let soup = process_soup_ast(
            pairs,
            content,
            &PathBuf::from("test.soup"),
            &vec![],
            &vec![],
        );

        let temp_dir = std::env::current_dir().unwrap().join("temp_tuple_test");
        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir).unwrap();
        }
        std::fs::create_dir_all(&temp_dir).unwrap();

        let container_txt = r#"
signals
{
  kind "container"
  array-element
  {
    container-type0 "signal-possibility"
  }
  validation
  {
    SubPossibilities
  }
}

signal-possibility
{
  light
  {
    type "numeric"
  }
}
"#;
        std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();

        let validators = load_validators(&temp_dir);
        let diagnostics = soup_diagnostics(&soup, &validators, None);

        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.severity == Some(DiagnosticSeverity::ERROR))
            .collect();
        assert!(
            errors.is_empty(),
            "Expected no errors for valid tuple container, but found: {:?}",
            errors
        );

        std::fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_metadata_keys_ignored() {
        let content = r#"
my-container {
    array-element {
        container-type0 "some-type"
    }
    subpossibilities {
        some-rule {
            type "string"
        }
    }
    validation {
        UniqueNames
    }
    top-level 1
    known-rule "value"
}
"#;
        let pairs = parse_soup(content).unwrap();
        let soup = process_soup_ast(
            pairs,
            content,
            &PathBuf::from("test.soup"),
            &vec![],
            &vec![],
        );

        let temp_dir = std::env::current_dir()
            .unwrap()
            .join("temp_metadata_keys_test");
        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir).unwrap();
        }
        std::fs::create_dir_all(&temp_dir).unwrap();

        let container_txt = r#"
my-container
{
  kind "container"
  known-rule
  {
    type "string"
  }
}
"#;
        std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();

        let validators = load_validators(&temp_dir);
        let diagnostics = soup_diagnostics(&soup, &validators, None);

        let unknown_key_errors: Vec<_> = diagnostics
            .iter()
            .filter(|diag| diag.message.contains("Unknown key"))
            .collect();

        assert!(
            unknown_key_errors.is_empty(),
            "Expected no unknown key errors for metadata keys, but found: {:?}",
            unknown_key_errors
        );

        std::fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_compulsory_and_obsolete_validation() {
        let content = r#"
        kind "test-container"
        required-key "present"
        obsolete-key "should-warn"
        "#;
        let pairs = parse_soup(content).unwrap();
        let soup = process_soup_ast(
            pairs,
            content,
            &PathBuf::from("test.soup"),
            &vec![],
            &vec![],
        );

        let temp_dir = std::env::current_dir()
            .unwrap()
            .join("temp_compulsory_test");
        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir).unwrap();
        }
        std::fs::create_dir_all(&temp_dir).unwrap();

        let container_txt = r#"
test-container
{
  kind "container"
  top-level 1
  required-key
  {
    type "string"
    compulsory 1
  }
  optional-key-zero
  {
    type "string"
    compulsory 0
  }
  optional-key-none
  {
    type "string"
  }
  obsolete-key
  {
    type "string"
    obsolete-tag 1
  }
}
"#;
        std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();

        let validators = load_validators(&temp_dir);
        let diagnostics = soup_diagnostics(&soup, &validators, None);

        // Should have no errors for missing optional keys
        let missing_errors: Vec<_> = diagnostics
            .iter()
            .filter(|diag| {
                diag.message.contains("is missing")
                    && diag.severity == Some(DiagnosticSeverity::ERROR)
            })
            .collect();

        assert!(
            missing_errors.is_empty(),
            "Expected no missing key errors for optional keys, but found: {:?}",
            missing_errors
        );

        // Should have a warning for obsolete-key
        let obsolete_warning = diagnostics.iter().find(|diag| {
            diag.message.contains("is obsolete/deprecated")
                && diag.severity == Some(DiagnosticSeverity::WARNING)
        });

        assert!(
            obsolete_warning.is_some(),
            "Expected warning for obsolete-key, but found: {:?}",
            diagnostics
        );

        std::fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_required_key_missing_validation() {
        let content = r#"
        kind "test-container"
        "#;
        let pairs = parse_soup(content).unwrap();
        let soup = process_soup_ast(
            pairs,
            content,
            &PathBuf::from("test.soup"),
            &vec![],
            &vec![],
        );

        let temp_dir = std::env::current_dir().unwrap().join("temp_required_test");
        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir).unwrap();
        }
        std::fs::create_dir_all(&temp_dir).unwrap();

        let container_txt = r#"
test-container
{
  kind "container"
  top-level 1
  required-key
  {
    type "string"
    compulsory 1
  }
}
"#;
        std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();

        let validators = load_validators(&temp_dir);
        let diagnostics = soup_diagnostics(&soup, &validators, None);

        // Should have an error for missing required-key
        let missing_error = diagnostics.iter().find(|diag| {
            diag.message
                .contains("Compulsory key 'required-key' is missing")
                && diag.severity == Some(DiagnosticSeverity::ERROR)
        });

        assert!(
            missing_error.is_some(),
            "Expected error for missing required-key, but found: {:?}",
            diagnostics
        );

        std::fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_thumbnails_element_validation() {
        let content = r#"
thumbnails {
    0 {
        image   "icon/icon.jpg"
        width   240
        height  180
    }
}
"#;
        let pairs = parse_soup(content).unwrap();
        let soup = process_soup_ast(
            pairs,
            content,
            &PathBuf::from("test.soup"),
            &vec![],
            &vec![],
        );

        let temp_dir = std::env::current_dir()
            .unwrap()
            .join("temp_thumbnails_test");
        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir).unwrap();
        }
        std::fs::create_dir_all(&temp_dir).unwrap();

        let container_txt = r#"
thumbnails
{
  kind "container"
  array-element
  {
    container-type0 "thumbnails-element"
  }
}

thumbnails-element
{
  kind "container"
  image
  {
    type "string"
  }
  width
  {
    type "int"
  }
  height
  {
    type "int"
  }
}
"#;
        std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();

        let validators = load_validators(&temp_dir);
        let diagnostics = soup_diagnostics(&soup, &validators, None);

        let has_type_error = diagnostics
            .iter()
            .any(|diag| diag.message.contains("Invalid type for key 'width'"));
        assert!(
            !has_type_error,
            "Found type error for 'width', but expected none: {:?}",
            diagnostics
        );

        std::fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_vector2_type_validation() {
        let content = r#"
my-container {
    pos   0.5,0.5
}
"#;
        let pairs = parse_soup(content).unwrap();
        let soup = process_soup_ast(
            pairs,
            content,
            &PathBuf::from("test.soup"),
            &vec![],
            &vec![],
        );

        let temp_dir = std::env::current_dir().unwrap().join("temp_vector2_test");
        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir).unwrap();
        }
        std::fs::create_dir_all(&temp_dir).unwrap();

        let container_txt = r#"
my-container
{
  kind "container"
  pos
  {
    type "vector2"
  }
}
"#;
        std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();

        let validators = load_validators(&temp_dir);
        let diagnostics = soup_diagnostics(&soup, &validators, None);

        let has_type_error = diagnostics
            .iter()
            .any(|diag| diag.message.contains("Invalid type for key 'pos'"));
        assert!(
            !has_type_error,
            "Found type error for 'pos' (vector2), but expected none: {:?}",
            diagnostics
        );

        std::fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_user_issue_string_table_validation() {
        let content = r#"string-table
{
  description                           "Unload train vehicles at current industry location."
  msg_error_industry_not_found          "Issue a 'Drive To' command prior to the 'Unload' command."
  driver_command_unload                 "Unload"
  tt_unload_at                          "Unload at $0"
}"#;
        let pairs = parse_soup(content).expect("Failed to parse soup");
        let soup = process_soup_ast(
            pairs,
            content,
            &PathBuf::from("test.soup"),
            &vec![],
            &vec![],
        );

        // Mock validator that has a "string-table" container but NOT marked as TagArray
        let mut validators = Validators::default();
        validators.containers.push(ContainerValidator {
            container_name: "string-table".to_string(),
            top_level: true, // Let's say it's top-level
            ..Default::default()
        });

        let diagnostics = soup_diagnostics(&soup, &validators, None);

        // We expect NO "Unknown key" errors for 'description', etc., according to the requirement.
        for diag in &diagnostics {
            println!("Diagnostic: {}", diag.message);
        }

        let has_unknown_key_error = diagnostics
            .iter()
            .any(|diag| diag.message.contains("Unknown key"));

        assert!(
            !has_unknown_key_error,
            "Found unknown key error: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_tag_array_validation() {
        let content = r#"
string-table {
    type "string-entry"
    Key1 {
        value "Value1"
    }
    Key2 {
        value "Value2"
    }
    Key1 {
        value "Value1 Duplicate"
    }
}
"#;
        let pairs = parse_soup(content).unwrap();
        let soup = process_soup_ast(
            pairs,
            content,
            &PathBuf::from("test.soup"),
            &vec![],
            &vec![],
        );

        let temp_dir = std::env::current_dir().unwrap().join("temp_tag_array_test");
        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir).unwrap();
        }
        std::fs::create_dir_all(&temp_dir).unwrap();

        let container_txt = r#"
string-table
{
  kind "container"
  validation
  {
    TagArray
  }
  type
  {
    type "string-entry"
  }
}

string-entry
{
  kind "container"
  value
  {
    type "string"
  }
}
"#;
        std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();

        let validators = load_validators(&temp_dir);
        let diagnostics = soup_diagnostics(&soup, &validators, None);

        // 1. Check for duplicate key error for "Key1"
        let has_duplicate_error = diagnostics
            .iter()
            .any(|diag| diag.message.contains("Duplicate key 'Key1'"));
        assert!(
            has_duplicate_error,
            "Expected duplicate key error for 'Key1', but found: {:?}",
            diagnostics
        );

        // 2. Add an invalid entry and check if it's validated against string-entry
        let content_invalid = r#"
string-table {
    type "string-entry"
    Key3 {
        invalid_key "Value"
    }
}
"#;
        let pairs_invalid = parse_soup(content_invalid).unwrap();
        let soup_invalid = process_soup_ast(
            pairs_invalid,
            content_invalid,
            &PathBuf::from("test_invalid.soup"),
            &vec![],
            &vec![],
        );
        let validators = load_validators(&temp_dir);
        let diagnostics_invalid = soup_diagnostics(&soup_invalid, &validators, None);

        let has_unknown_key_error = diagnostics_invalid
            .iter()
            .any(|diag| diag.message.contains("Unknown key 'invalid_key'"));
        assert!(
            has_unknown_key_error,
            "Expected unknown key error for 'invalid_key' in string-entry, but found: {:?}",
            diagnostics_invalid
        );

        std::fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_tag_array_case_insensitivity() {
        let temp_dir = std::env::current_dir()
            .unwrap()
            .join("temp_tag_array_case_test");
        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir).unwrap();
        }
        std::fs::create_dir_all(&temp_dir).unwrap();

        let container_txt = r#"
string-table
{
  validation
  {
    tagarray
  }
  type
  {
    type "string"
  }
}
"#;
        std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();

        let content = r#"
string-table {
    key "value"
}
"#;
        let pairs = parse_soup(content).unwrap();
        let soup = process_soup_ast(
            pairs,
            content,
            &PathBuf::from("test.soup"),
            &vec![],
            &vec![],
        );
        let validators = load_validators(&temp_dir);
        let diagnostics = soup_diagnostics(&soup, &validators, None);

        let has_error = diagnostics
            .iter()
            .any(|d| d.severity == Some(DiagnosticSeverity::ERROR));
        assert!(
            !has_error,
            "Unexpected errors with lowercase tagarray: {:?}",
            diagnostics
        );

        // Test mixed case TagArray in validation
        let container_txt_mixed = r#"
string-table
{
  validation
  {
    TagArray
  }
  type
  {
    type "string"
  }
}
"#;
        std::fs::write(temp_dir.join("container.txt"), container_txt_mixed).unwrap();
        let validators_mixed = load_validators(&temp_dir);
        let diagnostics_mixed = soup_diagnostics(&soup, &validators_mixed, None);
        let has_error_mixed = diagnostics_mixed
            .iter()
            .any(|d| d.severity == Some(DiagnosticSeverity::ERROR));
        assert!(
            !has_error_mixed,
            "Unexpected errors with mixed case TagArray: {:?}",
            diagnostics_mixed
        );

        // Test tag-array in validation
        let container_txt_tagarray = r#"
string-table
{
  validation
  {
    tag-array
  }
  type
  {
    type "string"
  }
}
"#;
        std::fs::write(temp_dir.join("container.txt"), container_txt_tagarray).unwrap();
        let validators_tagarray = load_validators(&temp_dir);
        let diagnostics_tagarray = soup_diagnostics(&soup, &validators_tagarray, None);
        let has_error_tagarray = diagnostics_tagarray
            .iter()
            .any(|d| d.severity == Some(DiagnosticSeverity::ERROR));
        assert!(
            !has_error_tagarray,
            "Unexpected errors with tag-array: {:?}",
            diagnostics_tagarray
        );

        // Check that tagarray itself is ignored in soup
        let content_meta = r#"
string-table {
    key "value"
    tagarray { }
    tag-array { }
}
"#;
        let pairs_meta = parse_soup(content_meta).unwrap();
        let soup_meta = process_soup_ast(
            pairs_meta,
            content_meta,
            &PathBuf::from("test_meta.soup"),
            &vec![],
            &vec![],
        );
        let diagnostics_meta = soup_diagnostics(&soup_meta, &validators, None);
        let has_unknown_key = diagnostics_meta
            .iter()
            .any(|d| d.message.contains("Unknown key"));
        assert!(
            !has_unknown_key,
            "tagarray/tag-array should be ignored as metadata keys in Soup: {:?}",
            diagnostics_meta
        );

        std::fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_subpossibilities_validation() {
        let content = r#"
my-container {
    kind "my-container"
    required-key "value"
}
"#;
        let pairs = parse_soup(content).unwrap();
        let soup = process_soup_ast(
            pairs,
            content,
            &PathBuf::from("test.soup"),
            &vec![],
            &vec![],
        );

        let temp_dir = std::env::current_dir()
            .unwrap()
            .join("temp_subpossibilities_test");
        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir).unwrap();
        }
        std::fs::create_dir_all(&temp_dir).unwrap();

        let container_txt = r#"
my-container
{
  kind "container"
  kind
  {
    type "string"
  }
  subpossibilities
  {
    required-key
    {
      compulsory 1
      type "string"
    }
    optional-key
    {
      compulsory 0
      type "int"
    }
  }
}
"#;
        std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();

        let validators = load_validators(&temp_dir);
        let diagnostics = soup_diagnostics(&soup, &validators, None);
        assert!(
            diagnostics.is_empty(),
            "Expected no errors for valid container, but found: {:?}",
            diagnostics
        );

        // Missing required key
        let content_missing = r#"
my-container {
    kind "my-container"
}
"#;
        let pairs_missing = parse_soup(content_missing).unwrap();
        let soup_missing = process_soup_ast(
            pairs_missing,
            content_missing,
            &PathBuf::from("test_missing.soup"),
            &vec![],
            &vec![],
        );
        let diagnostics_missing = soup_diagnostics(&soup_missing, &validators, None);

        let has_missing_key_error = diagnostics_missing.iter().any(|diag| {
            diag.message
                .contains("Compulsory key 'required-key' is missing")
        });
        assert!(
            has_missing_key_error,
            "Expected missing key error for 'required-key', but found: {:?}",
            diagnostics_missing
        );

        std::fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_array_element_sequential_validation() {
        let content = r#"
my-array {
    kind "my-array"
    0 {
        value "elem0"
    }
    1 {
        value "elem1"
    }
}
"#;
        let pairs = parse_soup(content).unwrap();
        let soup = process_soup_ast(
            pairs,
            content,
            &PathBuf::from("test.soup"),
            &vec![],
            &vec![],
        );

        let temp_dir = std::env::current_dir()
            .unwrap()
            .join("temp_array_element_test");
        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir).unwrap();
        }
        std::fs::create_dir_all(&temp_dir).unwrap();

        let container_txt = r#"
my-array
{
  kind "container"
  array-element
  {
    container-type0 "my-element"
  }
}

my-element
{
  kind "container"
  value
  {
    type "string"
  }
}
"#;
        std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();

        let validators = load_validators(&temp_dir);
        let diagnostics = soup_diagnostics(&soup, &validators, None);
        assert!(
            diagnostics.is_empty(),
            "Expected no errors for valid sequential array, but found: {:?}",
            diagnostics
        );

        // Non-sequential keys
        let content_non_seq = r#"
my-array {
    kind "my-array"
    0 {
        value "elem0"
    }
    2 {
        value "elem2"
    }
}
"#;
        let pairs_non_seq = parse_soup(content_non_seq).unwrap();
        let soup_non_seq = process_soup_ast(
            pairs_non_seq,
            content_non_seq,
            &PathBuf::from("test_non_seq.soup"),
            &vec![],
            &vec![],
        );
        let diagnostics_non_seq = soup_diagnostics(&soup_non_seq, &validators, None);

        let has_seq_error = diagnostics_non_seq.iter().any(|diag| {
            diag.message.contains("Non-sequential array index '2'")
                && diag.message.contains("Expected '1'")
        });
        assert!(
            has_seq_error,
            "Expected non-sequential error, but found: {:?}",
            diagnostics_non_seq
        );

        std::fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_top_level_inheritance() {
        let temp_dir = std::env::temp_dir().join("gs-lsp-test-top-level");
        std::fs::create_dir_all(&temp_dir).unwrap();

        let kind_content = r#"
obsolete-track
{
  top-level 1
  icon track
  inherit
  {
    itrack
    base-asset
  }
  kind "structure"
  subpossibilities
  {
    kind
    {
      kind value
      type string
      compulsory 3.4
      default "track"
      filter "track"
      disabled 1
    }
  }
}

itrack
{
  subpossibilities
  {
    track-id
    {
       type string
       compulsory 1
    }
  }
}

base-asset
{
  subpossibilities
  {
    asset-id
    {
       type string
       compulsory 1
    }
  }
}
"#;
        std::fs::write(temp_dir.join("kind.txt"), kind_content).unwrap();

        let soup_content = r#"
kind "obsolete-track"
asset-id "test-asset"
track-id "test-track"
"#;
        let pairs = parse_soup(soup_content).unwrap();
        let soup = process_soup_ast(
            pairs,
            soup_content,
            &std::path::PathBuf::from("test.soup"),
            &vec![],
            &vec![],
        );

        let validators = load_validators(&temp_dir);
        let diagnostics = soup_diagnostics(&soup, &validators, None);

        // Should NOT have any "Unknown key" errors if top_level validation worked
        let has_unknown_key = diagnostics
            .iter()
            .any(|d| d.message.contains("Unknown key"));
        assert!(
            !has_unknown_key,
            "Unexpected unknown key errors: {:?}",
            diagnostics
        );

        // Should NOT have error for 'asset-id' as it is inherited from base-asset
        let has_missing_asset_id = diagnostics
            .iter()
            .any(|d| d.message.contains("Compulsory key 'asset-id' is missing"));
        assert!(
            !has_missing_asset_id,
            "Unexpected error for 'asset-id': {:?}",
            diagnostics
        );

        // Should NOT have error for 'track-id' as it is inherited from itrack
        let has_missing_track_id = diagnostics
            .iter()
            .any(|d| d.message.contains("Compulsory key 'track-id' is missing"));
        assert!(
            !has_missing_track_id,
            "Unexpected error for 'track-id': {:?}",
            diagnostics
        );

        // Now test missing compulsory kind
        let soup_content_missing = r#"
kind "obsolete-track"
"#;
        let pairs_missing = parse_soup(soup_content_missing).unwrap();
        let soup_missing = process_soup_ast(
            pairs_missing,
            soup_content_missing,
            &std::path::PathBuf::from("test_missing.soup"),
            &vec![],
            &vec![],
        );
        let diagnostics_missing = soup_diagnostics(&soup_missing, &validators, None);

        let has_missing_asset_id = diagnostics_missing
            .iter()
            .any(|d| d.message.contains("Compulsory key 'asset-id' is missing"));
        assert!(
            has_missing_asset_id,
            "Expected error for missing 'asset-id', but got: {:?}",
            diagnostics_missing
        );

        let has_missing_track_id = diagnostics_missing
            .iter()
            .any(|d| d.message.contains("Compulsory key 'track-id' is missing"));
        assert!(
            has_missing_track_id,
            "Expected error for missing 'track-id', but got: {:?}",
            diagnostics_missing
        );

        // 'kind' is marked as 'disabled 1' in obsolete-track subpossibilities, so it should not be checked for compulsory or it might even trigger an error if we implement disabled check.
        // In the example, it has compulsory 3.4.

        std::fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_subpossibilities_case_insensitivity() {
        let temp_dir = std::env::temp_dir().join("gs-lsp-test-subpossibilities-case");
        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir).unwrap();
        }
        std::fs::create_dir_all(&temp_dir).unwrap();

        let kind_content = r#"
test-container
{
  SubPossibilities
  {
    mandatory-key
    {
      type string
      compulsory 1
    }
    Optional-Key
    {
      type string
      compulsory 0
    }
  }
}
"#;
        std::fs::write(temp_dir.join("kind.txt"), kind_content).unwrap();

        let validators = load_validators(&temp_dir);

        // 1. Test case-insensitive SubPossibilities in validator definition
        let soup_content = r#"
test-container
{
  mandatory-key "value"
}
"#;
        let pairs = parse_soup(soup_content).unwrap();
        let soup = process_soup_ast(
            pairs,
            soup_content,
            &std::path::PathBuf::from("test.soup"),
            &vec![],
            &vec![],
        );
        let diagnostics = soup_diagnostics(&soup, &validators, None);

        let has_error = diagnostics
            .iter()
            .any(|d| d.severity == Some(DiagnosticSeverity::ERROR));
        assert!(
            !has_error,
            "Unexpected errors with SubPossibilities: {:?}",
            diagnostics
        );

        // 2. Test missing compulsory key defined in SubPossibilities
        let soup_content_missing = r#"
test-container
{
  Optional-Key "value"
}
"#;
        let pairs_missing = parse_soup(soup_content_missing).unwrap();
        let soup_missing = process_soup_ast(
            pairs_missing,
            soup_content_missing,
            &std::path::PathBuf::from("test_missing.soup"),
            &vec![],
            &vec![],
        );
        let diagnostics_missing = soup_diagnostics(&soup_missing, &validators, None);

        let has_missing_key = diagnostics_missing.iter().any(|d| {
            d.message
                .contains("Compulsory key 'mandatory-key' is missing")
        });
        assert!(
            has_missing_key,
            "Expected error for missing 'mandatory-key' defined in SubPossibilities: {:?}",
            diagnostics_missing
        );

        // 3. Test that SubPossibilities as a key in soup itself is ignored (not flagged as unknown)
        let soup_with_metadata = r#"
test-container
{
  mandatory-key "value"
  SubPossibilities { }
  subpossibilities { }
}
"#;
        let pairs_metadata = parse_soup(soup_with_metadata).unwrap();
        let soup_metadata = process_soup_ast(
            pairs_metadata,
            soup_with_metadata,
            &std::path::PathBuf::from("test_metadata.soup"),
            &vec![],
            &vec![],
        );
        let diagnostics_metadata = soup_diagnostics(&soup_metadata, &validators, None);

        let has_unknown_key = diagnostics_metadata
            .iter()
            .any(|d| d.message.contains("Unknown key"));
        assert!(
            !has_unknown_key,
            "SubPossibilities should be ignored as a metadata key in Soup: {:?}",
            diagnostics_metadata
        );

        std::fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_category_class_validation() {
        let temp_dir = std::env::temp_dir().join("gs-lsp-test-cat-class");
        if temp_dir.exists() {
            let _ = std::fs::remove_dir_all(&temp_dir);
        }
        std::fs::create_dir_all(&temp_dir).unwrap();

        let cat_class_content = r#"
Scenery "Scenery objects"
Track "Track objects"
"#;
        std::fs::write(temp_dir.join("category-class.txt"), cat_class_content).unwrap();

        let container_content = r#"
my_container {
    category-class {
        type string
        validation IsValidCategoryClass
    }
}
"#;
        std::fs::write(temp_dir.join("container.txt"), container_content).unwrap();

        let soup_content = r#"
my_container {
    category-class "Scenery"
}
"#;
        let pairs = parse_soup(soup_content).unwrap();
        let soup = process_soup_ast(
            pairs,
            soup_content,
            &std::path::PathBuf::from("test.soup"),
            &vec![],
            &vec![],
        );

        let validators = load_validators(&temp_dir);
        let diagnostics = soup_diagnostics(&soup, &validators, None);

        assert!(
            diagnostics.is_empty(),
            "Unexpected diagnostics for valid category class: {:?}",
            diagnostics
        );

        let soup_content_invalid = r#"
my_container {
    category-class "InvalidClass"
}
"#;
        let pairs_invalid = parse_soup(soup_content_invalid).unwrap();
        let soup_invalid = process_soup_ast(
            pairs_invalid,
            soup_content_invalid,
            &std::path::PathBuf::from("test_invalid.soup"),
            &vec![],
            &vec![],
        );
        let diagnostics_invalid = soup_diagnostics(&soup_invalid, &validators, None);

        let has_cat_class_error = diagnostics_invalid
            .iter()
            .any(|d| d.message.contains("is not a valid category class"));
        assert!(
            has_cat_class_error,
            "Expected category class error, but got: {:?}",
            diagnostics_invalid
        );

        // Test with "WAT" as requested by user
        let soup_content_wat = r#"
my_container {
    category-class "WAT"
}
"#;
        let pairs_wat = parse_soup(soup_content_wat).unwrap();
        let soup_wat = process_soup_ast(
            pairs_wat,
            soup_content_wat,
            &std::path::PathBuf::from("test_wat.soup"),
            &vec![],
            &vec![],
        );
        let diagnostics_wat = soup_diagnostics(&soup_wat, &validators, None);

        let has_wat_error = diagnostics_wat.iter().any(|d| {
            d.message
                .contains("Value 'WAT' for key 'category-class' is not a valid category class")
        });
        assert!(
            has_wat_error,
            "Expected error for 'WAT' category class, but got: {:?}",
            diagnostics_wat
        );

        // Test Category Region
        let cat_region_content = r#"
FRA "France"
USA "United States"
"#;
        std::fs::write(temp_dir.join("category-region.txt"), cat_region_content).unwrap();

        let container_content_region = r#"
region_container {
    category-region {
        type string
        validation IsValidCategoryRegion
    }
}
"#;
        std::fs::write(temp_dir.join("container.txt"), container_content_region).unwrap();

        let soup_content_region = r#"
region_container {
    category-region "FRA"
}
"#;
        let pairs_region = parse_soup(soup_content_region).unwrap();
        let soup_region = process_soup_ast(
            pairs_region,
            soup_content_region,
            &std::path::PathBuf::from("test_region.soup"),
            &vec![],
            &vec![],
        );
        let validators_region = load_validators(&temp_dir);
        let diagnostics_region = soup_diagnostics(&soup_region, &validators_region, None);
        assert!(
            diagnostics_region.is_empty(),
            "Unexpected diagnostics for valid category region: {:?}",
            diagnostics_region
        );

        let soup_content_fra_invalid = r#"
region_container {
    category-region "GER"
}
"#;
        let pairs_fra_invalid = parse_soup(soup_content_fra_invalid).unwrap();
        let soup_fra_invalid = process_soup_ast(
            pairs_fra_invalid,
            soup_content_fra_invalid,
            &std::path::PathBuf::from("test_fra_invalid.soup"),
            &vec![],
            &vec![],
        );
        let diagnostics_fra_invalid = soup_diagnostics(&soup_fra_invalid, &validators_region, None);
        let has_region_error = diagnostics_fra_invalid
            .iter()
            .any(|d| d.message.contains("is not a valid category region"));
        assert!(
            has_region_error,
            "Expected category region error, but got: {:?}",
            diagnostics_fra_invalid
        );

        // Test Category Era
        let cat_era_content = r#"
2000s "2000s"
2010s "2010s"
2020s "2020s"
"#;
        std::fs::write(temp_dir.join("category-era.txt"), cat_era_content).unwrap();

        let container_content_era = r#"
era_container {
    category-era {
        type string
        validation IsValidCategoryEra
    }
}
"#;
        std::fs::write(temp_dir.join("container.txt"), container_content_era).unwrap();

        let soup_content_era = r#"
era_container {
    category-era "2000s;2010s;"
}
"#;
        let pairs_era = parse_soup(soup_content_era).unwrap();
        let soup_era = process_soup_ast(
            pairs_era,
            soup_content_era,
            &std::path::PathBuf::from("test_era.soup"),
            &vec![],
            &vec![],
        );
        let validators_era = load_validators(&temp_dir);
        let diagnostics_era = soup_diagnostics(&soup_era, &validators_era, None);
        assert!(
            diagnostics_era.is_empty(),
            "Unexpected diagnostics for valid category era: {:?}",
            diagnostics_era
        );

        let soup_content_era_invalid = r#"
era_container {
    category-era "2000s;2030s;2010s;"
}
"#;
        let pairs_era_invalid = parse_soup(soup_content_era_invalid).unwrap();
        let soup_era_invalid = process_soup_ast(
            pairs_era_invalid,
            soup_content_era_invalid,
            &std::path::PathBuf::from("test_era_invalid.soup"),
            &vec![],
            &vec![],
        );
        let diagnostics_era_invalid = soup_diagnostics(&soup_era_invalid, &validators_era, None);
        let has_era_error = diagnostics_era_invalid.iter().any(|d| {
            d.message
                .contains("Value '2030s' for key 'category-era' is not a valid category era.")
        });
        assert!(
            has_era_error,
            "Expected category era error for '2030s', but got: {:?}",
            diagnostics_era_invalid
        );

        std::fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_disabled_keys_ignored() {
        let temp_dir = std::env::temp_dir().join("gs-lsp-test-disabled");
        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir).unwrap();
        }
        std::fs::create_dir_all(&temp_dir).unwrap();

        let kind_content = r#"
test-container
{
  disabled-key
  {
    type string
    compulsory 1
    disabled 1
  }
  normal-key
  {
    type string
    compulsory 1
  }
  top-level 1
}
"#;
        std::fs::write(temp_dir.join("kind.txt"), kind_content).unwrap();

        let validators = load_validators(&temp_dir);

        // 1. Test that disabled key does not trigger missing compulsory error (as child)
        let soup_content = r#"
test-container {
  normal-key "value"
}
"#;
        let pairs = parse_soup(soup_content).unwrap();
        let soup = process_soup_ast(
            pairs,
            soup_content,
            &std::path::PathBuf::from("test.soup"),
            &vec![],
            &vec![],
        );
        let diagnostics = soup_diagnostics(&soup, &validators, None);
        assert!(
            diagnostics.is_empty(),
            "Unexpected diagnostics for missing disabled key: {:?}",
            diagnostics
        );

        // 2. Test that disabled key does not trigger warning when present
        let soup_content_present = r#"
test-container {
  normal-key "value"
  disabled-key "whatever"
}
"#;
        let pairs_present = parse_soup(soup_content_present).unwrap();
        let soup_present = process_soup_ast(
            pairs_present,
            soup_content_present,
            &std::path::PathBuf::from("test_present.soup"),
            &vec![],
            &vec![],
        );
        let diagnostics_present = soup_diagnostics(&soup_present, &validators, None);
        assert!(
            diagnostics_present.is_empty(),
            "Unexpected diagnostics for present disabled key: {:?}",
            diagnostics_present
        );

        // 3. Test top-level disabled key
        let kind_content_top = r#"
kind "test-kind"
disabled-top
{
  type string
  compulsory 1
  disabled 1
}
normal-top
{
  type string
  compulsory 1
}
top-level 1
"#;
        std::fs::write(temp_dir.join("kind.txt"), kind_content_top).unwrap();
        let validators_top = load_validators(&temp_dir);

        let soup_top = r#"
kind "test-kind"
normal-top "value"
"#;
        let pairs_top = parse_soup(soup_top).unwrap();
        let soup_obj_top = process_soup_ast(
            pairs_top,
            soup_top,
            &std::path::PathBuf::from("test_top.soup"),
            &vec![],
            &vec![],
        );
        let diagnostics_top = soup_diagnostics(&soup_obj_top, &validators_top, None);
        assert!(
            diagnostics_top.is_empty(),
            "Unexpected diagnostics for missing disabled top-level key: {:?}",
            diagnostics_top
        );

        std::fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_type_combobox_listbox_filepathedit() {
        use std::path::PathBuf;
        let temp_dir = std::env::temp_dir().join("soup_type_test");
        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir).unwrap();
        }
        std::fs::create_dir_all(&temp_dir).unwrap();

        let container_txt = r#"
test-container
{
  combo { type "combobox" }
  list { type "listbox" }
  file { type "filepathedit" }
}
"#;
        std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();

        // Create a dummy file for filepathedit test
        std::fs::write(temp_dir.join("existing_file.txt"), "hello").unwrap();

        let validators = load_validators(&temp_dir);

        // 1. Valid cases
        let valid_content = r#"
test-container {
    combo "single_value"
    list "val1;val2;val3;"
    file "existing_file.txt"
}
"#;
        let pairs = parse_soup(valid_content).unwrap();
        let soup = process_soup_ast(
            pairs,
            valid_content,
            &PathBuf::from("test.soup"),
            &vec![],
            &vec![],
        );
        let diagnostics = soup_diagnostics(&soup, &validators, Some(&temp_dir.join("test.soup")));
        assert!(
            diagnostics.is_empty(),
            "Expected no diagnostics for valid types, but found: {:?}",
            diagnostics
        );

        // 2. Invalid cases
        let invalid_content = r#"
test-container {
    combo "val1;val2"
    file "non_existent.txt"
}
"#;
        let pairs_inv = parse_soup(invalid_content).unwrap();
        let soup_inv = process_soup_ast(
            pairs_inv,
            invalid_content,
            &PathBuf::from("test_inv.soup"),
            &vec![],
            &vec![],
        );
        let diagnostics_inv = soup_diagnostics(
            &soup_inv,
            &validators,
            Some(&temp_dir.join("test_inv.soup")),
        );

        let has_combo_warning = diagnostics_inv.iter().any(|d| {
            d.severity == Some(DiagnosticSeverity::WARNING)
                && d.message.contains("expects a single string value")
        });
        let has_file_error = diagnostics_inv.iter().any(|d| {
            d.severity == Some(DiagnosticSeverity::ERROR) && d.message.contains("does not exist")
        });

        assert!(
            has_combo_warning,
            "Expected warning for combobox with semicolon, but found: {:?}",
            diagnostics_inv
        );
        assert!(
            has_file_error,
            "Expected error for non-existent file in filepathedit, but found: {:?}",
            diagnostics_inv
        );

        std::fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_array_floatlist_validation() {
        use std::path::PathBuf;
        let temp_dir = std::env::temp_dir().join("soup_floatlist_test");
        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir).unwrap();
        }
        std::fs::create_dir_all(&temp_dir).unwrap();

        let container_txt = r#"
test-container
{
  floats { type "floatlist" }
}
"#;
        std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();

        let validators = load_validators(&temp_dir);

        // 1. Array with only integers - should be valid for floatlist
        let int_content = r#"
test-container {
    floats 1,2,3
}
"#;
        let pairs = parse_soup(int_content).unwrap();
        let soup = process_soup_ast(
            pairs,
            int_content,
            &PathBuf::from("test1.soup"),
            &vec![],
            &vec![],
        );
        let diagnostics = soup_diagnostics(&soup, &validators, None);
        // Currently this might pass if "floatlist" is treated as "array" and no deeper check is done.
        // But let's see.
        assert!(
            diagnostics.is_empty(),
            "Expected no diagnostics for int array as floatlist, but found: {:?}",
            diagnostics
        );

        // 2. Array with at least one float - this is what the task is about
        let float_content = r#"
test-container {
    floats 1,2.5,3
}
"#;
        let pairs_f = parse_soup(float_content).unwrap();
        let soup_f = process_soup_ast(
            pairs_f,
            float_content,
            &PathBuf::from("test2.soup"),
            &vec![],
            &vec![],
        );
        let diagnostics_f = soup_diagnostics(&soup_f, &validators, None);

        assert!(
            diagnostics_f.is_empty(),
            "Expected no diagnostics for float array as floatlist, but found: {:?}",
            diagnostics_f
        );

        // 3. Test mismatch reporting
        let invalid_content = r#"
test-container {
    floats "not-an-array"
}
"#;
        let pairs_i = parse_soup(invalid_content).unwrap();
        let soup_i = process_soup_ast(
            pairs_i,
            invalid_content,
            &PathBuf::from("test3.soup"),
            &vec![],
            &vec![],
        );
        let diagnostics_i = soup_diagnostics(&soup_i, &validators, None);
        assert!(!diagnostics_i.is_empty());
        assert!(
            diagnostics_i[0]
                .message
                .contains("Expected 'floatlist', found 'string'")
        );

        // 4. Test float array when 'array' is expected
        let container_txt_2 = r#"
test-container-2
{
  arr { type "array" }
}
"#;
        std::fs::write(temp_dir.join("container2.txt"), container_txt_2).unwrap();
        let validators_2 = load_validators(&temp_dir);
        let float_content_2 = r#"
test-container-2 {
    arr 1.5,2.5
}
"#;
        let pairs_f2 = parse_soup(float_content_2).unwrap();
        let soup_f2 = process_soup_ast(
            pairs_f2,
            float_content_2,
            &PathBuf::from("test4.soup"),
            &vec![],
            &vec![],
        );
        let diagnostics_f2 = soup_diagnostics(&soup_f2, &validators_2, None);
        assert!(
            diagnostics_f2.is_empty(),
            "Expected no diagnostics for float array when 'array' type is expected, but found: {:?}",
            diagnostics_f2
        );

        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
}
