use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use std::collections::HashMap;
use std::str::FromStr;
use tower_lsp_server::ls_types::{
    CodeDescription, Diagnostic, DiagnosticSeverity, NumberOrString, Uri,
};
use tracing::debug;
use trainz_ast::find::HasRange;
use trainz_ast::gs::dependency_graph;
use trainz_ast::gs::program::Program;
use trainz_ast::gs::stmt::{Block, Stmt};
use trainz_ast::gs::type_eval;
use trainz_ast::gs::types::TypeOrVoid;
use trainz_ast::gs::{ClassDef, Expr, MethodDef};

#[tracing::instrument(skip(resolver, program_resolver))]
pub fn trainz_diagnostics(
    path: &str,
    program: &Program,
    resolver: &dyn type_eval::ClassResolver,
    program_resolver: &dyn dependency_graph::ProgramResolver,
) -> Vec<Diagnostic> {
    debug!(
        "Include paths: {:?}",
        program
            .includes
            .iter()
            .map(|i| i.name.clone())
            .collect::<Vec<_>>()
    );
    let mut diagnostics = vec![];

    // Check cyclic includes
    diagnostics.extend(
        dependency_graph::find_cyclic_includes(path, program, program_resolver)
            .into_iter()
            .map(|include| Diagnostic {
                range: include.range,
                severity: Some(DiagnosticSeverity::WARNING),
                code: Some(NumberOrString::Number(1)),
                code_description: None,
                source: Some(String::from("game-script lsp")),
                message: format!("Cyclic include detected: {}", include.name),
                related_information: None,
                tags: None,
                data: None,
            }),
    );

    // Check missing includes
    diagnostics.extend(
        program
            .includes
            .par_iter()
            .filter(|include| include.path.is_none())
            .map(|include| {
                debug!("Include not found: {:?}", include.name);
                Diagnostic {
                    range: include.range,
                    severity: Some(DiagnosticSeverity::WARNING),
                    code: Some(NumberOrString::Number(0)),
                    code_description: if let Ok(href) = Uri::from_str(
                        "https://online.ts2009.com/mediaWiki/index.php/TrainzScript_Keywords#include",
                    ) {
                        Some(CodeDescription { href })
                    } else {
                        None
                    },
                    source: Some(String::from("game-script lsp")),
                    message: format!("File not found: {}", include.name),
                    related_information: None,
                    tags: None,
                    data: None,
                }
            })
            .collect::<Vec<Diagnostic>>(),
    );

    // Check classes and methods
    let class_diagnostics: Vec<Diagnostic> = program
        .classes
        .par_iter()
        .flat_map(|(_, class)| {
            let mut local_diagnostics = Vec::new();
            let mut class_array_sizes = HashMap::new();
            for field in class.fields.values() {
                if let Some(init) = &field.initializer {
                    if let Ok(type_eval::EvaluatedType::Array(_, Some(size), _)) =
                        type_eval::evaluate_expr_type(
                            init,
                            program,
                            resolver,
                            init.range().start,
                            Some(class),
                            &HashMap::new(),
                        )
                    {
                        class_array_sizes.insert(field.name.name.clone(), size);
                    }
                    check_expr(
                        init,
                        program,
                        resolver,
                        Some(class),
                        &mut local_diagnostics,
                        &class_array_sizes,
                    );
                }
            }
            for methods in class.methods.values() {
                for method in methods {
                    if let Some(body) = &method.body {
                        check_block(
                            body,
                            program,
                            resolver,
                            Some(class),
                            &mut local_diagnostics,
                            class_array_sizes.clone(),
                            Some(&method.return_type),
                        );
                    }
                }
            }
            local_diagnostics
        })
        .collect();
    diagnostics.extend(class_diagnostics);

    diagnostics
}

fn is_method_compatible(
    method: &MethodDef,
    args: &[Expr],
    arg_types: &[Option<trainz_ast::gs::Type>],
    program: &Program,
    resolver: &dyn type_eval::ClassResolver,
) -> bool {
    if method.params.len() != args.len() {
        return false;
    }

    for (param, arg_ty) in method.params.iter().zip(arg_types.iter()) {
        if let Some(actual_ty) = arg_ty {
            if !type_eval::is_type_compatible(&param.ty, actual_ty, program, resolver) {
                return false;
            }
        } else {
            return false;
        }
    }

    true
}

fn check_block(
    block: &Block,
    program: &Program,
    resolver: &dyn type_eval::ClassResolver,
    class: Option<&ClassDef>,
    diagnostics: &mut Vec<Diagnostic>,
    mut array_sizes: HashMap<String, usize>,
    expected_return_type: Option<&TypeOrVoid>,
) {
    for stmt in &block.statements {
        check_stmt(
            stmt,
            program,
            resolver,
            class,
            diagnostics,
            &mut array_sizes,
            expected_return_type,
        );
    }
}

fn check_stmt(
    stmt: &Stmt,
    program: &Program,
    resolver: &dyn type_eval::ClassResolver,
    class: Option<&ClassDef>,
    diagnostics: &mut Vec<Diagnostic>,
    array_sizes: &mut HashMap<String, usize>,
    expected_return_type: Option<&TypeOrVoid>,
) {
    match stmt {
        Stmt::Expr(expr) => check_expr(expr, program, resolver, class, diagnostics, array_sizes),
        Stmt::Decl(decl) => {
            for (id, val) in decl.names.iter().zip(decl.values.iter()) {
                check_expr(val, program, resolver, class, diagnostics, array_sizes);
                let val_ty = type_eval::evaluate_expr_type(
                    val,
                    program,
                    resolver,
                    val.range().start,
                    class,
                    array_sizes,
                );

                if let Ok(type_eval::EvaluatedType::Array(_, Some(size), _)) = &val_ty {
                    array_sizes.insert(id.name.clone(), *size);
                }

                if let Ok(eval_ty) = val_ty
                    && let Some(actual_ty) = eval_ty.to_type()
                {
                    if !type_eval::is_type_compatible(&decl.ty, &actual_ty, program, resolver) {
                        diagnostics.push(Diagnostic {
                            range: val.range(),
                            severity: Some(DiagnosticSeverity::ERROR),
                            message: format!(
                                "Assignment type mismatch: cannot assign '{}' to '{}'",
                                actual_ty, decl.ty
                            ),
                            ..Default::default()
                        });
                    } else if matches!(decl.ty, trainz_ast::gs::types::Type::Int(_))
                        && matches!(actual_ty, trainz_ast::gs::types::Type::Float(_))
                    {
                        diagnostics.push(Diagnostic {
                            range: val.range(),
                            severity: Some(DiagnosticSeverity::WARNING),
                            message: "Implicit cast from 'float' to 'int' may lose precision"
                                .to_string(),
                            ..Default::default()
                        });
                    }
                }
            }
        }
        Stmt::Return(expr, range, _) => {
            if let Some(expr) = expr {
                check_expr(expr, program, resolver, class, diagnostics, array_sizes);
            }

            if let Some(expected_ty) = expected_return_type {
                match (expr, expected_ty) {
                    (Some(e), TypeOrVoid::Type(t)) => {
                        if let Ok(eval_ty) = type_eval::evaluate_expr_type(
                            e,
                            program,
                            resolver,
                            e.range().start,
                            class,
                            array_sizes,
                        ) && let Some(actual_ty) = eval_ty.to_type()
                        {
                            if !type_eval::is_type_compatible(t, &actual_ty, program, resolver) {
                                diagnostics.push(Diagnostic {
                                    range: e.range(),
                                    severity: Some(DiagnosticSeverity::ERROR),
                                    message: format!(
                                        "Return type mismatch: expected '{}', got '{}'",
                                        t, actual_ty
                                    ),
                                    ..Default::default()
                                });
                            } else if matches!(t, trainz_ast::gs::types::Type::Int(_))
                                && matches!(actual_ty, trainz_ast::gs::types::Type::Float(_))
                            {
                                diagnostics.push(Diagnostic {
                                    range: e.range(),
                                    severity: Some(DiagnosticSeverity::WARNING),
                                    message:
                                        "Implicit cast from 'float' to 'int' may lose precision"
                                            .to_string(),
                                    ..Default::default()
                                });
                            }
                        }
                    }
                    (None, TypeOrVoid::Type(t)) => {
                        diagnostics.push(Diagnostic {
                            range: *range,
                            severity: Some(DiagnosticSeverity::ERROR),
                            message: format!("Return type mismatch: expected '{}', got nothing", t),
                            ..Default::default()
                        });
                    }
                    (Some(e), TypeOrVoid::Void(_)) => {
                        diagnostics.push(Diagnostic {
                            range: e.range(),
                            severity: Some(DiagnosticSeverity::ERROR),
                            message: "Return type mismatch: expected void, got an expression"
                                .to_string(),
                            ..Default::default()
                        });
                    }
                    (None, TypeOrVoid::Void(_)) => {}
                }
            }
        }
        Stmt::If(if_stmt) => {
            check_condition(
                &if_stmt.cond,
                program,
                resolver,
                class,
                diagnostics,
                array_sizes,
            );
            check_block(
                &if_stmt.then_block,
                program,
                resolver,
                class,
                diagnostics,
                array_sizes.clone(),
                expected_return_type,
            );
            if let Some(else_block) = &if_stmt.else_block {
                check_block(
                    else_block,
                    program,
                    resolver,
                    class,
                    diagnostics,
                    array_sizes.clone(),
                    expected_return_type,
                );
            }
        }
        Stmt::While(while_stmt) => {
            check_condition(
                &while_stmt.cond,
                program,
                resolver,
                class,
                diagnostics,
                array_sizes,
            );
            if let trainz_ast::gs::LoopBody::Block(b) = &while_stmt.body {
                check_block(
                    b,
                    program,
                    resolver,
                    class,
                    diagnostics,
                    array_sizes.clone(),
                    expected_return_type,
                )
            }
        }
        Stmt::For(for_stmt) => {
            // Track size in for init if it's an assignment
            check_expr(
                &for_stmt.init.target,
                program,
                resolver,
                class,
                diagnostics,
                array_sizes,
            );
            check_expr(
                &for_stmt.init.value,
                program,
                resolver,
                class,
                diagnostics,
                array_sizes,
            );
            if let trainz_ast::gs::Expr::Identifier(id) = &for_stmt.init.target
                && let Ok(type_eval::EvaluatedType::Array(_, Some(size), _)) =
                    type_eval::evaluate_expr_type(
                        &for_stmt.init.value,
                        program,
                        resolver,
                        for_stmt.init.value.range().start,
                        class,
                        array_sizes,
                    )
            {
                array_sizes.insert(id.name.clone(), size);
            }

            // For loop might update sizes, but usually not in init/cond/step
            check_condition(
                &for_stmt.cond,
                program,
                resolver,
                class,
                diagnostics,
                array_sizes,
            );
            if let Some(step) = &for_stmt.step {
                check_expr(step, program, resolver, class, diagnostics, array_sizes);
            }
            if let trainz_ast::gs::LoopBody::Block(b) = &for_stmt.body {
                check_block(
                    b,
                    program,
                    resolver,
                    class,
                    diagnostics,
                    array_sizes.clone(),
                    expected_return_type,
                )
            }
        }
        Stmt::Switch(switch_stmt) => {
            check_expr(
                &switch_stmt.expr,
                program,
                resolver,
                class,
                diagnostics,
                array_sizes,
            );
            for case in &switch_stmt.cases {
                check_expr(
                    &case.value,
                    program,
                    resolver,
                    class,
                    diagnostics,
                    array_sizes,
                );
                check_block(
                    &case.body,
                    program,
                    resolver,
                    class,
                    diagnostics,
                    array_sizes.clone(),
                    expected_return_type,
                );
            }
            if let Some(default) = &switch_stmt.default {
                check_block(
                    default,
                    program,
                    resolver,
                    class,
                    diagnostics,
                    array_sizes.clone(),
                    expected_return_type,
                );
            }
        }
        Stmt::Block(block) => check_block(
            block,
            program,
            resolver,
            class,
            diagnostics,
            array_sizes.clone(),
            expected_return_type,
        ),
        _ => {}
    }
}

fn find_best_overload<'a>(
    methods: &'a [MethodDef],
    args: &[Expr],
    arg_types: &[Option<trainz_ast::gs::types::Type>],
    program: &Program,
    resolver: &dyn type_eval::ClassResolver,
) -> Option<&'a MethodDef> {
    methods.iter().max_by_key(|m| {
        let mut score = 0;
        if m.params.len() == args.len() {
            score += 100;
            for (p, at) in m.params.iter().zip(arg_types.iter()) {
                if let Some(at) = at
                    && type_eval::is_type_compatible(&p.ty, at, program, resolver)
                {
                    score += 10;
                }
            }
        } else {
            score -= (m.params.len() as i32 - args.len() as i32).abs() * 10;
        }
        score
    })
}

fn check_expr(
    expr: &Expr,
    program: &Program,
    resolver: &dyn type_eval::ClassResolver,
    class: Option<&ClassDef>,
    diagnostics: &mut Vec<Diagnostic>,
    array_sizes: &HashMap<String, usize>,
) {
    let pos = expr.range().start;
    if let Err(msg) =
        type_eval::evaluate_expr_type(expr, program, resolver, pos, class, array_sizes)
    {
        diagnostics.push(Diagnostic {
            range: expr.range(),
            severity: Some(DiagnosticSeverity::ERROR),
            message: msg,
            ..Default::default()
        });
    }

    // Recurse into sub-expressions to find all errors
    match expr {
        Expr::Assign { left, right, .. } => {
            check_expr(left, program, resolver, class, diagnostics, array_sizes);
            check_expr(right, program, resolver, class, diagnostics, array_sizes);

            let left_ty =
                type_eval::evaluate_expr_type(left, program, resolver, pos, class, array_sizes);
            let right_ty =
                type_eval::evaluate_expr_type(right, program, resolver, pos, class, array_sizes);

            if let (Ok(l_eval), Ok(r_eval)) = (left_ty, right_ty)
                && let (Some(l_ty), Some(r_ty)) = (l_eval.to_type(), r_eval.to_type())
            {
                if !type_eval::is_type_compatible(&l_ty, &r_ty, program, resolver) {
                    diagnostics.push(Diagnostic {
                        range: right.range(),
                        severity: Some(DiagnosticSeverity::ERROR),
                        message: format!(
                            "Assignment type mismatch: cannot assign '{}' to '{}'",
                            r_ty, l_ty
                        ),
                        ..Default::default()
                    });
                } else if matches!(l_ty, trainz_ast::gs::types::Type::Int(_))
                    && matches!(r_ty, trainz_ast::gs::types::Type::Float(_))
                {
                    diagnostics.push(Diagnostic {
                        range: right.range(),
                        severity: Some(DiagnosticSeverity::WARNING),
                        message: "Implicit cast from 'float' to 'int' may lose precision"
                            .to_string(),
                        ..Default::default()
                    });
                }
            }
        }
        Expr::BinaryMath { left, right, .. } => {
            check_expr(left, program, resolver, class, diagnostics, array_sizes);
            check_expr(right, program, resolver, class, diagnostics, array_sizes);
        }
        Expr::Comparison { left, right, .. } => {
            check_expr(left, program, resolver, class, diagnostics, array_sizes);
            check_expr(right, program, resolver, class, diagnostics, array_sizes);
        }
        Expr::Equality { left, right, .. } => {
            check_expr(left, program, resolver, class, diagnostics, array_sizes);
            check_expr(right, program, resolver, class, diagnostics, array_sizes);
        }
        Expr::LogicalAnd { left, right, .. } => {
            check_expr(left, program, resolver, class, diagnostics, array_sizes);
            check_expr(right, program, resolver, class, diagnostics, array_sizes);
        }
        Expr::LogicalOr { left, right, .. } => {
            check_expr(left, program, resolver, class, diagnostics, array_sizes);
            check_expr(right, program, resolver, class, diagnostics, array_sizes);
        }
        Expr::Bitwise {
            left, right, op, ..
        } => {
            check_expr(left, program, resolver, class, diagnostics, array_sizes);
            check_expr(right, program, resolver, class, diagnostics, array_sizes);

            let left_ty =
                type_eval::evaluate_expr_type(left, program, resolver, pos, class, array_sizes);
            let right_ty =
                type_eval::evaluate_expr_type(right, program, resolver, pos, class, array_sizes);

            let op_name = match op {
                trainz_ast::gs::BitwiseOp::Shl | trainz_ast::gs::BitwiseOp::Shr => "Bit shift",
                _ => "Bitwise",
            };

            if let (Ok(l_eval), Ok(r_eval)) = (left_ty, right_ty) {
                if let Some(l_ty) = l_eval.to_type() {
                    if !matches!(l_ty, trainz_ast::gs::types::Type::Int(_)) {
                        diagnostics.push(Diagnostic {
                            range: left.range(),
                            severity: Some(DiagnosticSeverity::ERROR),
                            message: format!(
                                "{} operator requires 'int' operand, got '{}'",
                                op_name, l_ty
                            ),
                            ..Default::default()
                        });
                    }
                }
                if let Some(r_ty) = r_eval.to_type() {
                    if !matches!(r_ty, trainz_ast::gs::types::Type::Int(_)) {
                        diagnostics.push(Diagnostic {
                            range: right.range(),
                            severity: Some(DiagnosticSeverity::ERROR),
                            message: format!(
                                "{} operator requires 'int' operand, got '{}'",
                                op_name, r_ty
                            ),
                            ..Default::default()
                        });
                    }
                }
            }
        }
        Expr::Unary {
            expr: sub_expr, op, ..
        } => {
            check_expr(sub_expr, program, resolver, class, diagnostics, array_sizes);

            if matches!(op, trainz_ast::gs::UnaryPrefixOp::Inverse) {
                let ty = type_eval::evaluate_expr_type(
                    sub_expr,
                    program,
                    resolver,
                    pos,
                    class,
                    array_sizes,
                );
                if let Ok(eval) = ty
                    && let Some(t) = eval.to_type()
                {
                    if !matches!(t, trainz_ast::gs::types::Type::Int(_)) {
                        diagnostics.push(Diagnostic {
                            range: sub_expr.range(),
                            severity: Some(DiagnosticSeverity::ERROR),
                            message: format!(
                                "Bitwise NOT operator requires 'int' operand, got '{}'",
                                t
                            ),
                            ..Default::default()
                        });
                    }
                }
            }
        }
        Expr::Postfix {
            expr: sub_expr,
            ops,
            ..
        } => {
            check_expr(sub_expr, program, resolver, class, diagnostics, array_sizes);
            let mut current_type =
                type_eval::evaluate_expr_type(sub_expr, program, resolver, pos, class, array_sizes);

            for (i, op) in ops.iter().enumerate() {
                match op {
                    trainz_ast::gs::PostfixOp::Call(args, op_range) => {
                        for arg in args {
                            check_expr(arg, program, resolver, class, diagnostics, array_sizes);
                        }

                        if let Ok(type_eval::EvaluatedType::Methods(methods)) = &current_type {
                            let mut arg_types = Vec::new();
                            for arg in args {
                                arg_types.push(
                                    type_eval::evaluate_expr_type(
                                        arg,
                                        program,
                                        resolver,
                                        pos,
                                        class,
                                        array_sizes,
                                    )
                                    .ok()
                                    .and_then(|t| t.to_type()),
                                );
                            }

                            let is_inherited = if let Expr::Identifier(id) = &**sub_expr {
                                id.name == "inherited"
                            } else {
                                false
                            };

                            if is_inherited {
                                let mut methods_by_class: HashMap<Option<String>, Vec<&MethodDef>> =
                                    HashMap::new();
                                for method in methods {
                                    methods_by_class
                                        .entry(method.parent_class.clone())
                                        .or_default()
                                        .push(method);
                                }

                                for (class_name, class_methods) in methods_by_class {
                                    let mut match_found = false;
                                    let mut perfect_match_method = None;
                                    for method in &class_methods {
                                        if is_method_compatible(
                                            method, args, &arg_types, program, resolver,
                                        ) {
                                            match_found = true;
                                            perfect_match_method = Some(method);
                                            break;
                                        }
                                    }

                                    if !match_found {
                                        if let Some(best) = find_best_overload(
                                            &class_methods
                                                .iter()
                                                .map(|m| (*m).clone())
                                                .collect::<Vec<_>>(),
                                            args,
                                            &arg_types,
                                            program,
                                            resolver,
                                        ) {
                                            let suggestion = format!(
                                                "{}({})",
                                                best.name.name,
                                                best.params
                                                    .iter()
                                                    .map(|p| p.ty.to_string())
                                                    .collect::<Vec<_>>()
                                                    .join(", ")
                                            );

                                            diagnostics.push(Diagnostic {
                                                range: *op_range,
                                                severity: Some(DiagnosticSeverity::WARNING),
                                                message: format!(
                                                    "Arguments not compatible with 'inherited' method in class '{}'. Closest match: {}",
                                                    class_name.unwrap_or_else(|| "unknown".to_string()),
                                                    suggestion
                                                ),
                                                ..Default::default()
                                            });
                                        } else {
                                            diagnostics.push(Diagnostic {
                                                range: *op_range,
                                                severity: Some(DiagnosticSeverity::WARNING),
                                                message: format!(
                                                    "Arguments not compatible with 'inherited' method in class '{}'",
                                                    class_name.unwrap_or_else(|| "unknown".to_string())
                                                ),
                                                ..Default::default()
                                            });
                                        }
                                    } else if let Some(method) = perfect_match_method {
                                        // Check for risky casts in arguments
                                        for ((param, arg), arg_ty) in method
                                            .params
                                            .iter()
                                            .zip(args.iter())
                                            .zip(arg_types.iter())
                                        {
                                            if let Some(actual_ty) = arg_ty {
                                                if matches!(
                                                    param.ty,
                                                    trainz_ast::gs::types::Type::Int(_)
                                                ) && matches!(
                                                    actual_ty,
                                                    trainz_ast::gs::types::Type::Float(_)
                                                ) {
                                                    diagnostics.push(Diagnostic {
                                                        range: arg.range(),
                                                        severity: Some(DiagnosticSeverity::WARNING),
                                                        message: "Implicit cast from 'float' to 'int' may lose precision"
                                                            .to_string(),
                                                        ..Default::default()
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                let mut perfect_match_method = None;
                                for method in methods {
                                    if is_method_compatible(
                                        method, args, &arg_types, program, resolver,
                                    ) {
                                        perfect_match_method = Some(method);
                                        break;
                                    }
                                }

                                if let Some(method) = perfect_match_method {
                                    // Check for risky casts in arguments
                                    for ((param, arg), arg_ty) in
                                        method.params.iter().zip(args.iter()).zip(arg_types.iter())
                                    {
                                        if let Some(actual_ty) = arg_ty {
                                            if matches!(
                                                param.ty,
                                                trainz_ast::gs::types::Type::Int(_)
                                            ) && matches!(
                                                actual_ty,
                                                trainz_ast::gs::types::Type::Float(_)
                                            ) {
                                                diagnostics.push(Diagnostic {
                                                    range: arg.range(),
                                                    severity: Some(DiagnosticSeverity::WARNING),
                                                    message: "Implicit cast from 'float' to 'int' may lose precision"
                                                        .to_string(),
                                                    ..Default::default()
                                                });
                                            }
                                        }
                                    }
                                } else {
                                    if let Some(best) = find_best_overload(
                                        methods, args, &arg_types, program, resolver,
                                    ) {
                                        let suggestion = format!(
                                            "{}({})",
                                            best.name.name,
                                            best.params
                                                .iter()
                                                .map(|p| p.ty.to_string())
                                                .collect::<Vec<_>>()
                                                .join(", ")
                                        );

                                        diagnostics.push(Diagnostic {
                                            range: *op_range,
                                            severity: Some(DiagnosticSeverity::WARNING),
                                            message: format!(
                                            "No matching overload of method takes {} arguments with these types. Closest match: {}",
                                            args.len(),
                                            suggestion
                                        ),
                                            ..Default::default()
                                        });
                                    } else {
                                        diagnostics.push(Diagnostic {
                                            range: *op_range,
                                            severity: Some(DiagnosticSeverity::ERROR),
                                            message: format!(
                                                "No overload of method takes {} arguments",
                                                args.len()
                                            ),
                                            ..Default::default()
                                        });
                                    }
                                }
                            }
                        }
                    }
                    trainz_ast::gs::PostfixOp::Index(indices, _) => {
                        for idx in indices {
                            check_expr(idx, program, resolver, class, diagnostics, array_sizes);
                        }
                    }
                    _ => {}
                }

                // Update current_type for next op in chain
                if current_type.is_ok() {
                    let temp_expr = Expr::Postfix {
                        expr: sub_expr.clone(),
                        ops: ops[..=i].to_vec(),
                        range: expr.range(),
                    };
                    current_type = type_eval::evaluate_expr_type(
                        &temp_expr,
                        program,
                        resolver,
                        pos,
                        class,
                        array_sizes,
                    );
                }
            }
        }
        Expr::Cast { expr, .. } => {
            check_expr(expr, program, resolver, class, diagnostics, array_sizes);
        }
        Expr::NewObject { args, .. } => {
            for arg in args {
                check_expr(arg, program, resolver, class, diagnostics, array_sizes);
            }
        }
        Expr::NewArray { size, .. } => {
            check_expr(size, program, resolver, class, diagnostics, array_sizes);
        }
        Expr::Grouped(expr, _) => {
            check_expr(expr, program, resolver, class, diagnostics, array_sizes);
        }
        _ => {}
    }
}

fn check_condition(
    expr: &Expr,
    program: &Program,
    resolver: &dyn type_eval::ClassResolver,
    class: Option<&ClassDef>,
    diagnostics: &mut Vec<Diagnostic>,
    array_sizes: &HashMap<String, usize>,
) {
    check_expr(expr, program, resolver, class, diagnostics, array_sizes);

    if let Ok(eval_ty) = type_eval::evaluate_expr_type(
        expr,
        program,
        resolver,
        expr.range().start,
        class,
        array_sizes,
    ) && let Some(actual_ty) = eval_ty.to_type()
    {
        let expected_ty = trainz_ast::gs::types::Type::Bool(expr.range());
        if !type_eval::is_type_compatible(&expected_ty, &actual_ty, program, resolver) {
            diagnostics.push(Diagnostic {
                range: expr.range(),
                severity: Some(DiagnosticSeverity::ERROR),
                message: format!(
                    "Condition type mismatch: expected 'bool', got '{}'",
                    actual_ty
                ),
                ..Default::default()
            });
        }
    }
}

#[cfg(test)]
mod tests;
