use std::path::Path;
use std::sync::Arc;
use tower_lsp_server::ls_types::{Location, Position, Range, Uri};
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
    base_path: Option<&Path>,
    script_resolver: Option<&dyn ScriptResolver>,
) -> Option<Vec<Location>> {
    // Search for a key or a filepathedit value at position
    let result = find_definition_recursive(
        &acs_text.key_value_pairs,
        position,
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
    base_path: Option<&Path>,
    script_resolver: Option<&dyn ScriptResolver>,
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
