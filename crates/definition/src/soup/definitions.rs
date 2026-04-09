use std::path::Path;
use tower_lsp_server::ls_types::{Location, Position, Range, Uri};
use trainz_ast::soup::key_value_pair::KeyValuePair;
use trainz_ast::soup::soup::Soup;
use trainz_ast::soup::Value;
use trainz_soup_validators::{ArrayElementType, ContainerValidator, Validators};

pub fn soup_goto_definition(
    soup: &Soup,
    position: Position,
    uri: Uri,
    validators: &Validators,
    base_path: Option<&Path>,
) -> Option<Vec<Location>> {
    // Search for a key or a filepathedit value at position
    let result = find_definition_recursive(
        &soup.key_value_pairs,
        position,
        validators,
        None,
        base_path,
        &uri,
    );

    if let Some(res) = result {
        match res {
            DefinitionResult::Key(key) => {
                let mut locations = vec![];
                find_all_key_locations(soup, &key, &uri, &mut locations);
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
    current_uri: &Uri,
) -> Option<DefinitionResult> {
    for kv in kvs {
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
                    rule = cv
                        .rules
                        .iter()
                        .find(|r| r.key.eq_ignore_ascii_case(&kv.key));
                    if let Some(r) = rule {
                        if let Some(type_name) = &r.type_name {
                            next_validator = validators
                                .containers
                                .iter()
                                .find(|v| v.container_name.eq_ignore_ascii_case(type_name));
                        }
                    }

                    if next_validator.is_none() {
                        if let Some(array_element_type) = &cv.array_element {
                            let type_name = match array_element_type {
                                ArrayElementType::Array(s) => Some(s),
                                ArrayElementType::Tuple(types) => {
                                    kv.key.parse::<usize>().ok().and_then(|idx| types.get(idx))
                                }
                            };
                            next_validator = type_name.and_then(|tn| {
                                validators
                                    .containers
                                    .iter()
                                    .find(|v| v.container_name.eq_ignore_ascii_case(tn))
                            });
                        }
                    }
                } else {
                    // Top-level
                    next_validator = validators
                        .containers
                        .iter()
                        .find(|cv| cv.container_name.eq_ignore_ascii_case(&kv.key));

                    // If not a container, check simple validators
                    if next_validator.is_none() {
                        if let Some(_) = validators.simple.get(&kv.key) {
                            rule = validators
                                .containers
                                .iter()
                                .find(|c| c.container_name.eq_ignore_ascii_case(&kv.key))
                                .and_then(|container| {
                                    container
                                        .rules
                                        .iter()
                                        .find(|r| r.key.eq_ignore_ascii_case(&kv.key))
                                });
                        }
                    }
                }

                // Check for filepathedit
                if let Some(r) = rule {
                    if let Some(type_name_str) = &r.type_name {
                        if type_name_str.eq_ignore_ascii_case("filepathedit") {
                            if let Value::String(path_str, _) = value {
                                if let Some(base) = base_path {
                                    let full_path = base.join(path_str);
                                    if full_path.exists() {
                                        return Some(DefinitionResult::Location(Location {
                                            uri: Uri::from_file_path(
                                                full_path.canonicalize().unwrap(),
                                            )
                                            .unwrap(),
                                            range: Range::default(),
                                        }));
                                    }
                                }
                            }
                        }
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
                        current_uri,
                    );
                }
            }
        }
    }

    None
}

fn is_in_range(pos: Position, range: &trainz_ast::Range) -> bool {
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

fn get_value_range(value: &Value) -> trainz_ast::Range {
    value.range()
}

fn find_all_key_locations(soup: &Soup, target: &str, uri: &Uri, locations: &mut Vec<Location>) {
    for kv in &soup.key_value_pairs {
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
    use trainz_ast::soup::process::process_soup_ast;
    use trainz_parser::soup::parse_soup;
    use trainz_soup_validators::{ContainerRule, ContainerValidator, Validators};

    #[test]
    fn test_soup_goto_definition_key() {
        let src = "test\n{\n  key1 \"value\"\n  key1 \"value2\"\n}";
        let pairs = parse_soup(src).unwrap();
        let soup = process_soup_ast(pairs, src);
        let uri = Uri::from_file_path("/test.soup").unwrap();
        let validators = Validators::default();

        let pos = Position {
            line: 2,
            character: 3,
        }; // "key1"
        let locs = soup_goto_definition(&soup, pos, uri.clone(), &validators, None);
        assert!(locs.is_some());
        assert_eq!(locs.unwrap().len(), 2);
    }

    #[test]
    fn test_soup_goto_definition_filepathedit() {
        let dir = tempdir().unwrap();
        let soup_file_path = dir.path().join("test.soup");
        let target_file_path = dir.path().join("target.txt");
        File::create(&target_file_path)
            .unwrap()
            .write_all(b"hello")
            .unwrap();

        let src = "test\n{\n  config \"target.txt\"\n}";
        let pairs = parse_soup(src).unwrap();
        let soup = process_soup_ast(pairs, src);
        let uri = Uri::from_file_path(&soup_file_path).unwrap();

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
                array_element: None,
            }],
            ..Default::default()
        });

        let pos = Position {
            line: 2,
            character: 10,
        }; // within "target.txt"
        let locs =
            soup_goto_definition(&soup, pos, uri.clone(), &validators, Some(dir.path())).unwrap();

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
}
