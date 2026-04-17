#![allow(deprecated)]
use std::collections::HashMap;
use tower_lsp_server::ls_types::{DocumentSymbol, SymbolKind};
use tracing::trace;
use trainz_ast::find::HasRange;
use trainz_ast::gs::type_eval::{ClassResolver, EvaluatedType, evaluate_expr_type};
use trainz_ast::gs::{Expr, Literal, PostfixOp, Type};

#[allow(deprecated)]
#[tracing::instrument(skip(resolver))]
pub(crate) fn process_expr(
    expr: &Expr,
    program: &trainz_ast::gs::Program,
    resolver: &dyn ClassResolver,
) -> Vec<DocumentSymbol> {
    let mut symbols = vec![];

    match expr {
        Expr::Assign {
            left, right, range, ..
        } => {
            symbols.extend(process_expr(left, program, resolver));
            symbols.extend(process_expr(right, program, resolver));
            symbols.push(DocumentSymbol {
                name: "assign".to_string(),
                detail: None,
                kind: SymbolKind::VARIABLE,
                tags: None,
                deprecated: None,
                range: *range,
                selection_range: *range,
                children: None,
            });
        }
        Expr::LogicalOr { left, right, .. }
        | Expr::LogicalAnd { left, right, .. }
        | Expr::Equality { left, right, .. }
        | Expr::Comparison { left, right, .. }
        | Expr::Bitwise { left, right, .. }
        | Expr::BinaryMath { left, right, .. } => {
            symbols.extend(process_expr(left, program, resolver));
            symbols.extend(process_expr(right, program, resolver));
        }
        Expr::Unary {
            op, expr, range, ..
        } => {
            match (op, &**expr) {
                (trainz_ast::gs::UnaryPrefixOp::Minus, Expr::Literal(Literal::Float(v, _))) => {
                    let mut name = format!("-{}", v);
                    if !name.contains('.') {
                        name.push_str(".0");
                    }
                    symbols.push(DocumentSymbol {
                        name,
                        detail: Some(format!("{:?}", Literal::Float(-*v, *range))),
                        kind: SymbolKind::CONSTANT,
                        tags: None,
                        deprecated: None,
                        range: *range,
                        selection_range: *range,
                        children: None,
                    });
                }
                (trainz_ast::gs::UnaryPrefixOp::Minus, Expr::Literal(Literal::Int(v, _))) => {
                    symbols.push(DocumentSymbol {
                        name: format!("-{}", v),
                        detail: Some(format!("{:?}", Literal::Int(-*v, *range))),
                        kind: SymbolKind::CONSTANT,
                        tags: None,
                        deprecated: None,
                        range: *range,
                        selection_range: *range,
                        children: None,
                    });
                }
                _ => {
                    // Only recurse into operand, don't create a symbol for the operator itself
                    // unless it's a folded literal handled above.
                    symbols.extend(process_expr(expr, program, resolver));
                }
            }
        }
        Expr::Postfix {
            expr: inner, ops, ..
        } => {
            symbols.extend(process_expr(inner, program, resolver));
            for (idx, op) in ops.iter().enumerate() {
                match op {
                    PostfixOp::Call(args, _) => {
                        for arg in args {
                            symbols.extend(process_expr(arg, program, resolver));
                        }
                    }
                    PostfixOp::Deref(id) => {
                        let mut kind = SymbolKind::METHOD;
                        let ops_before = &ops[..idx];
                        let res = if ops_before.is_empty() {
                            evaluate_expr_type(
                                inner,
                                program,
                                resolver,
                                id.range.start,
                                None,
                                &HashMap::new(),
                            )
                        } else {
                            let temp_expr = Expr::Postfix {
                                expr: inner.clone(),
                                ops: ops_before.to_vec(),
                                range: inner.range(),
                            };
                            evaluate_expr_type(
                                &temp_expr,
                                program,
                                resolver,
                                id.range.start,
                                None,
                                &HashMap::new(),
                            )
                        };

                        if let Ok(EvaluatedType::Type(Type::Named(class_id))) = res
                            && let Some(class) = resolver.find_class(&class_id.name)
                            && class.find_field(resolver, &id.name).is_some()
                        {
                            kind = SymbolKind::PROPERTY;
                        }

                        symbols.push(DocumentSymbol {
                            name: id.name.clone(),
                            detail: None,
                            kind,
                            tags: None,
                            deprecated: None,
                            range: id.range,
                            selection_range: id.range,
                            children: None,
                        });
                    }
                    PostfixOp::Index(indices, _) => {
                        for idx_expr in indices {
                            symbols.extend(process_expr(idx_expr, program, resolver));
                        }
                    }
                    PostfixOp::Unary(_, _) => {}
                }
            }
        }
        Expr::Cast { expr, ty, .. } => {
            symbols.extend(process_type_symbols(ty));
            symbols.extend(process_expr(expr, program, resolver));
        }
        Expr::NewObject { args, ty, .. } => {
            symbols.extend(process_type_symbols(ty));
            for arg in args {
                symbols.extend(process_expr(arg, program, resolver));
            }
        }
        Expr::NewArray { size, ty, .. } => {
            symbols.extend(process_type_symbols(ty));
            symbols.extend(process_expr(size, program, resolver));
        }
        Expr::Literal(lit) => {
            let name = match lit {
                Literal::Int(v, _) => v.to_string(),
                Literal::Float(v, _) => {
                    let s = if *v == 0.0 && v.is_sign_negative() {
                        "-0.0".to_string()
                    } else {
                        v.to_string()
                    };
                    if s.contains('.') {
                        s
                    } else {
                        format!("{:.1}", v)
                    }
                }
                Literal::Hex(v, _) => format!("0x{:x}", v),
                Literal::Char(v, _) => format!("'{}'", v),
                Literal::String(s) => s.value.clone(),
                Literal::Bool(v, _) => v.to_string(),
                Literal::Null(_) => "null".to_string(),
            };
            symbols.push(DocumentSymbol {
                name,
                detail: Some(format!("{:?}", lit)),
                kind: SymbolKind::CONSTANT,
                tags: None,
                deprecated: None,
                range: lit.range(),
                selection_range: lit.range(),
                children: None,
            });
        }
        Expr::Identifier(id) => {
            if id.name.starts_with("UNKNOWN_RULE_") || id.name.is_empty() {
                return symbols;
            }
            trace!("symboliser: processing identifier {}", id.name);
            let kind = if resolver.find_class(&id.name).is_some() {
                SymbolKind::CLASS
            } else {
                SymbolKind::VARIABLE
            };
            symbols.push(DocumentSymbol {
                name: id.name.clone(),
                detail: None,
                kind,
                tags: None,
                deprecated: None,
                range: id.range,
                selection_range: id.range,
                children: None,
            });
        }
        Expr::IsClass(id) => {
            symbols.push(DocumentSymbol {
                name: "isclass".to_string(),
                detail: None,
                kind: SymbolKind::OPERATOR,
                tags: None,
                deprecated: None,
                range: id.range,
                selection_range: id.range,
                children: None,
            });
        }
        Expr::Grouped(expr, _) => {
            symbols.extend(process_expr(expr, program, resolver));
        }
    }

    symbols
}

pub(crate) fn process_type_symbols(ty: &Type) -> Vec<DocumentSymbol> {
    let mut symbols = vec![];
    match ty {
        Type::Named(id) => {
            symbols.push(DocumentSymbol {
                name: id.name.clone(),
                detail: None,
                kind: SymbolKind::CLASS,
                tags: None,
                deprecated: None,
                range: id.range,
                selection_range: id.range,
                children: None,
            });
        }
        Type::Array(inner, _) => {
            symbols.extend(process_type_symbols(inner));
        }
        _ => {}
    }
    symbols
}
