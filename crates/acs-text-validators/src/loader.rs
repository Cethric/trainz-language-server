use rayon::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tracing::{debug, error, trace, warn};
use trainz_ast::acs_text::process::process_acs_text_ast;
use trainz_ast::acs_text::value::{NumericValue, Value};
use trainz_ast::acs_text::{AcsText, KeyValuePair};
use trainz_parser::acs_text::parse_acs_text;

use crate::Validation;
use crate::models::{ArrayElementType, ContainerRule, ContainerValidator, Validators};

#[tracing::instrument(skip(key, rule_details))]
fn parse_rule(
    key: String,
    rule_details: Vec<KeyValuePair>,
    default_top_level: bool,
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
    let mut obsolete_version = None;
    let mut obsolete_message = None;
    let mut minimum_version = None;
    let mut source = None;
    let mut sub_rules = vec![];
    let mut tag_array = None;
    let mut array_element = None;
    let mut sub_possibilities = vec![];
    let mut inherit = vec![];
    let mut top_level = default_top_level;
    let mut allow_any_key = false;
    let mut container_validation = None;

    for detail in rule_details {
        let key_lower = detail.key.to_lowercase();
        match key_lower.as_str() {
            "type" | "element-type" if !matches!(detail.value, Some(Value::Container(_, _, _))) => {
                type_name = match detail.value {
                    Some(Value::String(s, _)) => Some(s),
                    Some(Value::Variable(s, _)) => Some(s),
                    _ => None,
                }
            }
            "kind" if !matches!(detail.value, Some(Value::Container(_, _, _))) => {
                kind = match detail.value {
                    Some(Value::String(s, _)) => Some(s),
                    Some(Value::Variable(s, _)) => Some(s),
                    _ => None,
                }
            }
            "default" if !matches!(detail.value, Some(Value::Container(_, _, _))) => {
                default_value = match detail.value {
                    Some(Value::String(s, _)) => Some(s),
                    Some(Value::Variable(s, _)) => Some(s),
                    _ => None,
                }
            }
            "description" if !matches!(detail.value, Some(Value::Container(_, _, _))) => {
                description = match detail.value {
                    Some(Value::String(s, _)) => Some(s),
                    Some(Value::Variable(s, _)) => Some(s),
                    _ => None,
                }
            }
            "validation" => {
                if let Some(Value::Container(details, _, _)) = &detail.value {
                    rule_validation = parse_validation(details);
                    container_validation = rule_validation.clone();
                } else if let Some(Value::String(s, _)) | Some(Value::Variable(s, _)) =
                    &detail.value
                {
                    rule_validation = Some(vec![Validation::Named(s.clone())]);
                    container_validation = rule_validation.clone();
                } else {
                    warn!("Invalid validation value: {:?}", detail.value);
                }
            }
            "compulsory" if !matches!(detail.value, Some(Value::Container(_, _, _))) => {
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
            "filter" if !matches!(detail.value, Some(Value::Container(_, _, _))) => {
                filter = match detail.value {
                    Some(Value::String(s, _)) => Some(s),
                    _ => None,
                }
            }
            "disabled" if !matches!(detail.value, Some(Value::Container(_, _, _))) => {
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
            "obsolete-tag" if !matches!(detail.value, Some(Value::Container(_, _, _))) => {
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
            "obsolete-version" if !matches!(detail.value, Some(Value::Container(_, _, _))) => {
                obsolete_version = match detail.value {
                    Some(Value::Numeric(NumericValue::Float(n), _)) => Some(n),
                    Some(Value::Numeric(NumericValue::Int(n), _)) => Some(n as f64),
                    Some(Value::String(s, _)) => s.parse().ok(),
                    _ => None,
                }
            }
            "obsolete-message" if !matches!(detail.value, Some(Value::Container(_, _, _))) => {
                obsolete_message = match detail.value {
                    Some(Value::String(s, _)) => Some(s),
                    _ => None,
                }
            }
            "minimum-version" if !matches!(detail.value, Some(Value::Container(_, _, _))) => {
                minimum_version = match detail.value {
                    Some(Value::Numeric(NumericValue::Float(n), _)) => Some(n),
                    Some(Value::Numeric(NumericValue::Int(n), _)) => Some(n as f64),
                    Some(Value::String(s, _)) => s.parse().ok(),
                    _ => None,
                }
            }
            "source" if !matches!(detail.value, Some(Value::Container(_, _, _))) => {
                source = match detail.value {
                    Some(Value::String(s, _)) => Some(s),
                    _ => None,
                }
            }
            "inherit" => match detail.value {
                Some(Value::Container(details, _, _)) => {
                    for d in details {
                        inherit.push(d.key);
                    }
                }
                Some(Value::String(s, _)) | Some(Value::Variable(s, _)) => {
                    inherit.push(s);
                }
                _ => {}
            },
            "top-level" if !matches!(detail.value, Some(Value::Container(_, _, _))) => {
                top_level = match detail.value {
                    Some(Value::String(s, _)) | Some(Value::Variable(s, _)) => {
                        s == "1" || s == "true" || s == "container"
                    }
                    Some(Value::Numeric(n, _)) => parse_numeric_as_bool(&n),
                    _ => false,
                };
            }
            "allow-any-key" if !matches!(detail.value, Some(Value::Container(_, _, _))) => {
                allow_any_key = match detail.value {
                    Some(Value::Numeric(n, _)) => parse_numeric_as_bool(&n),
                    _ => false,
                };
            }
            "subpossibilities" => {
                if let Some(Value::Container(details, _, _)) = detail.value {
                    for sub_kv in details {
                        if let Some(Value::Container(sub_details, _, _)) = sub_kv.value {
                            sub_possibilities.push(parse_rule(sub_kv.key, sub_details, false));
                        }
                    }
                }
            }
            _ => match detail.value {
                Some(Value::Container(details, _, _)) => {
                    if key_lower == "tagarray" || key_lower == "tag-array" {
                        tag_array = parse_array_element(detail.key, details);
                    } else if key_lower == "array-element" {
                        array_element = parse_array_element(detail.key, details);
                    } else if key_lower == "validation" {
                        container_validation = parse_validation(&details);
                    } else if key_lower == "obsolete-tag" {
                        for sub_kv in details {
                            if let Some(Value::Container(sub_details, _, _)) = sub_kv.value {
                                let mut rule = parse_rule(sub_kv.key, sub_details, false);
                                rule.obsolete_tag = Some(true);
                                sub_possibilities.push(rule);
                            }
                        }
                    } else {
                        sub_rules.push(parse_rule(detail.key, details, false));
                    }
                }
                Some(Value::String(s, _)) | Some(Value::Variable(s, _)) => {
                    if key_lower == "tagarray" || key_lower == "tag-array" {
                        tag_array = Some(ArrayElementType::Array(detail.key, s));
                    } else if key_lower == "array-element" {
                        array_element = Some(ArrayElementType::Array(detail.key, s));
                    } else if key_lower != "uniquenames" {
                        sub_rules.push(ContainerRule {
                            key: detail.key,
                            type_name: Some(s),
                            ..Default::default()
                        });
                    }
                }
                _ => {}
            },
        }
    }

    let child_validator = if !sub_rules.is_empty()
        || tag_array.is_some()
        || array_element.is_some()
        || !sub_possibilities.is_empty()
        || !inherit.is_empty()
        || top_level
        || allow_any_key
        || container_validation.is_some()
    {
        Some(Box::new(ContainerValidator {
            container_name: key.clone(),
            rules: sub_rules,
            tag_array,
            array_element,
            sub_possibilities,
            inherit,
            top_level,
            allow_any_key,
            validation: container_validation,
        }))
    } else {
        None
    };

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
        obsolete_version,
        obsolete_message,
        minimum_version,
        source,
        child_validator,
    }
}

#[tracing::instrument(skip(validator_acs_text))]
fn parse_simple_validator(validator_acs_text: AcsText) -> HashMap<String, Option<String>> {
    let mut allowed_values: HashMap<String, Option<String>> = HashMap::new();
    for kv in validator_acs_text.key_value_pairs {
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

#[tracing::instrument(skip(numeric_value))]
fn parse_numeric_as_bool(numeric_value: &NumericValue) -> bool {
    match numeric_value {
        NumericValue::Float(v) => *v > 0f64,
        NumericValue::Hex(v) => *v > 0,
        NumericValue::Int(v) => *v > 0,
    }
}

fn parse_array_element(key: String, rule_details: Vec<KeyValuePair>) -> Option<ArrayElementType> {
    let mut types = Vec::new();
    let mut element_rules = Vec::new();

    // Check if it's a "simple" array/tagarray that just contains properties like type, kind, etc.
    let is_simple_rule = rule_details.iter().any(|d| {
        let k = d.key.to_lowercase();
        k == "type" || k == "kind" || k == "description" || k == "validation"
    }) && !rule_details
        .iter()
        .any(|d| d.key.to_lowercase().starts_with("container-type"));

    if is_simple_rule {
        return Some(ArrayElementType::Rule(Box::new(parse_rule(
            key,
            rule_details,
            false,
        ))));
    }

    for d in rule_details {
        let key_lower = d.key.to_lowercase();
        match d.value {
            Some(Value::String(s, _)) | Some(Value::Variable(s, _))
                if key_lower.starts_with("container-type") =>
            {
                types.push((d.key, s));
            }
            Some(Value::Container(sub_details, _, _)) => {
                element_rules.push(parse_rule(d.key, sub_details, false));
            }
            _ => {}
        }
    }

    if !element_rules.is_empty() {
        Some(ArrayElementType::Inline(Box::new(ContainerValidator {
            container_name: key,
            rules: element_rules,
            ..Default::default()
        })))
    } else if !types.is_empty() {
        // Sort by the numeric suffix of container-typeN
        types.sort_by_key(|(k, _)| {
            k.strip_prefix("container-type")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0)
        });

        if types.len() == 1 {
            let (k, v) = types.into_iter().next().unwrap();
            Some(ArrayElementType::Array(k, v))
        } else {
            Some(ArrayElementType::Tuple(types))
        }
    } else {
        None
    }
}

#[tracing::instrument(skip(validators))]
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
        } else if validator.key.eq_ignore_ascii_case("MustBePaired") {
            if let Some(Value::Container(details, _, _)) = &validator.value {
                let mut paired = vec![];
                for d in details {
                    paired.push(d.key.clone());
                }
                validations.push(Validation::MustBePaired(paired));
            }
            continue;
        } else if validator
            .key
            .eq_ignore_ascii_case("FilepathTableFilesExist")
        {
            validations.push(Validation::FilepathTableFilesExist);
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

#[tracing::instrument(skip(filename, validator_content))]
fn process_file(
    filename: &str,
    validator_content: &str,
) -> Option<(SimpleValidators, ContainerValidators)> {
    let is_kind_file = filename.eq_ignore_ascii_case("kind");
    if let Ok(pairs) = parse_acs_text(validator_content) {
        let validator_acs_text = process_acs_text_ast(pairs, validator_content);

        // Look for existing container name or create new
        let is_container_style = validator_acs_text
            .key_value_pairs
            .par_iter()
            .any(|kv| matches!(kv.value, Some(Value::Container(_, _, _))));

        let mut simple_validators: SimpleValidators = HashMap::new();
        let mut container_validators: ContainerValidators = vec![];

        if is_container_style {
            for kv in &validator_acs_text.key_value_pairs {
                if let Some(Value::Container(container_kv, _, _)) = &kv.value {
                    let rule = parse_rule(kv.key.clone(), container_kv.clone(), is_kind_file);
                    if let Some(mut validator) = rule.child_validator {
                        validator.container_name = kv.key.clone();
                        container_validators.push(*validator);
                    } else {
                        container_validators.push(ContainerValidator {
                            container_name: kv.key.clone(),
                            ..Default::default()
                        });
                    }
                }
            }

            // Post-process to handle 'top-level' set via a separate tag
            for kv in &validator_acs_text.key_value_pairs {
                if kv.key.eq_ignore_ascii_case("top-level")
                    && let Some(Value::String(s, _)) | Some(Value::Variable(s, _)) = &kv.value
                {
                    for cv in &mut container_validators {
                        if cv.container_name.eq_ignore_ascii_case(s) {
                            cv.top_level = true;
                        }
                    }
                }
            }
        } else {
            simple_validators.insert(
                filename.to_string(),
                parse_simple_validator(validator_acs_text),
            );
        }

        Some((simple_validators, container_validators))
    } else {
        None
    }
}

#[tracing::instrument(skip(validation_path, extensions_overrides_path))]
pub fn load_validators(
    validation_path: &Path,
    extensions_overrides_path: Option<&Path>,
) -> Validators {
    let mut simple_validators: SimpleValidators = HashMap::new();
    let mut container_validators: ContainerValidators = vec![];

    if !validation_path.exists() || !validation_path.is_dir() {
        return Validators {
            simple: simple_validators,
            containers: container_validators,
            container_map: HashMap::new(),
        };
    }

    let mut load_from_dir = |dir: &Path| {
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(e) => {
                error!("Failed to read validation directory {:?}: {}", dir, e);
                return;
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
                    // Handle overrides for containers: replace if name matches
                    for new_cv in parsed_container_validators {
                        if let Some(pos) = container_validators.iter().position(|cv| {
                            cv.container_name
                                .eq_ignore_ascii_case(&new_cv.container_name)
                        }) {
                            container_validators[pos] = new_cv;
                        } else {
                            container_validators.push(new_cv);
                        }
                    }
                }
            }
        }
    };

    load_from_dir(validation_path);

    if let Some(overrides) = extensions_overrides_path {
        load_from_dir(overrides);
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
                                .par_iter()
                                .any(|r: &ContainerRule| r.key.eq_ignore_ascii_case(&rule.key))
                            {
                                merged_rules.push(rule.clone());
                            }
                        }
                        for sub in &parent.sub_possibilities {
                            if !merged_subpossibilities
                                .par_iter()
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
