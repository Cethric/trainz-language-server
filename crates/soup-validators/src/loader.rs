use rayon::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tracing::{debug, error, trace, warn};
use trainz_ast::soup::process::process_soup_ast;
use trainz_ast::soup::value::{NumericValue, Value};
use trainz_ast::soup::{KeyValuePair, Soup};
use trainz_parser::soup::parse_soup;

use crate::Validation;
use crate::models::{ArrayElementType, ContainerRule, ContainerValidator, Validators};

fn parse_rule(key: String, rule_details: Vec<KeyValuePair>) -> ContainerRule {
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
                if let Some(Value::Container(details, _, _)) = detail.value {
                    rule_validation = parse_validation(&details);
                } else if let Some(Value::String(s, _)) | Some(Value::Variable(s, _)) = detail.value
                {
                    rule_validation = Some(vec![Validation::Named(s)]);
                } else {
                    warn!("Invalid validation value: {:?}", detail.value);
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

fn parse_simple_validator(validator_soup: Soup) -> HashMap<String, Option<String>> {
    let mut allowed_values: HashMap<String, Option<String>> = HashMap::new();
    for kv in validator_soup.key_value_pairs {
        let description = match kv.value {
            Some(Value::String(s, _)) => Some(s),
            Some(Value::Variable(s, _)) => Some(s),
            val => {
                trace!("Unexpected value type: {:?}", val);
                None
            }
        };
        allowed_values.insert(kv.key.clone(), description);
    }

    allowed_values
}

fn parse_numeric_as_bool(numeric_value: &NumericValue) -> bool {
    match numeric_value {
        NumericValue::Float(v) => *v > 0f64,
        NumericValue::Hex(v) => *v > 0,
        NumericValue::Int(v) => *v > 0,
    }
}

fn parse_validation(validators: &Vec<KeyValuePair>) -> Option<Vec<Validation>> {
    let mut validations = vec![];

    for validator in validators {
        if validator.key.eq_ignore_ascii_case("range") {
            if let Some(Value::Container(details, _, _)) = &validator.value {
                let mut min: Option<NumericValue> = None;
                let mut max: Option<NumericValue> = None;
                for range in details {
                    if range.key.eq_ignore_ascii_case("min")
                        && let Some(Value::Numeric(min_value, _)) = &range.value
                    {
                        min = Some(min_value.clone());
                    } else if range.key.eq_ignore_ascii_case("max")
                        && let Some(Value::Numeric(max_value, _)) = &range.value
                    {
                        max = Some(max_value.clone());
                    }
                }

                if let Some(min_value) = min
                    && let Some(max_value) = max
                {
                    if let NumericValue::Float(float_min) = min_value
                        && let NumericValue::Float(float_max) = max_value
                    {
                        validations.push(Validation::FloatRange(float_min, float_max));
                        continue;
                    } else if let NumericValue::Hex(float_min) = min_value
                        && let NumericValue::Hex(float_max) = max_value
                    {
                        validations.push(Validation::HexRange(float_min, float_max));
                        continue;
                    } else if let NumericValue::Int(float_min) = min_value
                        && let NumericValue::Int(float_max) = max_value
                    {
                        validations.push(Validation::IntRange(float_min, float_max));
                        continue;
                    }
                }
            }
            warn!("Invalid range validator: {:?}", validator.value);
            continue;
        } else if validator.key.eq_ignore_ascii_case("NotOwnParent") {
            warn!(
                "NotOwnParent validator not yet implemented {} {:?}",
                validator.key, validator.value
            );
            validations.push(Validation::NotOwnParent)
        } else if validator.key.eq_ignore_ascii_case("NeedCollateMeshes") {
            warn!(
                "NeedCollateMeshes validator not yet implemented {} {:?}",
                validator.key, validator.value
            );
            validations.push(Validation::NeedCollateMeshes(vec![]))
        } else {
            validations.push(Validation::Named(validator.key.clone()));
            continue;
        }
    }

    Some(validations)
}

type SimpleValidators = HashMap<String, HashMap<String, Option<String>>>;
type ContainerValidators = Vec<ContainerValidator>;

fn process_file(
    filename: &str,
    validator_content: &str,
) -> Option<(SimpleValidators, ContainerValidators)> {
    if let Ok(pairs) = parse_soup(validator_content) {
        let validator_soup = process_soup_ast(pairs, validator_content);

        // Look for existing container name or create new
        let is_container_style = validator_soup
            .key_value_pairs
            .par_iter()
            .any(|kv| matches!(kv.value, Some(Value::Container(_, _, _))));

        let mut simple_validators: SimpleValidators = HashMap::new();
        let mut container_validators: ContainerValidators = vec![];

        if is_container_style {
            for kv in validator_soup.key_value_pairs {
                if let Some(Value::Container(container_kv, _, _)) = kv.value {
                    let mut rules = vec![];
                    let mut array_element: Option<ArrayElementType> = None;
                    let mut validation = None;
                    let mut inherit = vec![];
                    let mut top_level = false;
                    let mut subpossibilities = vec![];
                    let mut tag_array = None;
                    let mut allow_any_key = false;

                    for rule_kv in container_kv {
                        match rule_kv.value {
                            Some(Value::Container(rule_details, _, _)) => {
                                if rule_kv.key.eq_ignore_ascii_case("array-element") {
                                    let mut types = Vec::new();
                                    for detail in rule_details {
                                        if detail.key.starts_with("container-type")
                                            && let Some(val) = match detail.value {
                                                Some(Value::String(s, _))
                                                | Some(Value::Variable(s, _)) => Some(s),
                                                _ => None,
                                            }
                                        {
                                            types.push((detail.key.clone(), val));
                                        }
                                    }
                                    // Sort by the numeric suffix of container-typeN
                                    types.sort_by_key(|(k, _)| {
                                        k.strip_prefix("container-type")
                                            .and_then(|s| s.parse::<usize>().ok())
                                            .unwrap_or(0)
                                    });

                                    let type_values: Vec<String> =
                                        types.into_iter().map(|(_, v)| v).collect();
                                    if type_values.len() == 1 {
                                        array_element =
                                            Some(ArrayElementType::Array(type_values[0].clone()));
                                    } else if type_values.len() > 1 {
                                        array_element = Some(ArrayElementType::Tuple(type_values));
                                    }
                                    continue;
                                } else if rule_kv.key.eq_ignore_ascii_case("validation") {
                                    validation = parse_validation(&rule_details);
                                    continue;
                                } else if rule_kv.key.eq_ignore_ascii_case("inherit") {
                                    for detail in rule_details {
                                        inherit.push(detail.key);
                                    }
                                    continue;
                                } else if rule_kv.key.eq_ignore_ascii_case("subpossibilities") {
                                    for sub_kv in rule_details {
                                        if let Some(Value::Container(sub_details, _, _)) =
                                            sub_kv.value
                                        {
                                            subpossibilities
                                                .push(parse_rule(sub_kv.key, sub_details));
                                        }
                                    }
                                    continue;
                                } else if rule_kv.key.eq_ignore_ascii_case("tagarray")
                                    || rule_kv.key.eq_ignore_ascii_case("tag-array")
                                {
                                    tag_array = Some(parse_rule(kv.key.clone(), rule_details));
                                    continue;
                                } else {
                                    rules.push(parse_rule(rule_kv.key.clone(), rule_details));
                                }
                            }
                            Some(Value::String(s, _)) | Some(Value::Variable(s, _)) => {
                                // Handle top-level keys in container definition
                                let key_lower = rule_kv.key.to_lowercase();
                                match key_lower.as_str() {
                                    "kind" => { /* handle container kind if needed */ }
                                    "top-level" => {
                                        top_level = s == "1" || s == "true" || s == "container";
                                    }
                                    "inherit" => {
                                        inherit.push(s);
                                    }
                                    "subpossibilities" | "array-element" | "tagarray"
                                    | "uniquenames" | "allow-any-key" => {
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
                                    "top-level" => top_level = parse_numeric_as_bool(&n),
                                    "allow-any-key" => allow_any_key = parse_numeric_as_bool(&n),
                                    "compulsory" => {
                                        // parsed as a rule at container level? usually it's inside rule_details
                                    }
                                    _ => {}
                                }
                            }
                            Some(Value::Array(v, _)) => {
                                if rule_kv.key.to_lowercase().as_str() == "top-level"
                                    && let Some(numeric_value) = v.first()
                                {
                                    top_level = parse_numeric_as_bool(numeric_value);
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
                        sub_possibilities: subpossibilities,
                        tag_array,
                        allow_any_key,
                    });
                } else {
                    warn!("Did not parse validator {:?}", kv)
                }
            }
        } else {
            simple_validators.insert(filename.to_string(), parse_simple_validator(validator_soup));
        }

        Some((simple_validators, container_validators))
    } else {
        None
    }
}

pub fn load_validators(validation_path: &Path) -> Validators {
    let mut simple_validators: SimpleValidators = HashMap::new();
    let mut container_validators: ContainerValidators = vec![];

    if !validation_path.exists() || !validation_path.is_dir() {
        return Validators {
            simple: simple_validators,
            containers: container_validators,
            container_map: HashMap::new(),
        };
    }

    let entries = match fs::read_dir(validation_path) {
        Ok(entries) => entries,
        Err(e) => {
            error!("Failed to read validation directory: {}", e);
            return Validators {
                simple: simple_validators,
                containers: container_validators,
                container_map: HashMap::new(),
            };
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            let filename = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if filename.is_empty() {
                continue;
            }

            if let Ok(validator_content) = fs::read_to_string(&path)
                && let Some((parsed_simple_validators, parsed_container_validators)) =
                    process_file(filename, &validator_content)
            {
                simple_validators.extend(parsed_simple_validators);
                container_validators.extend(parsed_container_validators);
            }
        }
    }

    let trainz_build = include_str!("custom-validators/trainz-build.txt");
    if let Some((parsed_simple_validators, parsed_container_validators)) =
        process_file("trainz-build", trainz_build)
    {
        simple_validators.extend(parsed_simple_validators);
        container_validators.extend(parsed_container_validators);
    }

    let mut validators = Validators {
        simple: simple_validators,
        containers: container_validators,
        container_map: HashMap::new(),
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
                        for sub in &parent.sub_possibilities {
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
            for sub in &container.sub_possibilities {
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
            container.sub_possibilities = merged_subpossibilities;
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
                    for sub in &parent.sub_possibilities {
                        if !container_to_update
                            .sub_possibilities
                            .iter()
                            .any(|r| r.key.eq_ignore_ascii_case(&sub.key))
                        {
                            container_to_update.sub_possibilities.push(sub.clone());
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

    // Populate container_map after inheritance merging
    validators.container_map = validators
        .containers
        .iter()
        .map(|c| (c.container_name.to_ascii_lowercase(), c.clone()))
        .collect();

    debug!("Loaded {} simple validators", validators.simple.len());
    debug!(
        "Loaded {} container validators",
        validators.containers.len()
    );

    validators
}
