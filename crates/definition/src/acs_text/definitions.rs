use rayon::prelude::*;
use std::path::Path;
use std::sync::Arc;
use tower_lsp_server::ls_types::{Location, Position, Range, Uri};
use trainz_acs_text_validators::{ArrayElementType, ContainerValidator, Validation, Validators};
use trainz_ast::acs_text::Value;
use trainz_ast::acs_text::base::AcsText;
use trainz_ast::acs_text::key_value_pair::KeyValuePair;
use trainz_ast::gs::Program;

pub trait ScriptResolver {
    fn resolve_script(&self, base_path: &Path, script_name: &str) -> Option<(Uri, Arc<Program>)>;
}

#[tracing::instrument(skip(script_resolver))]
pub fn acs_text_goto_definition(
    acs_text: &AcsText,
    position: Position,
    uri: Uri,
    validators: &Validators,
    base_path: Option<&Path>,
    script_resolver: Option<&dyn ScriptResolver>,
) -> Option<Vec<Location>> {
    // Search for a key or a filepathedit value at position
    let result = find_definition_recursive(
        &acs_text.key_value_pairs,
        position,
        validators,
        None,
        base_path,
        script_resolver,
    );

    if let Some(res) = result {
        match res {
            DefinitionResult::Key(key) => {
                let mut locations = vec![];
                find_all_key_locations(acs_text, &key, &uri, &mut locations);
                if !locations.is_empty() {
                    return Some(locations);
                }
            }
            DefinitionResult::Location(loc) => return Some(vec![loc]),
        }
    }

    None
}

enum DefinitionResult {
    Key(String),
    Location(Location),
}

fn find_definition_recursive(
    kvs: &[KeyValuePair],
    position: Position,
    validators: &Validators,
    current_validator: Option<&ContainerValidator>,
    base_path: Option<&Path>,
    script_resolver: Option<&dyn ScriptResolver>,
) -> Option<DefinitionResult> {
    for (index, kv) in kvs.iter().enumerate() {
        // Check if cursor is over key
        if is_in_range(position, &kv.key_range) {
            return Some(DefinitionResult::Key(kv.key.clone()));
        }

        // Check if cursor is over value
        if let Some(value) = &kv.value {
            let value_range = get_value_range(value);
            if is_in_range(position, &value_range) {
                // Determine the rule for this key
                let mut rule = None;
                let mut next_validator = None;

                if let Some(cv) = current_validator {
                    if let Some(array_element_type) = &cv.array_element {
                        match array_element_type {
                            ArrayElementType::Array(_, s) => {
                                next_validator = validators.container_map.get(&s.to_lowercase());
                            }
                            ArrayElementType::Tuple(types) => {
                                next_validator = kv.key.parse::<usize>().ok().and_then(|idx| {
                                    types.get(idx).and_then(|(_, tn)| {
                                        validators.container_map.get(&tn.to_lowercase())
                                    })
                                });
                            }
                            ArrayElementType::Inline(iv) => {
                                next_validator = Some(iv.as_ref());
                            }
                            ArrayElementType::Rule(r) => {
                                rule = Some(r.as_ref());
                                if let Some(cv) = &r.child_validator {
                                    next_validator = Some(cv.as_ref());
                                } else if let Some(tn) = &r.type_name {
                                    next_validator =
                                        validators.container_map.get(&tn.to_lowercase());
                                }
                            }
                        }
                    } else {
                        rule = cv
                            .rules
                            .par_iter()
                            .find_first(|r| r.key.eq_ignore_ascii_case(&kv.key))
                            .or_else(|| {
                                cv.sub_possibilities
                                    .par_iter()
                                    .find_first(|r| r.key.eq_ignore_ascii_case(&kv.key))
                            });
                        if let Some(r) = rule {
                            if let Some(child_validator) = &r.child_validator {
                                next_validator = Some(child_validator.as_ref());
                            } else if let Some(type_name) = &r.type_name {
                                next_validator =
                                    validators.container_map.get(&type_name.to_lowercase());
                            }
                        } else if let Some(tag_array) = &cv.tag_array {
                            match tag_array {
                                ArrayElementType::Array(_, s) => {
                                    next_validator =
                                        validators.container_map.get(&s.to_lowercase());
                                }
                                ArrayElementType::Tuple(types) => {
                                    next_validator = types.get(index).and_then(|(_, tn)| {
                                        validators.container_map.get(&tn.to_lowercase())
                                    });
                                }
                                ArrayElementType::Inline(iv) => {
                                    next_validator = Some(iv.as_ref());
                                }
                                ArrayElementType::Rule(r) => {
                                    rule = Some(r.as_ref());
                                    if let Some(cv) = &r.child_validator {
                                        next_validator = Some(cv.as_ref());
                                    } else if let Some(tn) = &r.type_name {
                                        next_validator =
                                            validators.container_map.get(&tn.to_lowercase());
                                    }
                                }
                            }
                        }
                    }
                } else {
                    // Top-level
                    next_validator = validators.container_map.get(&kv.key.to_lowercase());

                    // If not a container, check simple validators
                    if next_validator.is_none() && validators.simple.contains_key(&kv.key) {
                        rule = validators
                            .container_map
                            .get(&kv.key.to_lowercase())
                            .and_then(|container| {
                                container
                                    .rules
                                    .par_iter()
                                    .find_first(|r| r.key.eq_ignore_ascii_case(&kv.key))
                            });
                    }
                }

                // Check for filepathedit or filepath
                if let Some(r) = rule
                    && let Some(type_name_str) = &r.type_name
                    && (type_name_str.eq_ignore_ascii_case("filepathedit")
                        || type_name_str.eq_ignore_ascii_case("filepath"))
                    && let Value::String(path_str, _) = value
                    && let Some(base) = base_path
                {
                    let full_path = base.join(path_str);
                    if full_path.exists() {
                        return Some(DefinitionResult::Location(Location {
                            uri: Uri::from_file_path(full_path.canonicalize().unwrap()).unwrap(),
                            range: Range::default(),
                        }));
                    }
                }

                // Check for ScriptFileExists validation
                if let Some(r) = rule
                    && let Some(validations) = &r.validation
                    && let Value::String(path_str, _) = value
                    && let Some(base) = base_path
                {
                    for validation in validations {
                        if let Validation::Named(name) = validation
                            && name.eq_ignore_ascii_case("ScriptFileExists")
                        {
                            let mut full_path = base.join(path_str);
                            if !full_path.exists() && !path_str.to_lowercase().ends_with(".gs") {
                                full_path.set_extension("gs");
                            }

                            if full_path.exists() {
                                return Some(DefinitionResult::Location(Location {
                                    uri: Uri::from_file_path(full_path.canonicalize().unwrap())
                                        .unwrap(),
                                    range: Range::default(),
                                }));
                            }
                        }
                    }
                }

                // Check for class tag
                if kv.key.eq_ignore_ascii_case("class")
                    && let Value::String(class_name, _) = value
                    && let Some(base) = base_path
                    && let Some(resolver) = script_resolver
                {
                    // Find script tag in the same container
                    let script_name = kvs.iter().find_map(|kv| {
                        if kv.key.eq_ignore_ascii_case("script")
                            && let Some(Value::String(s, _)) = &kv.value
                        {
                            Some(s.as_str())
                        } else {
                            None
                        }
                    });

                    if let Some(script_name) = script_name
                        && let Some((_uri, program)) = resolver.resolve_script(base, script_name)
                        && let Some(class_def) = program.classes.get(class_name)
                    {
                        return Some(DefinitionResult::Location(Location {
                            uri: Uri::from_file_path(
                                std::path::Path::new(&program.src)
                                    .canonicalize()
                                    .unwrap_or_else(|_e| std::path::PathBuf::from(&program.src)),
                            )
                            .unwrap_or({
                                // Fallback to _uri if program.src is not a valid path
                                _uri
                            }),
                            range: class_def.name.range,
                        }));
                    }
                }

                // If it's a container, recurse
                if let Value::Container(inner_kvs, _, _) = value {
                    return find_definition_recursive(
                        inner_kvs,
                        position,
                        validators,
                        next_validator,
                        base_path,
                        script_resolver,
                    );
                }
            }
        }
    }

    None
}

#[tracing::instrument(skip(pos, range))]
fn is_in_range(pos: Position, range: &Range) -> bool {
    if pos.line < range.start.line || pos.line > range.end.line {
        return false;
    }
    if pos.line == range.start.line && pos.character < range.start.character {
        return false;
    }
    if pos.line == range.end.line && pos.character > range.end.character {
        return false;
    }
    true
}

#[tracing::instrument(skip(value))]
fn get_value_range(value: &Value) -> Range {
    value.range()
}

#[tracing::instrument(skip(acs_text, target, uri, locations))]
fn find_all_key_locations(
    acs_text: &AcsText,
    target: &str,
    uri: &Uri,
    locations: &mut Vec<Location>,
) {
    for kv in &acs_text.key_value_pairs {
        if kv.key == target {
            locations.push(Location {
                uri: uri.clone(),
                range: kv.key_range,
            });
        }
        if let Some(Value::Container(kv_pairs, _, _)) = &kv.value {
            find_all_key_locations_inner(kv_pairs, target, uri, locations);
        }
    }
}

#[tracing::instrument(skip(kv_pairs, target, uri, locations))]
fn find_all_key_locations_inner(
    kv_pairs: &Vec<KeyValuePair>,
    target: &str,
    uri: &Uri,
    locations: &mut Vec<Location>,
) {
    for kv in kv_pairs {
        if kv.key == target {
            locations.push(Location {
                uri: uri.clone(),
                range: kv.key_range,
            });
        }
        if let Some(Value::Container(inner_kv_pairs, _, _)) = &kv.value {
            find_all_key_locations_inner(inner_kv_pairs, target, uri, locations);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;
    use trainz_acs_text_validators::{ContainerRule, ContainerValidator, Validators};
    use trainz_ast::acs_text::process::process_acs_text_ast;
    use trainz_parser::acs_text::parse_acs_text;

    #[test]
    fn test_acs_text_goto_definition_key() {
        let src = "test\n{\n  key1 \"value\"\n  key1 \"value2\"\n}";
        let pairs = parse_acs_text(src).unwrap();
        let acs_text = process_acs_text_ast(pairs, src);
        let uri = Uri::from_file_path("/test.acs_text").unwrap();
        let validators = Validators::default();

        let pos = Position {
            line: 2,
            character: 3,
        }; // "key1"
        let locs = acs_text_goto_definition(&acs_text, pos, uri.clone(), &validators, None, None);
        assert!(locs.is_some());
        assert_eq!(locs.unwrap().len(), 2);
    }

    #[test]
    fn test_acs_text_goto_definition_filepathedit() {
        let dir = tempdir().unwrap();
        let acs_text_file_path = dir.path().join("test.acs_text");
        let target_file_path = dir.path().join("target.txt");
        File::create(&target_file_path)
            .unwrap()
            .write_all(b"hello")
            .unwrap();

        let src = "test\n{\n  config \"target.txt\"\n}";
        let pairs = parse_acs_text(src).unwrap();
        let acs_text = process_acs_text_ast(pairs, src);
        let uri = Uri::from_file_path(&acs_text_file_path).unwrap();

        let mut validators = Validators::default();
        validators.containers.push(ContainerValidator {
            container_name: "test".to_string(),
            rules: vec![ContainerRule {
                key: "config".to_string(),
                type_name: Some("filepathedit".to_string()),
                kind: None,
                default_value: None,
                description: None,
                validation: None,
                compulsory: None,
                filter: None,
                disabled: None,
                obsolete_tag: None,
                obsolete_version: None,
                obsolete_message: None,
                minimum_version: None,
                source: None,
                child_validator: None,
            }],
            ..Default::default()
        });
        for v in &validators.containers {
            validators
                .container_map
                .insert(v.container_name.to_lowercase(), v.clone());
        }

        let pos = Position {
            line: 2,
            character: 10,
        }; // within "target.txt"
        let locs = acs_text_goto_definition(
            &acs_text,
            pos,
            uri.clone(),
            &validators,
            Some(dir.path()),
            None,
        )
        .unwrap();

        assert_eq!(locs.len(), 1);
        let expected_path = target_file_path
            .canonicalize()
            .unwrap()
            .to_string_lossy()
            .to_lowercase();
        let actual_path = locs[0]
            .uri
            .to_file_path()
            .unwrap()
            .canonicalize()
            .unwrap()
            .to_string_lossy()
            .to_lowercase();
        assert_eq!(actual_path, expected_path);
    }

    #[test]
    fn test_acs_text_goto_definition_filepath_and_script() {
        let dir = tempdir().unwrap();
        let acs_text_file_path = dir.path().join("test.acs_text");
        let target_file_path = dir.path().join("target.txt");
        File::create(&target_file_path)
            .unwrap()
            .write_all(b"hello")
            .unwrap();

        let script_file_path = dir.path().join("myscript.gs");
        File::create(&script_file_path)
            .unwrap()
            .write_all(b"class MyClass { int x; };")
            .unwrap();

        let src = r#"test
{
  file "target.txt"
  script "myscript"
  class "MyClass"
}"#;
        let pairs = parse_acs_text(src).unwrap();
        let acs_text = process_acs_text_ast(pairs, src);
        let uri = Uri::from_file_path(&acs_text_file_path).unwrap();

        let mut validators = Validators::default();
        validators.containers.push(ContainerValidator {
            container_name: "test".to_string(),
            rules: vec![
                ContainerRule {
                    key: "file".to_string(),
                    type_name: Some("filepath".to_string()),
                    ..Default::default()
                },
                ContainerRule {
                    key: "script".to_string(),
                    type_name: Some("string".to_string()),
                    validation: Some(vec![Validation::Named("ScriptFileExists".to_string())]),
                    ..Default::default()
                },
                ContainerRule {
                    key: "class".to_string(),
                    type_name: Some("string".to_string()),
                    ..Default::default()
                },
            ],
            ..Default::default()
        });
        for v in &validators.containers {
            validators
                .container_map
                .insert(v.container_name.to_lowercase(), v.clone());
        }

        // 1. Test filepath
        let pos_file = Position {
            line: 2,
            character: 10,
        };
        let locs_file = acs_text_goto_definition(
            &acs_text,
            pos_file,
            uri.clone(),
            &validators,
            Some(dir.path()),
            None,
        )
        .unwrap();
        assert_eq!(locs_file.len(), 1);
        assert_eq!(
            locs_file[0]
                .uri
                .to_file_path()
                .unwrap()
                .canonicalize()
                .unwrap(),
            target_file_path.canonicalize().unwrap()
        );

        // 2. Test ScriptFileExists
        let pos_script = Position {
            line: 3,
            character: 10,
        };
        let locs_script = acs_text_goto_definition(
            &acs_text,
            pos_script,
            uri.clone(),
            &validators,
            Some(dir.path()),
            None,
        )
        .unwrap();
        assert_eq!(locs_script.len(), 1);
        assert_eq!(
            locs_script[0]
                .uri
                .to_file_path()
                .unwrap()
                .canonicalize()
                .unwrap(),
            script_file_path.canonicalize().unwrap()
        );

        // 3. Test class Goto
        struct MockResolver {
            script_uri: Uri,
            program: Arc<Program>,
        }
        impl ScriptResolver for MockResolver {
            fn resolve_script(
                &self,
                _base_path: &Path,
                script_name: &str,
            ) -> Option<(Uri, Arc<Program>)> {
                if script_name == "myscript" {
                    Some((self.script_uri.clone(), self.program.clone()))
                } else {
                    None
                }
            }
        }

        let script_src = "class MyClass { int x; };";
        let script_pairs = trainz_parser::gs::parse(script_src).unwrap();
        let program = Arc::new(trainz_ast::gs::process::process_trainz_ast(
            script_pairs,
            script_src,
        ));
        let mut program_with_src = (*program).clone();
        program_with_src.src = script_file_path.to_string_lossy().to_string();
        let program = Arc::new(program_with_src);

        let resolver = MockResolver {
            script_uri: Uri::from_file_path(&script_file_path).unwrap(),
            program,
        };

        let pos_class = Position {
            line: 4,
            character: 10,
        };
        let locs_class = acs_text_goto_definition(
            &acs_text,
            pos_class,
            uri.clone(),
            &validators,
            Some(dir.path()),
            Some(&resolver),
        )
        .unwrap();
        assert_eq!(locs_class.len(), 1);
        assert_eq!(
            locs_class[0]
                .uri
                .to_file_path()
                .unwrap()
                .canonicalize()
                .unwrap(),
            script_file_path.canonicalize().unwrap()
        );
        // The range should be the range of "MyClass" identifier in the script
        assert_eq!(locs_class[0].range.start.line, 0);
    }

    #[test]
    fn test_inline_nested_validator_goto_definition() {
        let content = r#"
example {
  nested {
    target_file "test.txt"
  }
}
"#;
        let pairs = trainz_parser::acs_text::parse_acs_text(content).unwrap();
        let acs_text = trainz_ast::acs_text::process::process_acs_text_ast(pairs, content);

        let dir = tempfile::tempdir().unwrap();
        let target_file_path = dir.path().join("test.txt");
        std::fs::write(&target_file_path, "test").unwrap();

        let mut validators = Validators::default();
        validators.containers.push(ContainerValidator {
            container_name: "example".to_string(),
            rules: vec![ContainerRule {
                key: "nested".to_string(),
                child_validator: Some(Box::new(ContainerValidator {
                    container_name: "nested".to_string(),
                    rules: vec![ContainerRule {
                        key: "target_file".to_string(),
                        type_name: Some("filepath".to_string()),
                        ..Default::default()
                    }],
                    ..Default::default()
                })),
                ..Default::default()
            }],
            ..Default::default()
        });
        for v in &validators.containers {
            validators
                .container_map
                .insert(v.container_name.to_lowercase(), v.clone());
        }

        let pos = Position {
            line: 3,
            character: 18, // Inside "test.txt"
        };
        let uri = Uri::from_file_path("/test.acs_text").unwrap();
        let locs =
            acs_text_goto_definition(&acs_text, pos, uri, &validators, Some(dir.path()), None)
                .unwrap();

        assert_eq!(
            locs.len(),
            1,
            "Should find 1 location for inline nested goto"
        );
        assert_eq!(
            locs[0].uri.to_file_path().unwrap().canonicalize().unwrap(),
            target_file_path.canonicalize().unwrap()
        );
    }
}
