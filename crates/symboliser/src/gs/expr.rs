use log::trace;
use tower_lsp_server::ls_types::{DocumentSymbol, SymbolKind};
use trainz_ast::find::HasRange;
use trainz_ast::gs::{Expr, Literal, PostfixOp};

#[allow(deprecated)]
pub(crate) fn process_expr(expr: &Expr) -> Vec<DocumentSymbol> {
    let mut symbols = vec![];
    trace!("symboliser: processing expr {:?}", expr);

    match expr {
        Expr::Assign {
            left, right, range, ..
        } => {
            symbols.extend(process_expr(left));
            symbols.extend(process_expr(right));
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
            symbols.extend(process_expr(left));
            symbols.extend(process_expr(right));
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
                    symbols.extend(process_expr(expr));
                }
            }
        }
        Expr::Postfix { expr, ops, .. } => {
            symbols.extend(process_expr(expr));
            for op in ops {
                match op {
                    PostfixOp::Call(args, _) => {
                        for arg in args {
                            symbols.extend(process_expr(arg));
                        }
                    }
                    PostfixOp::Deref(id) => {
                        symbols.push(DocumentSymbol {
                            name: id.name.clone(),
                            detail: None,
                            kind: SymbolKind::METHOD,
                            tags: None,
                            deprecated: None,
                            range: id.range,
                            selection_range: id.range,
                            children: None,
                        });
                    }
                    PostfixOp::Index(indices, _) => {
                        for idx in indices {
                            symbols.extend(process_expr(idx));
                        }
                    }
                    PostfixOp::Unary(_, _) => {}
                }
            }
        }
        Expr::Cast { expr, .. } => {
            symbols.extend(process_expr(expr));
        }
        Expr::NewObject { args, .. } => {
            for arg in args {
                symbols.extend(process_expr(arg));
            }
        }
        Expr::NewArray { size, .. } => {
            symbols.extend(process_expr(size));
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
            symbols.push(DocumentSymbol {
                name: id.name.clone(),
                detail: None,
                kind: SymbolKind::VARIABLE,
                tags: None,
                deprecated: None,
                range: id.range,
                selection_range: id.range,
                children: None,
            });
        }
        Expr::IsClass(range) => {
            symbols.push(DocumentSymbol {
                name: "isClass".to_string(),
                detail: None,
                kind: SymbolKind::OPERATOR,
                tags: None,
                deprecated: None,
                range: *range,
                selection_range: *range,
                children: None,
            });
        }
        Expr::Grouped(expr, _) => {
            symbols.extend(process_expr(expr));
        }
    }

    symbols
}
