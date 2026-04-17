use crate::find::HasRange;
use crate::gs::find::{find_class_at_position, find_method_at_position};
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
                if let Some(m) = methods.first()
                    && let TypeOrVoid::Type(t) = &m.return_type
                {
                    return Some(t.clone());
                }
                None
            }
            _ => None,
        }
    }
}

pub fn is_type_compatible(expected: &Type, actual: &Type, resolver: &dyn ClassResolver) -> bool {
    match (expected, actual) {
        (Type::Bool(_), Type::Bool(_)) => true,
        (Type::Int(_), Type::Int(_)) => true,
        (Type::Float(_), Type::Float(_)) => true,
        (Type::String(_), Type::String(_)) => true,
        (Type::Object(_), _) => true, // Everything inherits from object
        (_, Type::Object(_)) => true, // null (generic object) can be assigned to anything
        (Type::Named(e_id), Type::Named(a_id)) => {
            if e_id.name == a_id.name {
                return true;
            }
            // Check inheritance: actual must be a subclass of expected
            if let Some(actual_class) = resolver.find_class(&a_id.name) {
                return actual_class.is_subclass_of(&e_id.name, resolver);
            }
            false
        }
        (Type::Array(e_inner, _), Type::Array(a_inner, _)) => {
            is_type_compatible(e_inner, a_inner, resolver)
        }
        // Implicit casts
        (Type::Float(_), Type::Int(_)) => true, // int to float
        (Type::Int(_), Type::Float(_)) => true, // float to int (with warning)
        (Type::Bool(_), _) => true,             // Anything can be cast to bool
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
                if id.name == "inherited" {
                    if let Some(current_method) = find_method_at_position(program, pos) {
                        let mut all_parent_methods = Vec::new();
                        for super_id in &class.superclasses {
                            if let Some(super_class) = resolver.find_class(&super_id.name) {
                                if let Some(methods) =
                                    super_class.find_method(resolver, &current_method.name.name)
                                {
                                    all_parent_methods.extend(methods);
                                }
                            } else {
                                return Err(format!(
                                    "Inherited class '{}' not found",
                                    super_id.name
                                ));
                            }
                        }
                        if all_parent_methods.is_empty() {
                            return Err(format!(
                                "Method '{}' not defined in any inherited class",
                                current_method.name.name
                            ));
                        }
                        return Ok(EvaluatedType::Methods(all_parent_methods));
                    }
                    return Err("Cannot use 'inherited' outside of a method".to_string());
                }

                if let Some(field) = class.find_field(resolver, &id.name) {
                    return match &field.ty {
                        Type::Array(inner, range) => {
                            let size = known_array_sizes.get(&id.name).cloned();
                            Ok(EvaluatedType::Array(*inner.clone(), size, *range))
                        }
                        _ => Ok(EvaluatedType::Type(field.ty.clone())),
                    };
                }
                if let Some(methods) = class.find_method(resolver, &id.name) {
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
                            EvaluatedType::Array(inner, _, range) => {
                                let inner_type = inner.clone();
                                let range = *range;

                                if id.name == "size" {
                                    EvaluatedType::Methods(vec![crate::gs::MethodDef {
                                        parent_class: None,
                                        modifiers: vec![],
                                        return_type: TypeOrVoid::Type(Type::Int(id.range)),
                                        name: crate::gs::Identifier {
                                            name: "size".to_string(),
                                            range: id.range,
                                        },
                                        params: vec![],
                                        void_param_range: None,
                                        body: None,
                                        scope_id: 0,
                                        range: id.range,
                                    }])
                                } else if id.name == "copy" {
                                    EvaluatedType::Methods(vec![crate::gs::MethodDef {
                                        parent_class: None,
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
                                        void_param_range: None,
                                        body: None,
                                        scope_id: 0,
                                        range: id.range,
                                    }])
                                } else {
                                    return Err(format!("Member '{}' not found in array", id.name));
                                }
                            }
                            EvaluatedType::Type(t) => {
                                if id.name == "isclass" {
                                    // isclass is a special method available on all objects/named types
                                    // But GS also allows it on some other things? Let's check.
                                    match t {
                                        Type::Named(_) | Type::Object(_) => {
                                            EvaluatedType::Methods(vec![crate::gs::MethodDef {
                                                parent_class: None,
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
                                                void_param_range: None,
                                                body: None,
                                                scope_id: 0,
                                                range: id.range,
                                            }])
                                        }
                                        _ => {
                                            return Err(format!(
                                                "Cannot dereference primitive type '{}'",
                                                t
                                            ));
                                        }
                                    }
                                } else {
                                    match t {
                                        Type::Array(inner, range) => {
                                            let inner_type = (**inner).clone();
                                            let range = *range;

                                            if id.name == "size" {
                                                EvaluatedType::Methods(vec![crate::gs::MethodDef {
                                                    parent_class: None,
                                                    modifiers: vec![],
                                                    return_type: TypeOrVoid::Type(Type::Int(
                                                        id.range,
                                                    )),
                                                    name: crate::gs::Identifier {
                                                        name: "size".to_string(),
                                                        range: id.range,
                                                    },
                                                    params: vec![],
                                                    void_param_range: None,
                                                    body: None,
                                                    scope_id: 0,
                                                    range: id.range,
                                                }])
                                            } else if id.name == "copy" {
                                                EvaluatedType::Methods(vec![crate::gs::MethodDef {
                                                    parent_class: None,
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
                                                    void_param_range: None,
                                                    body: None,
                                                    scope_id: 0,
                                                    range: id.range,
                                                }])
                                            } else {
                                                return Err(format!(
                                                    "Member '{}' not found in array",
                                                    id.name
                                                ));
                                            }
                                        }
                                        Type::String(_) => {
                                            if id.name == "size" {
                                                EvaluatedType::Methods(vec![crate::gs::MethodDef {
                                                    parent_class: None,
                                                    modifiers: vec![],
                                                    return_type: TypeOrVoid::Type(Type::Int(
                                                        id.range,
                                                    )),
                                                    name: crate::gs::Identifier {
                                                        name: "size".to_string(),
                                                        range: id.range,
                                                    },
                                                    params: vec![],
                                                    void_param_range: None,
                                                    body: None,
                                                    scope_id: 0,
                                                    range: id.range,
                                                }])
                                            } else {
                                                return Err(format!(
                                                    "Member '{}' not found in string",
                                                    id.name
                                                ));
                                            }
                                        }
                                        Type::Named(class_id) => {
                                            if let Some(class) = resolver.find_class(&class_id.name)
                                            {
                                                if let Some(field) =
                                                    class.find_field(resolver, &id.name)
                                                {
                                                    EvaluatedType::Type(field.ty.clone())
                                                } else if let Some(methods) =
                                                    class.find_method(resolver, &id.name)
                                                {
                                                    EvaluatedType::Methods(methods)
                                                } else {
                                                    return Err(format!(
                                                        "Member '{}' not found in class '{}'",
                                                        id.name, class_id.name
                                                    ));
                                                }
                                            } else {
                                                return Err(format!(
                                                    "Class '{}' not found",
                                                    class_id.name
                                                ));
                                            }
                                        }
                                        Type::Object(_) => {
                                            return Err(
                                                "Cannot dereference generic 'object'".to_string()
                                            );
                                        }
                                        _ => {
                                            return Err(format!(
                                                "Cannot dereference primitive type '{}'",
                                                t
                                            ));
                                        }
                                    }
                                }
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
                                if let Some(size) = size
                                    && let Some(Expr::Literal(Literal::Int(idx_val, _))) =
                                        indices.first()
                                    && *idx_val as usize >= size
                                {
                                    return Err(format!(
                                        "Array index out of range: {} >= {}",
                                        idx_val, size
                                    ));
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
        Expr::Bitwise { range, .. } => Ok(EvaluatedType::Type(Type::Int(*range))),
        Expr::Unary {
            expr, op, range, ..
        } => match op {
            crate::gs::UnaryPrefixOp::Not | crate::gs::UnaryPrefixOp::NotNot => {
                Ok(EvaluatedType::Type(Type::Bool(*range)))
            }
            crate::gs::UnaryPrefixOp::Inverse => Ok(EvaluatedType::Type(Type::Int(*range))),
            _ => evaluate_expr_type(
                expr,
                program,
                resolver,
                pos,
                current_class,
                known_array_sizes,
            ),
        },
    }
}

pub fn is_compatible(source: &Type, target: &Type, resolver: &dyn ClassResolver) -> bool {
    // 1. Exact match
    if format!("{}", source) == format!("{}", target) {
        return true;
    }

    // 2. Object compatibility
    match (source, target) {
        (Type::Named(src_id), Type::Named(tgt_id)) => {
            // Check inheritance: src must be a subclass of tgt
            if let Some(src_class) = resolver.find_class(&src_id.name) {
                return src_class.is_subclass_of(&tgt_id.name, resolver);
            }
        }
        (Type::Object(_), _) => return true,
        (_, Type::Object(_)) => return true, // null (generic object) can be assigned to any class
        // Implicit casts
        (Type::Float(_), Type::Int(_)) => return true, // int to float
        (Type::Int(_), Type::Float(_)) => return true, // float to int
        (Type::Bool(_), _) => return true,             // Anything can be cast to bool
        _ => {}
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gs::process::process_trainz_ast;
    use pest::Parser;
    use trainz_parser::gs::grammar::{GameScriptParser, Rule};

    fn parse_gs(src: &str) -> crate::gs::Program {
        let pairs = GameScriptParser::parse(Rule::program, src)
            .unwrap_or_else(|e| panic!("Parse failed: {}", e));
        process_trainz_ast(pairs, src)
    }

    #[test]
    fn test_evaluate_string_size() {
        let src = "class Test {
    void Main() {
        string s = \"test\";
        s.size();
    }
};";
        let program = parse_gs(src);
        let class = program.classes.values().next().unwrap();
        let main_method = &class.methods.get("Main").unwrap()[0];
        let stmt = &main_method.body.as_ref().unwrap().statements[1];
        if let crate::gs::Stmt::Expr(expr) = stmt {
            let res = evaluate_expr_type(
                expr,
                &program,
                &program,
                Position::new(3, 12),
                Some(class),
                &std::collections::HashMap::new(),
            );
            assert!(res.is_ok());
            let ty = res.unwrap();
            if let EvaluatedType::Type(Type::Int(_)) = ty {
                // Success
            } else {
                panic!("Expected EvaluatedType::Type(Type::Int), got {:?}", ty);
            }
        } else {
            panic!("Expected Statement::Expr, got {:?}", stmt);
        }
    }

    #[test]
    fn test_evaluate_string_literal_size() {
        let src = "class Test {
    void Main() {
        \"test\".size();
    }
};";
        let program = parse_gs(src);
        let class = program.classes.values().next().unwrap();
        let main_method = &class.methods.get("Main").unwrap()[0];
        let stmt = &main_method.body.as_ref().unwrap().statements[0];
        if let crate::gs::Stmt::Expr(expr) = stmt {
            let res = evaluate_expr_type(
                expr,
                &program,
                &program,
                Position::new(3, 12),
                Some(class),
                &std::collections::HashMap::new(),
            );
            assert!(res.is_ok());
            let ty = res.unwrap();
            if let EvaluatedType::Type(Type::Int(_)) = ty {
                // Success
            } else {
                panic!("Expected EvaluatedType::Type(Type::Int), got {:?}", ty);
            }
        } else {
            panic!("Expected Statement::Expr, got {:?}", stmt);
        }
    }

    #[test]
    fn test_evaluate_string_method_return_size() {
        let src = "class Test {
    string GetName() { return \"test\"; }
    void Main() {
        GetName().size();
    }
};";
        let program = parse_gs(src);
        let class = program.classes.values().next().unwrap();
        let main_method = class.methods.get("Main").unwrap().first().unwrap();
        let stmt = &main_method.body.as_ref().unwrap().statements[0];
        if let crate::gs::Stmt::Expr(expr) = stmt {
            let res = evaluate_expr_type(
                expr,
                &program,
                &program,
                Position::new(4, 12),
                Some(class),
                &std::collections::HashMap::new(),
            );
            assert!(res.is_ok(), "Expected Ok, got {:?}", res);
        } else {
            panic!("Expected Statement::Expr, got {:?}", stmt);
        }
    }

    #[test]
    fn test_evaluate_string_field_size() {
        let src = "class Test {
    string name;
    void Main() {
        name.size();
    }
};";
        let program = parse_gs(src);
        let class = program.classes.values().next().unwrap();
        let main_method = class.methods.get("Main").unwrap().first().unwrap();
        let stmt = &main_method.body.as_ref().unwrap().statements[0];
        if let crate::gs::Stmt::Expr(expr) = stmt {
            let res = evaluate_expr_type(
                expr,
                &program,
                &program,
                Position::new(4, 12),
                Some(class),
                &std::collections::HashMap::new(),
            );
            assert!(res.is_ok(), "Expected Ok, got {:?}", res);
        } else {
            panic!("Expected Statement::Expr, got {:?}", stmt);
        }
    }

    #[test]
    fn test_evaluate_string_param_size() {
        let src = "class Test {
    void Main(string s) {
        s.size();
    }
};";
        let program = parse_gs(src);
        let class = program.classes.values().next().unwrap();
        let main_method = class.methods.get("Main").unwrap().first().unwrap();
        let stmt = &main_method.body.as_ref().unwrap().statements[0];
        if let crate::gs::Stmt::Expr(expr) = stmt {
            let res = evaluate_expr_type(
                expr,
                &program,
                &program,
                Position::new(2, 10),
                Some(class),
                &std::collections::HashMap::new(),
            );
            assert!(res.is_ok(), "Expected Ok, got {:?}", res);
        } else {
            panic!("Expected Statement::Expr, got {:?}", stmt);
        }
    }

    #[test]
    fn test_evaluate_string_index_size() {
        let src = "class Test {
    void Main() {
        string s = \"test\";
        s[0].size();
    }
};";
        let program = parse_gs(src);
        let class = program.classes.values().next().unwrap();
        let main_method = class.methods.get("Main").unwrap().first().unwrap();
        let stmt = &main_method.body.as_ref().unwrap().statements[1];
        if let crate::gs::Stmt::Expr(expr) = stmt {
            let res = evaluate_expr_type(
                expr,
                &program,
                &program,
                Position::new(3, 14),
                Some(class),
                &std::collections::HashMap::new(),
            );
            assert!(res.is_ok(), "Expected Ok, got {:?}", res);
        } else {
            panic!("Expected Statement::Expr, got {:?}", stmt);
        }
    }

    #[test]
    fn test_evaluate_string_invalid_member() {
        let src = "class Test {
    void Main() {
        string s = \"test\";
        s.foo;
    }
};";
        let program = parse_gs(src);
        let class = program.classes.values().next().unwrap();
        let main_method = class.methods.get("Main").unwrap().first().unwrap();
        let stmt = &main_method.body.as_ref().unwrap().statements[1];
        if let crate::gs::Stmt::Expr(expr) = stmt {
            let res = evaluate_expr_type(
                expr,
                &program,
                &program,
                Position::new(3, 10),
                Some(class),
                &std::collections::HashMap::new(),
            );
            assert!(res.is_err());
            assert_eq!(res.err().unwrap(), "Member 'foo' not found in string");
        } else {
            panic!("Expected Statement::Expr, got {:?}", stmt);
        }
    }

    #[test]
    fn test_evaluate_bit_shifts() {
        let src = "class Test {
    void Main() {
        int a = 1 << 2;
        int b = 4 >> 1;
    }
};";
        let program = parse_gs(src);
        let class = program.classes.values().next().unwrap();
        let main_method = class.methods.get("Main").unwrap().first().unwrap();

        // Check <<
        let stmt1 = &main_method.body.as_ref().unwrap().statements[0];
        if let crate::gs::Stmt::Decl(decl) = stmt1 {
            let res = evaluate_expr_type(
                &decl.values[0],
                &program,
                &program,
                Position::new(3, 17),
                Some(class),
                &std::collections::HashMap::new(),
            );
            assert!(res.is_ok());
            if let EvaluatedType::Type(Type::Int(_)) = res.unwrap() {
                // Success
            } else {
                panic!("Expected int type for shift left");
            }
        }

        // Check >>
        let stmt2 = &main_method.body.as_ref().unwrap().statements[1];
        if let crate::gs::Stmt::Decl(decl) = stmt2 {
            let res = evaluate_expr_type(
                &decl.values[0],
                &program,
                &program,
                Position::new(4, 17),
                Some(class),
                &std::collections::HashMap::new(),
            );
            assert!(res.is_ok());
            if let EvaluatedType::Type(Type::Int(_)) = res.unwrap() {
                // Success
            } else {
                panic!("Expected int type for shift right");
            }
        }
    }
}
