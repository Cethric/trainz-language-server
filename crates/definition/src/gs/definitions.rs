use dashmap::DashMap;
use rayon::prelude::*;
use std::collections::HashSet;
use std::sync::Arc;
use tower_lsp_server::ls_types::{GotoDefinitionResponse, Location, LocationLink, Position, Uri};
use tracing::trace;
use trainz_ast::find::{HasRange, position_in_range};
use trainz_ast::gs::find::{find_id_at_position, find_postfix_at_position};
use trainz_ast::gs::program::Program;
use trainz_ast::gs::type_eval::{ClassResolver, EvaluatedType, evaluate_expr_type};
use trainz_ast::gs::{Expr, MethodDef, PostfixOp, Type};

struct CombinedResolver<'a> {
    current_program: &'a Program,
    parsed_files: &'a DashMap<String, Arc<Program>>,
}

impl<'a> ClassResolver for CombinedResolver<'a> {
    fn find_class(&self, name: &str) -> Option<trainz_ast::gs::ClassDef> {
        let mut visited = HashSet::new();
        self.find_recursive(self.current_program, name, &mut visited)
    }
}

impl<'a> trainz_ast::gs::dependency_graph::ProgramResolver for CombinedResolver<'a> {
    fn resolve_program(&self, path: &str) -> Option<Arc<Program>> {
        self.parsed_files.get(path).map(|p| p.value().clone())
    }
}

impl<'a> CombinedResolver<'a> {
    fn find_recursive(
        &self,
        program: &trainz_ast::gs::Program,
        name: &str,
        visited: &mut HashSet<String>,
    ) -> Option<trainz_ast::gs::ClassDef> {
        if let Some(cls) = program.classes.get(name) {
            return Some(cls.clone());
        }

        for include in &program.includes {
            if let Some(path) = &include.path {
                let path_str = path.to_string_lossy().to_string();
                if visited.insert(path_str.clone())
                    && let Some(entry) = self.parsed_files.get(&path_str)
                    && let Some(cls) = self.find_recursive(entry.value(), name, visited)
                {
                    return Some(cls);
                }
            }
        }
        None
    }
}

#[tracing::instrument(skip(s))]
fn parse_uri_or_path(s: &str) -> Option<Uri> {
    if let Ok(uri) = s.parse::<Uri>()
        && uri.scheme().as_str() != ""
    {
        return Some(uri);
    }
    Uri::from_file_path(s)
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
    let resolver = CombinedResolver {
        current_program: program,
        parsed_files,
    };

    let current_class = current_class_name.and_then(|name| program.classes.get(name));

    // To evaluate the receiver's type, we construct a sub-postfix expression up to the point of dereference.
    // evaluate_expr_type is left-to-right, so we can just pass the expr and ops_before.
    let res = if ops.is_empty() {
        evaluate_expr_type(
            expr,
            program,
            &resolver,
            position,
            current_class,
            &std::collections::HashMap::new(),
        )
    } else {
        let temp_expr = Expr::Postfix {
            expr: Box::new(expr.clone()),
            ops: ops.to_vec(),
            range: expr.range(),
        };
        evaluate_expr_type(
            &temp_expr,
            program,
            &resolver,
            position,
            current_class,
            &std::collections::HashMap::new(),
        )
    };

    match res {
        Ok(ty) => match ty {
            EvaluatedType::Type(Type::Named(id)) => Some(id.name),
            _ => None,
        },
        Err(_) => None,
    }
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

        let mut locations: Vec<LocationLink> = vec![];

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
        for cls in program.classes.values() {
            if let Some(expected) = &expected_receiver_class
                && cls.name.name != *expected
            {
                continue;
            }
            // trace!("gs_goto_definition checking class: {}", cls.name.name);
            if expected_receiver_class.is_none() && cls.name.name == target {
                trace!("gs_goto_definition found local class: {}", cls.name.name);
                locations.push(LocationLink {
                    origin_selection_range,
                    target_uri: uri.clone(),
                    target_range: cls.range,
                    target_selection_range: cls.name.range,
                });
            }
            if let Some(field) = cls.fields.get(&target) {
                locations.push(LocationLink {
                    origin_selection_range,
                    target_uri: uri.clone(),
                    target_range: field.range,
                    target_selection_range: field.name.range,
                });
            }
            if let Some(ms) = cls.methods.get(&target) {
                for method in ms {
                    locations.push(LocationLink {
                        origin_selection_range,
                        target_uri: uri.clone(),
                        target_range: method.range,
                        target_selection_range: method.name.range,
                    });
                }
            }
        }

        // If not found as a local definition, search included files.
        if locations.is_empty() {
            trace!("gs_goto_definition searching included files for {}", target);

            let resolver = CombinedResolver {
                current_program: &program,
                parsed_files,
            };

            let transitive_programs =
                trainz_ast::gs::dependency_graph::get_transitive_programs(&program, &resolver);

            // 1. Check if it's a class in ANY included file.
            let mut found_class = None;
            if expected_receiver_class.is_none() {
                found_class = transitive_programs.par_iter().find_map_any(
                    |(included_uri_str, included_program)| {
                        if let Some(class) = included_program.classes.get(&target)
                            && let Some(target_uri) = parse_uri_or_path(included_uri_str)
                        {
                            trace!(
                                "gs_goto_definition found class in transitive include: {}",
                                target
                            );
                            return Some(GotoDefinitionResponse::Link(vec![LocationLink {
                                origin_selection_range,
                                target_uri,
                                target_range: class.range,
                                target_selection_range: class.name.range,
                            }]));
                        }
                        None
                    },
                );
            }

            if let Some(resp) = found_class
                && let GotoDefinitionResponse::Link(links) = resp
            {
                locations.extend(links)
            }

            // 2. If it might be a method or member, find the class context and search up the hierarchy.
            if locations.is_empty() || expected_receiver_class.is_some() {
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
                            && let Some((cls, uri)) = transitive_programs.par_iter().find_map_any(
                                |(included_uri_str, included_program)| {
                                    included_program.classes.get(&cls_name).map(|class| {
                                        (class.clone(), parse_uri_or_path(included_uri_str))
                                    })
                                },
                            )
                        {
                            class_def = Some(cls);
                            class_uri = uri;
                        }

                        if let Some(cls) = class_def {
                            // Check methods
                            if let Some(ms) = cls.methods.get(&target)
                                && let Some(target_uri) = class_uri.clone()
                            {
                                for m in ms {
                                    locations.push(LocationLink {
                                        origin_selection_range,
                                        target_uri: target_uri.clone(),
                                        target_range: m.range,
                                        target_selection_range: m.name.range,
                                    });
                                }
                            }

                            // Check fields
                            if let Some(field) = cls.fields.get(&target)
                                && let Some(target_uri) = class_uri
                            {
                                locations.push(LocationLink {
                                    origin_selection_range,
                                    target_uri,
                                    target_range: field.range,
                                    target_selection_range: field.name.range,
                                });
                            }

                            // Always add superclasses to to_visit to find all overloads/shadowed members
                            for super_cls in &cls.superclasses {
                                to_visit.push(super_cls.name.clone());
                            }
                        }
                    }
                }
            }
        }

        if !locations.is_empty() {
            return Some(GotoDefinitionResponse::Link(locations));
        }
    }

    None
}
