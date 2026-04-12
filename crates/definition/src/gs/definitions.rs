use dashmap::DashMap;
use rayon::prelude::*;
use std::sync::Arc;
use tower_lsp_server::ls_types::{GotoDefinitionResponse, Location, LocationLink, Position, Uri};
use tracing::trace;
use trainz_ast::find::position_in_range;
use trainz_ast::gs::find::{find_id_at_position, find_postfix_at_position};
use trainz_ast::gs::program::Program;
use trainz_ast::gs::{Expr, MethodDef, PostfixOp, Type};

#[tracing::instrument]
fn parse_uri_or_path(s: &str) -> Option<Uri> {
    if let Ok(uri) = s.parse::<Uri>()
        && uri.scheme().as_str() != ""
    {
        return Some(uri);
    }
    Uri::from_file_path(s)
}

#[tracing::instrument]
fn find_member_type(
    program: &Arc<Program>,
    parsed_files: &DashMap<String, Arc<Program>>,
    start_class: &str,
    member_name: &str,
) -> Option<String> {
    let mut visited_classes = std::collections::HashSet::new();
    let mut to_visit = vec![start_class.to_string()];

    while let Some(cls_name) = to_visit.pop() {
        if !visited_classes.insert(cls_name.clone()) {
            continue;
        }

        let mut class_def = program.classes.get(&cls_name).cloned();

        if class_def.is_none()
            && let Some((cls, _)) = parsed_files.par_iter().find_map_any(|entry| {
                entry
                    .value()
                    .classes
                    .get(&cls_name)
                    .map(|c| (c.clone(), ()))
            })
        {
            class_def = Some(cls);
        }

        if let Some(cls) = class_def {
            // Check fields
            if let Some(f) = cls.fields.get(member_name)
                && let Type::Named(tid) = &f.ty
            {
                return Some(tid.name.clone());
            }

            // Check methods
            if let Some(ms) = cls.methods.get(member_name) {
                for m in ms {
                    if let trainz_ast::gs::types::TypeOrVoid::Type(Type::Named(tid)) =
                        &m.return_type
                    {
                        return Some(tid.name.clone());
                    }
                }
            }

            // If not found, add superclasses
            for super_cls in &cls.superclasses {
                to_visit.push(super_cls.name.clone());
            }
        }
    }
    None
}

fn infer_receiver_type(
    program: &Arc<Program>,
    parsed_files: &DashMap<String, Arc<Program>>,
    expr: &Expr,
    ops: &[PostfixOp],
    current_class_name: Option<&str>,
    _current_method: Option<&MethodDef>,
    position: Position,
) -> Option<String> {
    let mut current_type = match expr {
        Expr::Identifier(id) => {
            let found_local = program
                .find_variable_declaration(&id.name, position)
                .map(|(ty, _)| format!("{}", ty));
            if found_local.is_some() {
                found_local
            } else {
                // It could be a class name (static method access) or a local/field/method returning something.
                // For now, if it matches a class name, assume static class access.
                let mut found_class = program.classes.contains_key(&id.name);

                if !found_class {
                    found_class = parsed_files
                        .par_iter()
                        .any(|entry| entry.value().classes.contains_key(&id.name));
                }
                if found_class {
                    Some(id.name.clone())
                } else {
                    // If it's a method call on 'this' implicit, its type is return type of that method.
                    // We'd have to search methods of current_class_name.
                    if let Some(cname) = current_class_name {
                        find_member_type(program, parsed_files, cname, &id.name)
                    } else {
                        None
                    }
                }
            }
        }
        Expr::NewObject {
            ty: Type::Named(id),
            ..
        } => Some(id.name.clone()),
        _ => None,
    };

    for op in ops {
        match op {
            PostfixOp::Call(_, _) => {
                // Return type of current_type
                // If current_type is a method, this doesn't help without more context, but if we had:
                // `GetAsset()` and current_type evaluated to the return type of `GetAsset`, we are good.
                // Actually, if `expr` was an identifier `GetAsset` and `ops` starts with `Call`, the identifier was a method.
                // Above, we already set `current_type` to the return type of `GetAsset`!
            }
            PostfixOp::Deref(id) => {
                // This gives a field/method. We need to find its return type if followed by Call.
                if let Some(cname) = current_type {
                    current_type = find_member_type(program, parsed_files, &cname, &id.name);
                }
            }
            _ => {}
        }
    }
    current_type
}

pub fn gs_goto_definition(
    program: Arc<Program>,
    position: Position,
    uri: Uri,
    parsed_files: &DashMap<String, Arc<Program>>,
) -> Option<GotoDefinitionResponse> {
    let mut target_identifier = None;
    let mut origin_selection_range = None;

    if let Some(id) = find_id_at_position(&program, position) {
        trace!(
            "gs_goto_definition identified {} with range {:?}",
            id.name, id.range
        );
        target_identifier = Some(id.name.to_string());
        origin_selection_range = Some(id.range);
    } else {
        trace!(
            "gs_goto_definition: find_id_at_position returned None for {:?}",
            position
        );
    }

    if target_identifier.is_none() {
        // Check for includes
        for include in &program.includes {
            trace!(
                "Checking position {:?} in include range {:?} or path_range {:?}",
                position, include.range, include.path_range
            );
            if (position_in_range(position, include.range)
                || include
                    .path_range
                    .is_some_and(|r| position_in_range(position, r)))
                && let Some(path) = &include.path
            {
                let target_uri = Uri::from_file_path(path).unwrap();
                return Some(GotoDefinitionResponse::Link(vec![LocationLink {
                    origin_selection_range: Some(include.range),
                    target_uri,
                    target_range: tower_lsp_server::ls_types::Range::default(),
                    target_selection_range: tower_lsp_server::ls_types::Range::default(),
                }]));
            }
        }
    }

    if let Some(target) = target_identifier {
        trace!("gs_goto_definition searching for {}", target);
        if target == "inherited" || target.starts_with("UNKNOWN_RULE_") {
            // Special handling for inherited()
            let effective_target = if target == "inherited" {
                target.clone()
            } else {
                // Try to see if it's inherited
                if target.contains("keyword_inherited") {
                    "inherited".to_string()
                } else {
                    target.clone()
                }
            };

            if effective_target == "inherited" {
                // Find which class and method the current position is in.
                let mut current_class = None;
                let mut current_method = None;

                for cls in program.classes.values() {
                    if position_in_range(position, cls.range) {
                        current_class = Some(cls.name.name.clone());

                        for methods in cls.methods.values() {
                            for method in methods {
                                if position_in_range(position, method.range) {
                                    current_method = Some(method.name.name.clone());
                                    break;
                                }
                            }
                            if current_method.is_some() {
                                break;
                            }
                        }
                        break;
                    }
                }

                if let (Some(class_name), Some(method_name)) = (current_class, current_method) {
                    trace!(
                        "gs_goto_definition found inherited() in {}::{}",
                        class_name, method_name
                    );

                    // Find the class definition to get superclasses
                    let mut superclasses = vec![];
                    let mut current_file_found = false;

                    // Look in current file first
                    if let Some(cls) = program.classes.get(&class_name) {
                        for super_cls in &cls.superclasses {
                            superclasses.push(super_cls.name.clone());
                        }
                        current_file_found = true;
                    }

                    if !current_file_found {
                        // Search in included files
                        for include in &program.includes {
                            trace!("gs_goto_definition checking include: {:?}", include.path);
                            if let Some(path) = &include.path {
                                let included_uri = Uri::from_file_path(path);
                                if let Some(included_uri) = included_uri {
                                    trace!(
                                        "gs_goto_definition checking included_uri: {:?}",
                                        included_uri
                                    );
                                    if let Some(included_program) =
                                        parsed_files.get(&included_uri.to_string())
                                    {
                                        trace!(
                                            "gs_goto_definition found included_program for {:?}",
                                            included_uri
                                        );
                                        if let Some(cls) = included_program.classes.get(&class_name)
                                        {
                                            trace!(
                                                "gs_goto_definition found class {} in {:?}",
                                                class_name, included_uri
                                            );
                                            for super_cls in &cls.superclasses {
                                                superclasses.push(super_cls.name.clone());
                                            }
                                        }
                                    }
                                }
                            }
                            if !superclasses.is_empty() {
                                break;
                            }
                        }
                    }

                    if !superclasses.is_empty() {
                        let mut locations = vec![];
                        let mut visited_classes = std::collections::HashSet::new();
                        let mut to_visit = superclasses;

                        while let Some(cls_name) = to_visit.pop() {
                            if !visited_classes.insert(cls_name.clone()) {
                                continue;
                            }

                            let mut class_def = None;
                            let mut class_uri = None;

                            // Search in current file
                            if let Some(cls) = program.classes.get(&cls_name) {
                                class_def = Some(cls.clone());
                                class_uri = Some(uri.clone());
                            }

                            if class_def.is_none()
                                && let Some((cls, uri)) =
                                    parsed_files.par_iter().find_map_any(|entry| {
                                        let included_uri_str = entry.key();
                                        let included_program = entry.value();
                                        included_program.classes.get(&cls_name).map(|c| {
                                            (c.clone(), parse_uri_or_path(included_uri_str))
                                        })
                                    })
                            {
                                class_def = Some(cls);
                                class_uri = uri;
                            }

                            if let Some(cls) = class_def {
                                // Search for method_name in this class
                                if let Some(ms) = cls.methods.get(&method_name) {
                                    for method in ms {
                                        // Ensure we're not pointing to the same method from which we started.
                                        // This can happen if a class names itself as its own superclass or through circular includes.
                                        if cls.name.name == class_name
                                            && method.name.name == method_name
                                        {
                                            continue;
                                        }

                                        if let Some(target_uri) = class_uri.clone() {
                                            if target_uri != uri {
                                                return Some(GotoDefinitionResponse::Link(vec![
                                                    LocationLink {
                                                        origin_selection_range,
                                                        target_uri,
                                                        target_range: method.range,
                                                        target_selection_range: method.name.range,
                                                    },
                                                ]));
                                            }

                                            locations.push(Location {
                                                uri: target_uri,
                                                range: method.name.range,
                                            });
                                            return Some(GotoDefinitionResponse::Array(locations));
                                        }
                                    }
                                }

                                // If not found, add superclasses to to_visit
                                for super_cls in &cls.superclasses {
                                    to_visit.push(super_cls.name.clone());
                                }
                            }
                        }
                    }
                }

                return None;
            }
        }

        let mut locations = vec![];

        if let Some((_, name_id)) = program.find_variable_declaration(&target, position) {
            return Some(GotoDefinitionResponse::Scalar(Location {
                uri: uri.clone(),
                range: name_id.range,
            }));
        }

        let mut current_class = None;
        let mut current_method = None;
        for cls in program.classes.values() {
            if position_in_range(position, cls.range) {
                current_class = Some(cls.name.name.clone());
                for methods in cls.methods.values() {
                    for method in methods {
                        if position_in_range(position, method.range) {
                            current_method = Some(method.clone());
                            break;
                        }
                    }
                    if current_method.is_some() {
                        break;
                    }
                }
                break;
            }
        }

        let mut expected_receiver_class = None;
        if let Some((postfix_expr, idx)) = find_postfix_at_position(&program, position)
            && let Expr::Postfix { expr, ops, .. } = postfix_expr
        {
            let ops_before = &ops[..idx];
            expected_receiver_class = infer_receiver_type(
                &program,
                parsed_files,
                expr,
                ops_before,
                current_class.as_deref(),
                current_method.as_ref(),
                position,
            );
        }

        // First, look in the current file.
        // We need to find all occurrences of the target name in the current file and check if they are definitions or references.
        // For simplicity, let's just collect all matching identifiers first.

        trace!("gs_goto_definition searching for {}", target);
        trace!(
            "gs_goto_definition classes in program: {:?}",
            program.classes.keys().collect::<Vec<_>>()
        );
        let mut local_definitions = vec![];

        for cls in program.classes.values() {
            if let Some(expected) = &expected_receiver_class
                && cls.name.name != *expected
            {
                continue;
            }
            // trace!("gs_goto_definition checking class: {}", cls.name.name);
            if expected_receiver_class.is_none() && cls.name.name == target {
                trace!("gs_goto_definition found local class: {}", cls.name.name);
                local_definitions.push(Location {
                    uri: uri.clone(),
                    range: cls.name.range,
                });
            }
            if let Some(field) = cls.fields.get(&target) {
                local_definitions.push(Location {
                    uri: uri.clone(),
                    range: field.name.range,
                });
            }
            if let Some(ms) = cls.methods.get(&target) {
                for method in ms {
                    local_definitions.push(Location {
                        uri: uri.clone(),
                        range: method.name.range,
                    });
                }
            }
        }

        if !local_definitions.is_empty() {
            locations.extend(local_definitions);
        }

        // If not found as a local definition, search other files.
        if locations.is_empty() {
            trace!("gs_goto_definition searching other files for {}", target);

            // 1. Check if it's a class in ANY parsed file.
            let mut found_class = None;
            if expected_receiver_class.is_none() {
                found_class = parsed_files.par_iter().find_map_any(|entry| {
                    let included_uri_str = entry.key();
                    let included_program = entry.value();
                    if let Some(class) = included_program.classes.get(&target)
                        && let Some(target_uri) = parse_uri_or_path(included_uri_str)
                    {
                        trace!(
                            "gs_goto_definition checking included target_uri: {:?}",
                            target_uri
                        );
                        if target_uri != uri {
                            return Some(GotoDefinitionResponse::Link(vec![LocationLink {
                                origin_selection_range,
                                target_uri,
                                target_range: class.range,
                                target_selection_range: class.name.range,
                            }]));
                        }
                        return Some(GotoDefinitionResponse::Array(vec![Location {
                            uri: target_uri,
                            range: class.name.range,
                        }]));
                    }
                    None
                });
            }

            if let Some(resp) = found_class {
                match resp {
                    GotoDefinitionResponse::Link(_) => return Some(resp),
                    GotoDefinitionResponse::Array(locs) => locations.extend(locs),
                    _ => {}
                }
            }

            // 2. If it might be a method or member, find the class context and search up the hierarchy.
            if locations.is_empty() {
                let start_class = expected_receiver_class.clone().or(current_class.clone());

                if let Some(class_name) = start_class {
                    trace!("gs_goto_definition found current class: {}", class_name);
                    let mut visited_classes = std::collections::HashSet::new();
                    let mut to_visit = vec![class_name];

                    while let Some(cls_name) = to_visit.pop() {
                        trace!("gs_goto_definition visiting class: {}", cls_name);
                        if !visited_classes.insert(cls_name.clone()) {
                            continue;
                        }

                        // Find the class definition for cls_name
                        let mut class_def = None;
                        let mut class_uri = None;

                        // Search in current file
                        if let Some(cls) = program.classes.get(&cls_name) {
                            class_def = Some(cls.clone());
                            class_uri = Some(uri.clone());
                        }

                        if class_def.is_none()
                            && let Some((cls, uri)) =
                                parsed_files.par_iter().find_map_any(|entry| {
                                    let included_uri_str = entry.key();
                                    let included_program = entry.value();
                                    included_program.classes.get(&cls_name).map(|class| {
                                        (class.clone(), parse_uri_or_path(included_uri_str))
                                    })
                                })
                        {
                            class_def = Some(cls);
                            class_uri = uri;
                        }

                        if let Some(cls) = class_def {
                            // Check methods
                            if let Some(ms) = cls.methods.get(&target)
                                && let Some(target_uri) = class_uri.clone()
                            {
                                if target_uri != uri {
                                    return Some(GotoDefinitionResponse::Link(vec![
                                        LocationLink {
                                            origin_selection_range: origin_selection_range,
                                            target_uri,
                                            target_range: ms[0].range,
                                            target_selection_range: ms[0].name.range,
                                        },
                                    ]));
                                }

                                locations.push(Location {
                                    uri: target_uri,
                                    range: ms[0].name.range,
                                });
                                return Some(GotoDefinitionResponse::Array(locations));
                            }

                            // Check fields
                            if let Some(field) = cls.fields.get(&target)
                                && let Some(target_uri) = class_uri
                            {
                                if target_uri != uri {
                                    return Some(GotoDefinitionResponse::Link(vec![
                                        LocationLink {
                                            origin_selection_range: origin_selection_range,
                                            target_uri,
                                            target_range: field.range,
                                            target_selection_range: field.name.range,
                                        },
                                    ]));
                                }

                                locations.push(Location {
                                    uri: target_uri,
                                    range: field.name.range,
                                });
                                return Some(GotoDefinitionResponse::Array(locations));
                            }

                            // If not found, add superclasses to to_visit
                            for super_cls in &cls.superclasses {
                                to_visit.push(super_cls.name.clone());
                            }
                        }
                    }
                }
            }
        }

        if !locations.is_empty() {
            return Some(GotoDefinitionResponse::Array(locations));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::str::FromStr;
    use tower_lsp_server::ls_types::Range;
    use trainz_parser::gs::parse;

    #[test]
    fn test_gs_goto_definition() {
        let _ = env_logger::builder().is_test(true).try_init();
        let source = "class MyClass { void MyMethod() { MyMethod(); } };";
        let pairs = parse(source).unwrap();
        let program = Arc::new(trainz_ast::gs::process::process_trainz_ast(pairs, source));
        let uri = Uri::from_str("file:///test.gs").unwrap();
        let position = Position {
            line: 0,
            character: 35,
        };
        let parsed_files = DashMap::new();
        parsed_files.insert(uri.to_string(), program.clone());

        let result = gs_goto_definition(program, position, uri, &parsed_files);
        assert!(result.is_some());
        if let Some(GotoDefinitionResponse::Array(locations)) = result {
            assert_eq!(locations.len(), 1);
            assert_eq!(locations[0].range.start.character, 21);
        }
    }

    #[test]
    fn test_gs_goto_definition_superclass() {
        let _ = env_logger::builder().is_test(true).try_init();
        let super_uri = "file:///super.gs";
        let sub_uri = "file:///sub.gs";

        let super_source = "class SuperClass { void MyMethod() {} };";
        let super_pairs = parse(super_source).unwrap();
        let super_program = Arc::new(trainz_ast::gs::process::process_trainz_ast(
            super_pairs,
            super_source,
        ));

        let sub_source =
            "class SubClass isclass SuperClass { void AnotherMethod() { MyMethod(); } };";
        let sub_pairs = parse(sub_source).unwrap();
        let sub_program = Arc::new(trainz_ast::gs::process::process_trainz_ast(
            sub_pairs, sub_source,
        ));

        let parsed_files = DashMap::new();
        parsed_files.insert(super_uri.to_string(), super_program);
        parsed_files.insert(sub_uri.to_string(), sub_program.clone());

        let position = Position {
            line: 0,
            character: 60,
        }; // Middle of "MyMethod"

        let result = gs_goto_definition(
            sub_program,
            position,
            Uri::from_str(sub_uri).unwrap(),
            &parsed_files,
        );
        assert!(result.is_some(), "Should find definition in superclass");
        if let Some(GotoDefinitionResponse::Link(links)) = result {
            assert_eq!(links.len(), 1);
            assert_eq!(links[0].target_uri.to_string(), super_uri);
        } else {
            panic!("Expected Link, got {:?}", result);
        }
    }

    #[test]
    fn test_gs_goto_definition_superclass_chain() {
        let _ = env_logger::builder().is_test(true).try_init();
        let gp_uri = "file:///gp.gs";
        let p_uri = "file:///p.gs";
        let c_uri = "file:///c.gs";

        let gp_source = "class GrandParent { void GPMethod() {} };";
        let gp_pairs = parse(gp_source).unwrap();
        let gp_program = Arc::new(trainz_ast::gs::process::process_trainz_ast(
            gp_pairs, gp_source,
        ));

        let p_source = "class Parent isclass GrandParent { };";
        let p_pairs = parse(p_source).unwrap();
        let p_program = Arc::new(trainz_ast::gs::process::process_trainz_ast(
            p_pairs, p_source,
        ));

        let c_source = "class Child isclass Parent { void ChildMethod() { GPMethod(); } };";
        let c_pairs = parse(c_source).unwrap();
        let c_program = Arc::new(trainz_ast::gs::process::process_trainz_ast(
            c_pairs, c_source,
        ));

        let parsed_files = DashMap::new();
        parsed_files.insert(gp_uri.to_string(), gp_program);
        parsed_files.insert(p_uri.to_string(), p_program);
        parsed_files.insert(c_uri.to_string(), c_program.clone());

        let position = Position {
            line: 0,
            character: 52,
        }; // Middle of "GPMethod"

        let result = gs_goto_definition(
            c_program,
            position,
            Uri::from_str(c_uri).unwrap(),
            &parsed_files,
        );
        assert!(result.is_some(), "Should find definition in GrandParent");
        if let Some(GotoDefinitionResponse::Link(links)) = result {
            assert_eq!(links.len(), 1);
            assert_eq!(links[0].target_uri.to_string(), gp_uri);
        } else {
            panic!("Expected Link, got {:?}", result);
        }
    }

    #[test]
    fn test_gs_goto_definition_superclass_member_chain() {
        let _ = env_logger::builder().is_test(true).try_init();
        let gp_uri = "file:///gp.gs";
        let p_uri = "file:///p.gs";
        let c_uri = "file:///c.gs";

        let gp_source = "class GrandParent { int gp_member; };";
        let gp_pairs = parse(gp_source).unwrap();
        let gp_program = Arc::new(trainz_ast::gs::process::process_trainz_ast(
            gp_pairs, gp_source,
        ));

        let p_source = "class Parent isclass GrandParent { };";
        let p_pairs = parse(p_source).unwrap();
        let p_program = Arc::new(trainz_ast::gs::process::process_trainz_ast(
            p_pairs, p_source,
        ));

        let c_source = "class Child isclass Parent { void ChildMethod() { gp_member = 1; } };";
        let c_pairs = parse(c_source).unwrap();
        let c_program = Arc::new(trainz_ast::gs::process::process_trainz_ast(
            c_pairs, c_source,
        ));

        let parsed_files = DashMap::new();
        parsed_files.insert(gp_uri.to_string(), gp_program);
        parsed_files.insert(p_uri.to_string(), p_program);
        parsed_files.insert(c_uri.to_string(), c_program.clone());

        let position = Position {
            line: 0,
            character: 52,
        }; // Middle of "gp_member"

        let result = gs_goto_definition(
            c_program,
            position,
            Uri::from_str(c_uri).unwrap(),
            &parsed_files,
        );
        assert!(result.is_some(), "Should find gp_member in GrandParent");
        if let Some(GotoDefinitionResponse::Link(links)) = result {
            assert_eq!(links.len(), 1);
            assert_eq!(links[0].target_uri.to_string(), gp_uri);
        } else {
            panic!("Expected Link, got {:?}", result);
        }
    }

    #[test]
    fn test_gs_goto_definition_superclass_name() {
        let _ = env_logger::builder().is_test(true).try_init();
        let source = "class Base { }; class Derived isclass Base { };";
        let pairs = parse(source).unwrap();
        let program = Arc::new(trainz_ast::gs::process::process_trainz_ast(pairs, source));
        let uri = Uri::from_str("file:///test.gs").unwrap();
        let position = Position {
            line: 0,
            character: 40,
        }; // On "Base" in "isclass Base"
        let parsed_files = DashMap::new();
        parsed_files.insert(uri.to_string(), program.clone());

        let result = gs_goto_definition(program, position, uri, &parsed_files);
        assert!(result.is_some());
        if let Some(GotoDefinitionResponse::Array(locations)) = result {
            assert_eq!(locations.len(), 1);
            assert_eq!(locations[0].range.start.character, 6);
        }
    }

    #[test]
    fn test_gs_goto_definition_inherited() {
        let _ = env_logger::builder().is_test(true).try_init();
        let super_uri = "file:///super.gs";
        let sub_uri = "file:///sub.gs";

        let super_source = "class SuperClass { void MyMethod() {} };";
        let super_pairs = parse(super_source).unwrap();
        let super_program = Arc::new(trainz_ast::gs::process::process_trainz_ast(
            super_pairs,
            super_source,
        ));

        let sub_source = "class SubClass isclass SuperClass { void MyMethod() { inherited(); } };";
        // 01234567890123456789012345678901234567890123456789012345678901234567890
        //                                                      ^ 54
        let sub_pairs = parse(sub_source).unwrap();
        let sub_program = Arc::new(trainz_ast::gs::process::process_trainz_ast(
            sub_pairs, sub_source,
        ));

        let sub_program_with_include = Arc::new(Program {
            includes: vec![trainz_ast::gs::Include {
                path: Some(std::path::PathBuf::from("/super.gs")),
                path_range: None,
                name: "super.gs".to_string(),
                range: Range::default(),
                keyword_include_range: Range::default(),
            }],
            ..(*sub_program).clone()
        });

        let parsed_files = DashMap::new();
        parsed_files.insert(super_uri.to_string(), super_program);
        parsed_files.insert(sub_uri.to_string(), sub_program_with_include.clone());

        let position = Position {
            line: 0,
            character: 54,
        }; // Middle of "inherited"

        let result = gs_goto_definition(
            sub_program_with_include,
            position,
            Uri::from_str(sub_uri).unwrap(),
            &parsed_files,
        );
        assert!(
            result.is_some(),
            "Should find parent MyMethod definition via inherited()"
        );
        if let Some(GotoDefinitionResponse::Link(links)) = result {
            assert_eq!(links.len(), 1);
            assert_eq!(links[0].target_uri.to_string(), super_uri);
            assert_eq!(links[0].target_selection_range.start.character, 24); // SuperClass::MyMethod
        } else {
            panic!("Expected Link, got {:?}", result);
        }
    }

    #[test]
    fn test_gs_goto_definition_inherited_deep() {
        let _ = env_logger::builder().is_test(true).try_init();
        let gp_uri = "file:///gp.gs";
        let p_uri = "file:///p.gs";
        let c_uri = "file:///c.gs";

        let gp_source = "class GrandParent { void SharedMethod() {} };";
        let gp_pairs = parse(gp_source).unwrap();
        let gp_program = Arc::new(trainz_ast::gs::process::process_trainz_ast(
            gp_pairs, gp_source,
        ));

        let p_source = "class Parent isclass GrandParent { };";
        let p_pairs = parse(p_source).unwrap();
        let p_program = Arc::new(trainz_ast::gs::process::process_trainz_ast(
            p_pairs, p_source,
        ));

        let c_source = "class Child isclass Parent { void SharedMethod() { inherited(); } };";
        // 012345678901234567890123456789012345678901234567890123
        //                                                   ^ 51
        let c_pairs = parse(c_source).unwrap();
        let c_program = Arc::new(trainz_ast::gs::process::process_trainz_ast(
            c_pairs, c_source,
        ));

        // Setup includes
        let c_program_with_include = Arc::new(Program {
            includes: vec![trainz_ast::gs::Include {
                path: Some(std::path::PathBuf::from("/p.gs")),
                path_range: None,
                name: "p.gs".to_string(),
                range: Range::default(),
                keyword_include_range: Range::default(),
            }],
            ..(*c_program).clone()
        });

        let p_program_with_include = Arc::new(Program {
            includes: vec![trainz_ast::gs::Include {
                path: Some(std::path::PathBuf::from("/gp.gs")),
                path_range: None,
                name: "gp.gs".to_string(),
                range: Range::default(),
                keyword_include_range: Range::default(),
            }],
            ..(*p_program).clone()
        });

        let parsed_files = DashMap::new();
        parsed_files.insert(gp_uri.to_string(), gp_program);
        parsed_files.insert(p_uri.to_string(), p_program_with_include);
        parsed_files.insert(c_uri.to_string(), c_program_with_include.clone());

        let position = Position {
            line: 0,
            character: 51,
        }; // Middle of "inherited"

        let res = gs_goto_definition(
            c_program_with_include,
            position,
            Uri::from_str(c_uri).unwrap(),
            &parsed_files,
        );
        assert!(
            res.is_some(),
            "Should find GrandParent SharedMethod via inherited() from Child"
        );
        if let Some(GotoDefinitionResponse::Link(links)) = res {
            assert_eq!(links.len(), 1);
            assert_eq!(links[0].target_uri.to_string(), gp_uri);
        } else {
            panic!("Expected Link, got {:?}", res);
        }
    }

    #[test]
    fn test_gs_goto_definition_inherited_not_current() {
        let _ = env_logger::builder().is_test(true).try_init();
        let uri = "file:///test.gs";
        let source = "class MyClass isclass MyClass { void SharedMethod() { inherited(); } };";
        // 0123456789012345678901234567890123456789012345678901234567890
        //                                                   ^ 54
        let pairs = parse(source).unwrap();
        let program = Arc::new(trainz_ast::gs::process::process_trainz_ast(pairs, source));
        let parsed_files = DashMap::new();
        parsed_files.insert(uri.to_string(), program.clone());

        let position = Position {
            line: 0,
            character: 54,
        };

        let res = gs_goto_definition(
            program,
            position,
            Uri::from_str(uri).unwrap(),
            &parsed_files,
        );

        // It should NOT find MyClass::SharedMethod as inherited, even though MyClass is its own superclass.
        assert!(
            res.is_none(),
            "Should NOT return current method for inherited() call"
        );
    }

    #[test]
    fn test_gs_goto_definition_include() {
        let _ = env_logger::builder().is_test(true).try_init();
        let include_path = "/path/to/Bar.gs";
        let source = format!("include \"{}\"\nclass Foo {{ }};", include_path);
        let pairs = parse(&source).unwrap();
        let mut program = trainz_ast::gs::process::process_trainz_ast(pairs, &source);
        println!("Includes count: {}", program.includes.len());
        if !program.includes.is_empty() {
            println!("Include 0 range: {:?}", program.includes[0].range);
            println!("Include 0 path_range: {:?}", program.includes[0].path_range);
        }
        let path_range = program.includes[0].path_range;
        program.includes[0].path = Some(PathBuf::from(include_path));
        let program = Arc::new(program);
        let uri = Uri::from_str("file:///test.gs").unwrap();
        let position = path_range.unwrap().start; // within the include path string
        println!("Testing position: {:?}", position);
        let parsed_files = DashMap::new();
        parsed_files.insert(uri.to_string(), program.clone());

        let result = gs_goto_definition(program.clone(), position, uri.clone(), &parsed_files);

        assert!(result.is_some());
        if let Some(GotoDefinitionResponse::Link(links)) = result {
            assert_eq!(links.len(), 1);
            assert_eq!(
                links[0].target_uri.to_file_path().unwrap(),
                PathBuf::from(include_path)
            );
        } else {
            panic!("Expected Link response");
        }
    }

    #[test]
    fn test_gs_goto_definition_field_method_call() {
        let _ = env_logger::builder().is_test(true).try_init();
        let source = r#"
            class Soup {
                void CountTags() { }
            };
            class Test {
                Soup soup;
                void Run() {
                    soup.CountTags();
                }
            };
        "#;
        let pairs = parse(source).unwrap();
        let program = Arc::new(trainz_ast::gs::process::process_trainz_ast(pairs, source));
        let uri = Uri::from_str("file:///test.gs").unwrap();
        let position = Position {
            line: 7,
            character: 28,
        };
        let parsed_files = DashMap::new();

        let result = gs_goto_definition(program, position, uri, &parsed_files);

        assert!(result.is_some());
        if let Some(GotoDefinitionResponse::Array(locations)) = result {
            assert_eq!(locations.len(), 1);
            let loc = &locations[0];
            assert_eq!(loc.range.start.line, 2);
            assert_eq!(loc.range.start.character, 21);
            assert_eq!(loc.range.end.line, 2);
            assert_eq!(loc.range.end.character, 30);
        } else {
            panic!("Expected Array response, got {:?}", result);
        }
    }

    #[test]
    fn test_gs_goto_definition_method_call() {
        let _ = env_logger::builder().is_test(true).try_init();
        let source = r#"
            class Str {
                void Tokens(int pid, string s) { }
            };
            class Test {
                void Run() {
                    Str.Tokens(1, "_");
                }
            };
        "#;
        let pairs = trainz_ast::gs::process::process_trainz_ast(
            trainz_parser::gs::parse(source).unwrap(),
            source,
        );
        let program = Arc::new(pairs);
        let uri = Uri::from_str("file:///test.gs").unwrap();
        let parsed_files = DashMap::new();

        // Position on "Tokens" in `Str.Tokens`
        let position = Position {
            line: 6,
            character: 25,
        };

        let result = gs_goto_definition(program.clone(), position, uri.clone(), &parsed_files);
        assert!(result.is_some(), "Expected a definition to be found");
        let locs = match result.unwrap() {
            GotoDefinitionResponse::Array(locs) => locs,
            GotoDefinitionResponse::Link(links) => links
                .into_iter()
                .map(|l| Location {
                    uri: l.target_uri,
                    range: l.target_selection_range,
                })
                .collect(),
            _ => panic!("Expected array or link"),
        };
        assert_eq!(locs.len(), 1);
        assert_eq!(locs[0].range.start.line, 2);
    }

    #[test]
    fn test_gs_goto_definition_chained_method_call() {
        let _ = env_logger::builder().is_test(true).try_init();
        let source = r#"
            class Asset {
                void FindAsset(string name) { }
            };
            class System {
                Asset GetAsset() { return new Asset(); }
            };
            class Test {
                void Run() {
                    System sys;
                    sys.GetAsset().FindAsset("lamp-lib");
                }
            };
        "#;
        let pairs = trainz_ast::gs::process::process_trainz_ast(
            trainz_parser::gs::parse(source).unwrap(),
            source,
        );
        let program = Arc::new(pairs);
        let uri = Uri::from_str("file:///test.gs").unwrap();
        let parsed_files = DashMap::new();

        // Position on "FindAsset" in `sys.GetAsset().FindAsset("lamp-lib");`
        let position = Position {
            line: 10,
            character: 35,
        };

        let result = gs_goto_definition(program.clone(), position, uri.clone(), &parsed_files);
        assert!(
            result.is_some(),
            "Expected a definition to be found for chained method call"
        );
        let locs = match result.unwrap() {
            GotoDefinitionResponse::Array(locs) => locs,
            GotoDefinitionResponse::Link(links) => links
                .into_iter()
                .map(|l| Location {
                    uri: l.target_uri,
                    range: l.target_selection_range,
                })
                .collect(),
            _ => panic!("Expected array or link"),
        };
        assert_eq!(locs.len(), 1);
        assert_eq!(locs[0].range.start.line, 2);
    }

    #[test]
    fn test_gs_goto_definition_chained_method_inheritance() {
        let _ = env_logger::builder().is_test(true).try_init();
        let source = r#"
            class Base {
                Soup GetConfigSoup() { return null; }
            };
            class Asset isclass Base {
            };
            class Soup {
                Soup GetNamedSoup(string name) { return null; }
            };
            class Test {
                Asset GetAsset() { return null; }
                void Run() {
                    GetAsset().GetConfigSoup().GetNamedSoup("mesh-table");
                }
            };
        "#;
        let pairs = parse(source).unwrap();
        let program = Arc::new(trainz_ast::gs::process::process_trainz_ast(pairs, source));
        let uri = Uri::from_str("file:///test.gs").unwrap();
        // "GetAsset().GetConfigSoup().GetNamedSoup("mesh-table");"
        let position = Position {
            line: 12,
            character: 53,
        }; // middle of GetNamedSoup
        let parsed_files = DashMap::new();
        parsed_files.insert(uri.to_string(), program.clone());

        let result = gs_goto_definition(program, position, uri.clone(), &parsed_files);
        assert!(result.is_some(), "Expected to find definition");
        let locs = match result.unwrap() {
            GotoDefinitionResponse::Array(locs) => locs,
            GotoDefinitionResponse::Link(links) => links
                .into_iter()
                .map(|l| Location {
                    uri: l.target_uri,
                    range: l.target_selection_range,
                })
                .collect(),
            _ => panic!("Expected array or link"),
        };
        assert_eq!(locs.len(), 1);
        assert_eq!(locs[0].range.start.line, 7);
    }

    #[test]
    fn test_gs_goto_definition_get_named_soup_inheritance() {
        let _ = env_logger::builder().is_test(true).try_init();
        let source = r#"
            class BaseSoup {
                Soup GetNamedSoup(string name) { return null; }
            };
            class Soup isclass BaseSoup {
            };
            class Base {
                Soup GetConfigSoup() { return null; }
            };
            class Asset isclass Base {
            };
            class Test {
                Asset GetAsset() { return null; }
                void Run() {
                    GetAsset().GetConfigSoup().GetNamedSoup("mesh-table");
                }
            };
        "#;
        let pairs = parse(source).unwrap();
        let program = Arc::new(trainz_ast::gs::process::process_trainz_ast(pairs, source));
        let uri = Uri::from_str("file:///test.gs").unwrap();
        // "GetAsset().GetConfigSoup().GetNamedSoup("mesh-table");"
        let position = Position {
            line: 14,
            character: 53,
        }; // middle of GetNamedSoup
        let parsed_files = DashMap::new();
        parsed_files.insert(uri.to_string(), program.clone());

        let result = gs_goto_definition(program, position, uri.clone(), &parsed_files);
        assert!(result.is_some(), "Expected to find definition");
        let locs = match result.unwrap() {
            GotoDefinitionResponse::Array(locs) => locs,
            GotoDefinitionResponse::Link(links) => links
                .into_iter()
                .map(|l| Location {
                    uri: l.target_uri,
                    range: l.target_selection_range,
                })
                .collect(),
            _ => panic!("Expected array or link"),
        };
        assert_eq!(locs.len(), 1);
        assert_eq!(locs[0].range.start.line, 2);
    }

    #[test]
    fn test_gs_goto_definition_local_var_method_call() {
        let _ = env_logger::builder().is_test(true).try_init();
        let source = r#"
class Soup {
    public void GetIndexedTagName(int i) {}
};

class SignalNSW {
    public void UnSetLamps() {
        Soup meshtable = new Soup();
        meshtable.GetIndexedTagName(0);
    }
};
        "#;
        let pairs = trainz_ast::gs::process::process_trainz_ast(
            trainz_parser::gs::parse(source).unwrap(),
            source,
        );
        let program = Arc::new(pairs);
        let parsed_files = DashMap::new();
        parsed_files.insert("test://file".to_string(), program.clone());
        let uri = Uri::from_str("test://file").unwrap();

        let position = Position {
            line: 8,
            character: 25,
        }; // inside GetIndexedTagName
        let result = gs_goto_definition(program, position, uri.clone(), &parsed_files);
        assert!(result.is_some(), "Definition not found");

        let locs = match result.unwrap() {
            GotoDefinitionResponse::Array(locs) => locs,
            GotoDefinitionResponse::Link(links) => links
                .into_iter()
                .map(|l| Location {
                    uri: l.target_uri,
                    range: l.target_selection_range,
                })
                .collect(),
            _ => panic!("Expected array or link"),
        };
        assert_eq!(locs.len(), 1);
        assert_eq!(locs[0].range.start.line, 2);
    }

    #[test]
    fn test_gs_goto_definition_meshtable_resolution() {
        let _ = env_logger::builder().is_test(true).try_init();
        let source = r#"
class Soup {
    public void GetIndexedTagName(int i) {}
};

class SignalNSW {
    Soup meshtable;
    public void UnSetLamps() {
        int i = 0;
        meshtable.GetIndexedTagName(i);
    }
};
        "#;
        let pairs = trainz_ast::gs::process::process_trainz_ast(
            trainz_parser::gs::parse(source).unwrap(),
            source,
        );
        let program = Arc::new(pairs);
        let parsed_files = DashMap::new();
        let uri = Uri::from_str("test://file").unwrap();
        parsed_files.insert(uri.to_string(), program.clone());

        let position = Position {
            line: 9,
            character: 25,
        }; // inside GetIndexedTagName
        let result = gs_goto_definition(program, position, uri.clone(), &parsed_files);
        assert!(result.is_some(), "Definition not found");

        let locs = match result.unwrap() {
            GotoDefinitionResponse::Array(locs) => locs,
            GotoDefinitionResponse::Link(links) => links
                .into_iter()
                .map(|l| Location {
                    uri: l.target_uri,
                    range: l.target_selection_range,
                })
                .collect(),
            _ => panic!("Expected array or link"),
        };
        assert_eq!(locs.len(), 1);
        assert_eq!(locs[0].range.start.line, 2);
    }

    #[test]
    fn test_gs_goto_definition_multilevel_inheritance() {
        let _ = env_logger::builder().is_test(true).try_init();
        let source = r#"
class GrandParent {
    public void GrandMethod() {}
};
class Parent isclass GrandParent {
    public void ParentMethod() {}
};
class Child isclass Parent {
    public void ChildMethod() {}
};

class Test {
    public void Run() {
        Child c = new Child();
        c.GrandMethod();
    }
};
        "#;
        let pairs = trainz_ast::gs::process::process_trainz_ast(
            trainz_parser::gs::parse(source).unwrap(),
            source,
        );
        let program = Arc::new(pairs);
        let parsed_files = DashMap::new();
        let uri = Uri::from_str("test://file").unwrap();
        parsed_files.insert(uri.to_string(), program.clone());

        let position = Position {
            line: 14,
            character: 15,
        }; // inside GrandMethod
        let result = gs_goto_definition(program, position, uri.clone(), &parsed_files);
        assert!(result.is_some(), "Definition not found");

        let locs = match result.unwrap() {
            GotoDefinitionResponse::Array(locs) => locs,
            GotoDefinitionResponse::Link(links) => links
                .into_iter()
                .map(|l| Location {
                    uri: l.target_uri,
                    range: l.target_selection_range,
                })
                .collect(),
            _ => panic!("Expected array or link"),
        };
        assert_eq!(locs.len(), 1);
        assert_eq!(locs[0].range.start.line, 2);
    }

    #[test]
    fn test_gs_goto_definition_inherited_field_method_call() {
        let _ = env_logger::builder().is_test(true).try_init();
        let source = r#"
class Soup {
    public void GetIndexedTagName(int i) {}
};

class BaseClass {
    Soup meshtable;
};

class SignalNSW isclass BaseClass {
    public void UnSetLamps() {
        int i = 0;
        meshtable.GetIndexedTagName(i);
    }
};
        "#;
        let pairs = trainz_ast::gs::process::process_trainz_ast(
            trainz_parser::gs::parse(source).unwrap(),
            source,
        );
        let program = Arc::new(pairs);
        let parsed_files = DashMap::new();
        let uri = Uri::from_str("test://file").unwrap();
        parsed_files.insert(uri.to_string(), program.clone());

        let position = Position {
            line: 12,
            character: 25,
        }; // inside GetIndexedTagName
        let result = gs_goto_definition(program, position, uri.clone(), &parsed_files);
        assert!(result.is_some(), "Definition not found");

        let locs = match result.unwrap() {
            GotoDefinitionResponse::Array(locs) => locs,
            GotoDefinitionResponse::Link(links) => links
                .into_iter()
                .map(|l| Location {
                    uri: l.target_uri,
                    range: l.target_selection_range,
                })
                .collect(),
            _ => panic!("Expected array or link"),
        };
        assert_eq!(locs.len(), 1);
        assert_eq!(locs[0].range.start.line, 2);
    }
}
