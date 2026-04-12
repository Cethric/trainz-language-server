use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use std::collections::HashMap;
use std::str::FromStr;
use tower_lsp_server::ls_types::{
    CodeDescription, Diagnostic, DiagnosticSeverity, NumberOrString, Uri,
};
use tracing::debug;
use trainz_ast::find::HasRange;
use trainz_ast::gs::program::Program;
use trainz_ast::gs::stmt::{Block, Stmt};
use trainz_ast::gs::type_eval;
use trainz_ast::gs::types::TypeOrVoid;
use trainz_ast::gs::{ClassDef, Expr};

#[tracing::instrument(skip(resolver))]
pub fn trainz_diagnostics(
    program: &Program,
    resolver: &dyn type_eval::ClassResolver,
) -> Vec<Diagnostic> {
    debug!("Include paths: {:?}", program.includes);
    let mut diagnostics = vec![];

    // Check missing includes
    diagnostics.extend(
        program
            .includes
            .par_iter()
            .filter(|include| include.path.is_none())
            .map(|include| {
                debug!("Include not found: {:?}", include);
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
    for class in program.classes.values() {
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
                    &mut diagnostics,
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
                        &mut diagnostics,
                        class_array_sizes.clone(),
                        Some(&method.return_type),
                    );
                }
            }
        }
    }

    diagnostics
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

                if let Ok(eval_ty) = val_ty {
                    if let Some(actual_ty) = eval_ty.to_type() {
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
                        }
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
                        ) {
                            if let Some(actual_ty) = eval_ty.to_type() {
                                if !type_eval::is_type_compatible(t, &actual_ty, program, resolver)
                                {
                                    diagnostics.push(Diagnostic {
                                        range: e.range(),
                                        severity: Some(DiagnosticSeverity::ERROR),
                                        message: format!(
                                            "Return type mismatch: expected '{}', got '{}'",
                                            t, actual_ty
                                        ),
                                        ..Default::default()
                                    });
                                }
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
            check_expr(
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
            check_expr(
                &while_stmt.cond,
                program,
                resolver,
                class,
                diagnostics,
                array_sizes,
            );
            match &while_stmt.body {
                trainz_ast::gs::LoopBody::Block(b) => check_block(
                    b,
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
            if let trainz_ast::gs::Expr::Identifier(id) = &for_stmt.init.target {
                if let Ok(type_eval::EvaluatedType::Array(_, Some(size), _)) =
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
            }

            // For loop might update sizes, but usually not in init/cond/step
            check_expr(
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
            match &for_stmt.body {
                trainz_ast::gs::LoopBody::Block(b) => check_block(
                    b,
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

            if let (Ok(l_eval), Ok(r_eval)) = (left_ty, right_ty) {
                if let (Some(l_ty), Some(r_ty)) = (l_eval.to_type(), r_eval.to_type()) {
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
                    }
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
        Expr::Bitwise { left, right, .. } => {
            check_expr(left, program, resolver, class, diagnostics, array_sizes);
            check_expr(right, program, resolver, class, diagnostics, array_sizes);
        }
        Expr::Unary { expr, .. } => {
            check_expr(expr, program, resolver, class, diagnostics, array_sizes);
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

                            if let Some(method) =
                                methods.iter().find(|m| m.params.len() == args.len())
                            {
                                for (j, (param, arg_ty)) in
                                    method.params.iter().zip(arg_types.iter()).enumerate()
                                {
                                    if let Some(actual_ty) = arg_ty {
                                        if !type_eval::is_type_compatible(
                                            &param.ty, actual_ty, program, resolver,
                                        ) {
                                            diagnostics.push(Diagnostic {
                                                range: args[j].range(),
                                                severity: Some(DiagnosticSeverity::ERROR),
                                                message: format!(
                                                    "Argument type mismatch: expected '{}', got '{}'",
                                                    param.ty, actual_ty
                                                ),
                                                ..Default::default()
                                            });
                                        }
                                    }
                                }
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
                    trainz_ast::gs::PostfixOp::Index(indices, _) => {
                        for idx in indices {
                            check_expr(idx, program, resolver, class, diagnostics, array_sizes);
                        }
                    }
                    _ => {}
                }

                // Update current_type for next op in chain
                if let Ok(_) = current_type {
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

#[cfg(test)]
mod tests;
