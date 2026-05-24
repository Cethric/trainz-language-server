use crate::gs::{format_method_hover, get_text_from_range};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::collections::{HashMap, HashSet};
use tower_lsp_server::ls_types::{Hover, HoverContents, MarkupContent, MarkupKind};
use tracing::debug;
use trainz_ast::Position;
use trainz_ast::find::HasRange;
use trainz_ast::gs::dependency_graph::ProgramResolver;
use trainz_ast::gs::find::{
    find_field_by_id_range, find_id_at_position, find_include_at_position, find_method_by_id_range,
    find_param_by_id_range, find_postfix_at_position,
};
use trainz_ast::gs::type_eval::{ClassResolver, EvaluatedType, evaluate_expr_type};
use trainz_ast::gs::{Expr, FieldModifier, Program, Type};

#[tracing::instrument(skip(resolver, program_resolver))]
pub fn trainz_hover(
    program: &Program,
    resolver: &dyn ClassResolver,
    program_resolver: &dyn ProgramResolver,
    position: Position,
) -> Option<Hover> {
    debug!("trainz_hover: position={:?}", position);
    if let Some(include) = find_include_at_position(program, position)
        && let Some(path) = &include.path
    {
        let path_str = path.to_string_lossy().to_string();
        if let Some(included_program) = program_resolver.resolve_program(&path_str) {
            let transitive = trainz_ast::gs::dependency_graph::get_transitive_programs(
                &included_program,
                program_resolver,
            );

            let mut classes = HashSet::<String>::new();
            for (_, p) in std::iter::once(("".to_string(), included_program)).chain(transitive) {
                for class_name in p.classes.keys() {
                    classes.insert(class_name.clone());
                }
            }

            if !classes.is_empty() {
                let mut names = classes
                    .par_iter()
                    .map(|c| format!("- {}", c))
                    .collect::<Vec<String>>();
                names.sort();
                return Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: format!("Includes classes:\n{}", names.join("\n")),
                    }),
                    range: Some(include.range),
                });
            }
        }
    }

    let id = find_id_at_position(program, position)?;
    debug!("trainz_hover: found id='{}' at {:?}", id.name, id.range);

    if let Some((ty, name)) = program.find_variable_declaration(&id.name, position) {
        let mut value = format!("{} {}", ty, name.name);

        if let Some(field) = find_field_by_id_range(program, name.range) {
            let is_define = field
                .modifiers
                .iter()
                .any(|m| matches!(m.0, FieldModifier::Define));

            let class_prefix = if let Some(parent) = &field.parent_class {
                format!("{}::", parent)
            } else {
                "".to_string()
            };

            if is_define {
                if let Some(init) = &field.initializer
                    && let Some(init_str) =
                        get_text_from_range::get_text_from_range(&program.src, init.range())
                {
                    value = format!("{} {}{} = {}", ty, class_prefix, name.name, init_str);
                }
            } else {
                value = format!("field: {} {}{}", ty, class_prefix, name.name);
            }
        } else if find_param_by_id_range(program, name.range).is_some() {
            value = format!("parameter: {} {}", ty, name.name);
        }

        return Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value,
            }),
            range: Some(id.range),
        });
    }

    if let Some(method) = find_method_by_id_range(program, id.range) {
        return Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format_method_hover::format_method_hover(method),
            }),
            range: Some(id.range),
        });
    }

    // Check if it's a method call or variable in the current context
    let current_class = trainz_ast::gs::find::find_class_at_position(program, position);
    if id.name == "me"
        && let Some(class) = current_class
    {
        return Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!("class {}", class.name.name),
            }),
            range: Some(id.range),
        });
    }

    if id.name == "isclass" {
        return Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: "bool isclass(object cls)".to_string(),
            }),
            range: Some(id.range),
        });
    }

    if let Some(class) = current_class {
        if let Some(field) = class.find_field(resolver, &id.name) {
            let class_prefix = if let Some(parent) = &field.parent_class {
                format!("{}::", parent)
            } else {
                "".to_string()
            };
            return Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: format!("field: {} {}{}", field.ty, class_prefix, field.name.name),
                }),
                range: Some(id.range),
            });
        }
        if let Some(methods) = class.find_method(resolver, &id.name) {
            let mut value = String::new();
            for (i, method) in methods.iter().enumerate() {
                if i > 0 {
                    value.push_str("\n\n---\n\n");
                }
                value.push_str(&format_method_hover::format_method_hover(method));
            }
            return Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value,
                }),
                range: Some(id.range),
            });
        }
    }

    // Check if it's a member of a postfix expression (chained call)
    if let Some((postfix_expr, idx)) = find_postfix_at_position(program, position)
        && let Expr::Postfix { expr, ops, .. } = postfix_expr
    {
        let ops_before = &ops[..idx];
        let res = if ops_before.is_empty() {
            evaluate_expr_type(
                expr,
                program,
                resolver,
                position,
                current_class,
                &HashMap::new(),
            )
        } else {
            let temp_expr = Expr::Postfix {
                expr: expr.clone(),
                ops: ops_before.to_vec(),
                range: expr.range(),
            };
            evaluate_expr_type(
                &temp_expr,
                program,
                resolver,
                position,
                current_class,
                &HashMap::new(),
            )
        };

        match res {
            Ok(EvaluatedType::Type(Type::Named(class_id))) => {
                if let Some(class) = resolver.find_class(&class_id.name) {
                    if let Some(field) = class.find_field(resolver, &id.name) {
                        let class_prefix = if let Some(parent) = &field.parent_class {
                            format!("{}::", parent)
                        } else {
                            "".to_string()
                        };
                        return Some(Hover {
                            contents: HoverContents::Markup(MarkupContent {
                                kind: MarkupKind::Markdown,
                                value: format!(
                                    "field: {} {}{}",
                                    field.ty, class_prefix, field.name.name
                                ),
                            }),
                            range: Some(id.range),
                        });
                    } else if let Some(methods) = class.find_method(resolver, &id.name) {
                        let mut value = String::new();
                        for (i, method) in methods.iter().enumerate() {
                            if i > 0 {
                                value.push_str("\n\n---\n\n");
                            }
                            value.push_str(&format_method_hover::format_method_hover(method));
                        }
                        return Some(Hover {
                            contents: HoverContents::Markup(MarkupContent {
                                kind: MarkupKind::Markdown,
                                value,
                            }),
                            range: Some(id.range),
                        });
                    }
                }
                if id.name == "isclass" {
                    return Some(Hover {
                        contents: HoverContents::Markup(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: "bool isclass(object cls)".to_string(),
                        }),
                        range: Some(id.range),
                    });
                }
            }
            Ok(ref eval_res @ EvaluatedType::Array(..))
            | Ok(ref eval_res @ EvaluatedType::Type(Type::Array(..)))
            | Ok(ref eval_res @ EvaluatedType::Type(Type::String(..))) => {
                if id.name == "size" {
                    return Some(Hover {
                        contents: HoverContents::Markup(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: "int size()".to_string(),
                        }),
                        range: Some(id.range),
                    });
                }

                let inner_type_str = match eval_res {
                    EvaluatedType::Array(inner, _, _) => Some(format!("{}", inner)),
                    EvaluatedType::Type(Type::Array(inner, _)) => Some(format!("{}", inner)),
                    _ => None,
                };

                if id.name == "copy"
                    && let Some(inner) = inner_type_str
                {
                    return Some(Hover {
                        contents: HoverContents::Markup(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: format!("{}[] copy()", inner),
                        }),
                        range: Some(id.range),
                    });
                }
            }
            _ => {}
        }
    }

    // Check if it's a class name
    if let Some(cls) = resolver.find_class(&id.name) {
        debug!("trainz_hover: matched class name '{}'", cls.name.name);
        return Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!("class {}", cls.name.name),
            }),
            range: Some(id.range),
        });
    }

    debug!("trainz_hover: no hover match found for id='{}'", id.name);
    None
}
