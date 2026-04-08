use log::error;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use trainz_ast::soup::process::process_soup_ast;
use trainz_ast::soup::value::{NumericValue, Value};
use trainz_parser::soup::parse_soup;

use crate::models::{ContainerRule, ContainerValidator, SoupValidator, Validators};

fn parse_rule(
    key: String,
    rule_details: Vec<trainz_ast::soup::key_value_pair::KeyValuePair>,
) -> ContainerRule {
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
                        _ => false,
                    };

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
                                                    top_level =
                                                        s == "1" || s == "true" || s == "container";
                                                }
                                                "validation" => {
                                                    validation = Some(s.clone());
                                                    if s.eq_ignore_ascii_case("TagArray") {
                                                        tag_array = true;
                                                    }
                                                }
                                                "inherit" => {
                                                    inherit.push(s);
                                                }
                                                "subpossibilities" | "array-element"
                                                | "tagarray" | "uniquenames" => {
                                                    // Already handled or special metadata
                                                }
                                                _ => {
                                                    // It's a simple rule override (e.g., author "string")
                                                    rules.push(ContainerRule {
                                                        key: rule_kv.key.clone(),
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
                                        Some(Value::Numeric(n, _)) => {
                                            let key_lower = rule_kv.key.to_lowercase();
                                            match key_lower.as_str() {
                                                "top-level" => {
                                                    top_level = match n {
                                                        NumericValue::Int(val) => val == 1,
                                                        NumericValue::Float(val) => {
                                                            (val - 1.0).abs() < 1e-9
                                                        }
                                                        _ => false,
                                                    };
                                                }
                                                "compulsory" => {
                                                    // parsed as a rule at container level? usually it's inside rule_details
                                                }
                                                _ => {}
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
                                    container_name: kv.key.clone(),
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
                            category_classes.insert(kv.key.clone(), description);
                        }
                    } else if full_filename_lower == "category-region.txt" {
                        for kv in validator_soup.key_value_pairs {
                            let description = match kv.value {
                                Some(Value::String(s, _)) => s,
                                Some(Value::Variable(s, _)) => s,
                                _ => String::new(),
                            };
                            category_regions.insert(kv.key.clone(), description);
                        }
                    } else if full_filename_lower == "category-era.txt" {
                        for kv in validator_soup.key_value_pairs {
                            let description = match kv.value {
                                Some(Value::String(s, _)) => s,
                                Some(Value::Variable(s, _)) => s,
                                _ => String::new(),
                            };
                            category_eras.insert(kv.key.clone(), description);
                        }
                    } else {
                        let mut allowed_values = HashMap::new();
                        for kv in validator_soup.key_value_pairs {
                            let description = match kv.value {
                                Some(Value::String(s, _)) => s,
                                Some(Value::Variable(s, _)) => s,
                                _ => String::new(),
                            };
                            allowed_values.insert(kv.key.clone(), description);
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
