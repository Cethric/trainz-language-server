use crate::find::HasRange;
use crate::gs::find::find_class_at_position;
use crate::gs::program::Program;
use crate::gs::types::{Type, TypeOrVoid};
use crate::gs::{ClassDef, Expr, Literal, PostfixOp};
use std::collections::HashMap;
use tower_lsp_server::ls_types::Position;

#[derive(Debug, Clone)]
pub enum EvaluatedType {
    Type(Type),
    Array(Type, Option<usize>, crate::Range),
    Methods(Vec<crate::gs::MethodDef>), // Represents methods, its value is its return type
    Void,
}

impl EvaluatedType {
    pub fn to_type(&self) -> Option<Type> {
        match self {
            EvaluatedType::Type(t) => Some(t.clone()),
            EvaluatedType::Array(t, _, range) => Some(Type::Array(Box::new(t.clone()), *range)),
            EvaluatedType::Methods(methods) => {
                if let Some(m) = methods.first() {
                    if let TypeOrVoid::Type(t) = &m.return_type {
                        return Some(t.clone());
                    }
                }
                None
            }
            _ => None,
        }
    }
}

pub fn is_type_compatible(
    expected: &Type,
    actual: &Type,
    program: &Program,
    resolver: &dyn ClassResolver,
) -> bool {
    match (expected, actual) {
        (Type::Bool(_), Type::Bool(_)) => true,
        (Type::Int(_), Type::Int(_)) => true,
        (Type::Float(_), Type::Float(_)) => true,
        (Type::String(_), Type::String(_)) => true,
        (Type::Object(_), Type::Object(_)) => true,
        (Type::Object(_), Type::Named(_)) => true, // Anything can be assigned to object
        (Type::Named(_), Type::Object(_)) => true, // object can be cast to anything? Usually requires explicit cast in many languages, but GS might be loose. Let's be strict for now.
        (Type::Named(e_id), Type::Named(a_id)) => {
            if e_id.name == a_id.name {
                return true;
            }
            // Check inheritance: actual must be a subclass of expected
            if let Some(actual_class) = resolver.find_class(&a_id.name) {
                return actual_class.is_subclass_of(&e_id.name, program, resolver);
            }
            false
        }
        (Type::Array(e_inner, _), Type::Array(a_inner, _)) => {
            is_type_compatible(e_inner, a_inner, program, resolver)
        }
        _ => false,
    }
}

pub trait ClassResolver: Sync + Send {
    fn find_class(&self, name: &str) -> Option<ClassDef>;
}

impl ClassResolver for Program {
    fn find_class(&self, name: &str) -> Option<ClassDef> {
        self.classes.get(name).cloned()
    }
}

pub fn evaluate_expr_type(
    expr: &Expr,
    program: &Program,
    resolver: &dyn ClassResolver,
    pos: Position,
    current_class: Option<&ClassDef>,
    known_array_sizes: &HashMap<String, usize>,
) -> Result<EvaluatedType, String> {
    match expr {
        Expr::Literal(lit) => match lit {
            Literal::Int(_, range) => Ok(EvaluatedType::Type(Type::Int(*range))),
            Literal::Float(_, range) => Ok(EvaluatedType::Type(Type::Float(*range))),
            Literal::String(s) => Ok(EvaluatedType::Type(Type::String(s.range))),
            Literal::Bool(_, range) => Ok(EvaluatedType::Type(Type::Bool(*range))),
            Literal::Hex(_, range) => Ok(EvaluatedType::Type(Type::Int(*range))),
            Literal::Char(_, range) => Ok(EvaluatedType::Type(Type::Int(*range))),
            Literal::Null(range) => Ok(EvaluatedType::Type(Type::Object(*range))),
        },
        Expr::Identifier(id) => {
            if id.name == "me" {
                if let Some(class) = current_class {
                    return Ok(EvaluatedType::Type(Type::Named(class.name.clone())));
                } else if let Some(class) = find_class_at_position(program, pos) {
                    return Ok(EvaluatedType::Type(Type::Named(class.name.clone())));
                }
                return Err("Cannot use 'me' outside of a class".to_string());
            }

            // 1. Look in scope
            if let Some((ty, _)) = program.find_variable_declaration(&id.name, pos) {
                return match ty {
                    Type::Array(inner, range) => {
                        let size = known_array_sizes.get(&id.name).cloned();
                        Ok(EvaluatedType::Array(*inner.clone(), size, *range))
                    }
                    _ => Ok(EvaluatedType::Type(ty.clone())),
                };
            }

            // 2. Look in current class fields
            if let Some(class) = current_class.or_else(|| find_class_at_position(program, pos)) {
                if let Some(field) = class.find_field(program, resolver, &id.name) {
                    return match &field.ty {
                        Type::Array(inner, range) => {
                            let size = known_array_sizes.get(&id.name).cloned();
                            Ok(EvaluatedType::Array(*inner.clone(), size, *range))
                        }
                        _ => Ok(EvaluatedType::Type(field.ty.clone())),
                    };
                }
                if let Some(methods) = class.find_method(program, resolver, &id.name) {
                    return Ok(EvaluatedType::Methods(methods));
                }
            }

            // 3. Look for class name (static access)
            if let Some(cls) = resolver.find_class(&id.name) {
                return Ok(EvaluatedType::Type(Type::Named(cls.name.clone())));
            }

            Err(format!("Identifier '{}' not found", id.name))
        }
        Expr::Postfix { expr, ops, .. } => {
            let mut current_type = evaluate_expr_type(
                expr,
                program,
                resolver,
                pos,
                current_class,
                known_array_sizes,
            )?;

            for op in ops {
                match op {
                    PostfixOp::Deref(id) => {
                        let ty = match &current_type {
                            EvaluatedType::Array(..) | EvaluatedType::Type(Type::Array(..)) => {
                                let (inner_type, range) = match &current_type {
                                    EvaluatedType::Array(inner, _, range) => {
                                        (inner.clone(), *range)
                                    }
                                    EvaluatedType::Type(Type::Array(inner, range)) => {
                                        ((**inner).clone(), *range)
                                    }
                                    _ => unreachable!(),
                                };

                                if id.name == "size" {
                                    EvaluatedType::Methods(vec![crate::gs::MethodDef {
                                        modifiers: vec![],
                                        return_type: TypeOrVoid::Type(Type::Int(id.range)),
                                        name: crate::gs::Identifier {
                                            name: "size".to_string(),
                                            range: id.range,
                                        },
                                        params: vec![],
                                        body: None,
                                        scope_id: 0,
                                        range: id.range,
                                    }])
                                } else if id.name == "copy" {
                                    EvaluatedType::Methods(vec![crate::gs::MethodDef {
                                        modifiers: vec![],
                                        return_type: TypeOrVoid::Type(Type::Array(
                                            Box::new(inner_type),
                                            range,
                                        )),
                                        name: crate::gs::Identifier {
                                            name: "copy".to_string(),
                                            range: id.range,
                                        },
                                        params: vec![],
                                        body: None,
                                        scope_id: 0,
                                        range: id.range,
                                    }])
                                } else {
                                    return Err(format!("Member '{}' not found in array", id.name));
                                }
                            }
                            EvaluatedType::Type(Type::Named(_))
                            | EvaluatedType::Type(Type::Object(_)) => {
                                if id.name == "isclass" {
                                    // isclass is a special method
                                    EvaluatedType::Methods(vec![crate::gs::MethodDef {
                                        modifiers: vec![],
                                        return_type: TypeOrVoid::Type(Type::Bool(id.range)),
                                        name: crate::gs::Identifier {
                                            name: "isclass".to_string(),
                                            range: id.range,
                                        },
                                        params: vec![crate::gs::Param {
                                            ty: Type::Object(id.range), // It takes a class
                                            name: crate::gs::Identifier {
                                                name: "cls".to_string(),
                                                range: id.range,
                                            },
                                            range: id.range,
                                        }],
                                        body: None,
                                        scope_id: 0,
                                        range: id.range,
                                    }])
                                } else if let EvaluatedType::Type(Type::Named(class_id)) =
                                    &current_type
                                {
                                    if let Some(class) = resolver.find_class(&class_id.name) {
                                        if let Some(field) =
                                            class.find_field(program, resolver, &id.name)
                                        {
                                            EvaluatedType::Type(field.ty.clone())
                                        } else if let Some(methods) =
                                            class.find_method(program, resolver, &id.name)
                                        {
                                            EvaluatedType::Methods(methods)
                                        } else {
                                            return Err(format!(
                                                "Member '{}' not found in class '{}'",
                                                id.name, class_id.name
                                            ));
                                        }
                                    } else {
                                        return Err(format!("Class '{}' not found", class_id.name));
                                    }
                                } else {
                                    return Err("Cannot dereference generic 'object'".to_string());
                                }
                            }
                            EvaluatedType::Type(t) => {
                                return Err(format!("Cannot dereference primitive type '{}'", t));
                            }
                            _ => return Err("Cannot dereference void or method".to_string()),
                        };
                        current_type = ty;
                    }
                    PostfixOp::Call(_, _) => {
                        match current_type {
                            EvaluatedType::Methods(methods) => {
                                // For evaluation purposes, we take the return type of the first overload.
                                // In the diagnostics crate, we'll check all overloads.
                                if let Some(method) = methods.first() {
                                    match &method.return_type {
                                        TypeOrVoid::Type(t) => {
                                            current_type = EvaluatedType::Type(t.clone());
                                        }
                                        TypeOrVoid::Void(_) => {
                                            current_type = EvaluatedType::Void;
                                        }
                                    }
                                } else {
                                    return Err("No matching overloads found".to_string());
                                }
                            }
                            _ => return Err("Expression is not a method".to_string()),
                        }
                    }
                    PostfixOp::Index(indices, op_range) => match current_type {
                        EvaluatedType::Array(inner, size, _) => {
                            if indices.len() == 1 {
                                if let Some(size) = size {
                                    if let Some(Expr::Literal(Literal::Int(idx_val, _))) =
                                        indices.first()
                                    {
                                        if *idx_val as usize >= size {
                                            return Err(format!(
                                                "Array index out of range: {} >= {}",
                                                idx_val, size
                                            ));
                                        }
                                    }
                                }
                                current_type = match &inner {
                                    Type::Array(nested_inner, nested_range) => {
                                        EvaluatedType::Array(
                                            *nested_inner.clone(),
                                            None,
                                            *nested_range,
                                        )
                                    }
                                    _ => EvaluatedType::Type(inner),
                                };
                            } else if indices.len() == 2 {
                                current_type = EvaluatedType::Array(inner, None, *op_range);
                            } else {
                                return Err("Invalid number of indices".to_string());
                            }
                        }
                        EvaluatedType::Type(Type::String(_)) => {
                            current_type = EvaluatedType::Type(Type::String(*op_range));
                        }
                        EvaluatedType::Type(Type::Array(inner, _)) => {
                            if indices.len() == 1 {
                                current_type = match &*inner {
                                    Type::Array(nested_inner, nested_range) => {
                                        EvaluatedType::Array(
                                            *nested_inner.clone(),
                                            None,
                                            *nested_range,
                                        )
                                    }
                                    _ => EvaluatedType::Type(*inner),
                                };
                            } else if indices.len() == 2 {
                                current_type = EvaluatedType::Array(*inner, None, *op_range);
                            } else {
                                return Err("Invalid number of indices".to_string());
                            }
                        }
                        _ => return Err("Expression is not an array or string".to_string()),
                    },
                    PostfixOp::Unary(_, _) => {}
                }
            }
            Ok(current_type)
        }
        Expr::Grouped(expr, _) => evaluate_expr_type(
            expr,
            program,
            resolver,
            pos,
            current_class,
            known_array_sizes,
        ),
        Expr::Cast { ty, .. } => Ok(EvaluatedType::Type(ty.clone())),
        Expr::NewObject { ty, .. } => Ok(EvaluatedType::Type(ty.clone())),
        Expr::NewArray { ty, size, .. } => {
            let size_val = if let Expr::Literal(Literal::Int(v, _)) = &**size {
                Some(*v as usize)
            } else {
                None
            };
            Ok(EvaluatedType::Array(ty.clone(), size_val, expr.range()))
        }
        Expr::BinaryMath { left, .. } => evaluate_expr_type(
            left,
            program,
            resolver,
            pos,
            current_class,
            known_array_sizes,
        ),
        Expr::Assign { left, .. } => evaluate_expr_type(
            left,
            program,
            resolver,
            pos,
            current_class,
            known_array_sizes,
        ),
        Expr::LogicalOr { range, .. }
        | Expr::LogicalAnd { range, .. }
        | Expr::Equality { range, .. }
        | Expr::Comparison { range, .. }
        | Expr::IsClass(crate::gs::Identifier { range, .. }) => {
            Ok(EvaluatedType::Type(Type::Bool(*range)))
        }
        Expr::Bitwise { left, .. } => evaluate_expr_type(
            left,
            program,
            resolver,
            pos,
            current_class,
            known_array_sizes,
        ),
        Expr::Unary { expr, .. } => evaluate_expr_type(
            expr,
            program,
            resolver,
            pos,
            current_class,
            known_array_sizes,
        ),
    }
}

pub fn is_compatible(
    source: &Type,
    target: &Type,
    program: &Program,
    resolver: &dyn ClassResolver,
) -> bool {
    // 1. Exact match
    if format!("{}", source) == format!("{}", target) {
        return true;
    }

    // 2. Object compatibility
    match (source, target) {
        (Type::Named(src_id), Type::Named(tgt_id)) => {
            // Check inheritance: src must be a subclass of tgt
            if let Some(src_class) = resolver.find_class(&src_id.name) {
                return src_class.is_subclass_of(&tgt_id.name, program, resolver);
            }
        }
        (Type::Object(_), Type::Object(_)) => return true,
        (Type::Named(_), Type::Object(_)) => return true, // Any class is an object
        (Type::Object(_), Type::Named(_)) => return true, // null (generic object) can be assigned to any class
        _ => {}
    }

    false
}
