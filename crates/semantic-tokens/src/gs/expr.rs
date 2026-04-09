use crate::gs::types::collect_type_tokens;
use tower_lsp_server::ls_types::{Range, SemanticTokenModifier, SemanticTokenType};
use trainz_ast::gs::{Expr, PostfixOp};

pub fn collect_expr_tokens(
    expr: &Expr,
    raw_tokens: &mut Vec<(Range, SemanticTokenType, Vec<SemanticTokenModifier>)>,
    known_classes: &std::collections::HashSet<String>,
) {
    match expr {
        Expr::Assign {
            left,
            right,
            op_range,
            ..
        } => {
            collect_expr_tokens(left, raw_tokens, known_classes);
            collect_expr_tokens(right, raw_tokens, known_classes);
            raw_tokens.push((*op_range, SemanticTokenType::OPERATOR, vec![]));
        }
        Expr::LogicalOr {
            left,
            right,
            op_range,
            ..
        }
        | Expr::LogicalAnd {
            left,
            right,
            op_range,
            ..
        } => {
            collect_expr_tokens(left, raw_tokens, known_classes);
            collect_expr_tokens(right, raw_tokens, known_classes);
            raw_tokens.push((*op_range, SemanticTokenType::OPERATOR, vec![]));
        }
        Expr::Equality {
            left,
            right,
            op_range,
            ..
        }
        | Expr::Comparison {
            left,
            right,
            op_range,
            ..
        }
        | Expr::Bitwise {
            left,
            right,
            op_range,
            ..
        }
        | Expr::BinaryMath {
            left,
            right,
            op_range,
            ..
        } => {
            collect_expr_tokens(left, raw_tokens, known_classes);
            collect_expr_tokens(right, raw_tokens, known_classes);
            raw_tokens.push((*op_range, SemanticTokenType::OPERATOR, vec![]));
        }
        Expr::Unary { expr, op_range, .. } => {
            collect_expr_tokens(expr, raw_tokens, known_classes);
            raw_tokens.push((*op_range, SemanticTokenType::OPERATOR, vec![]));
        }
        Expr::Postfix { expr, ops, .. } => {
            let mut is_expr_call = false;
            if let Some(PostfixOp::Call(_, _)) = ops.first() {
                is_expr_call = true;
            }

            if is_expr_call {
                if let Expr::Identifier(id) = &**expr {
                    let (token_type, modifiers) = if id.name == "isclass" {
                        (
                            SemanticTokenType::METHOD,
                            vec![SemanticTokenModifier::DEFAULT_LIBRARY],
                        )
                    } else if id.name == "inherited" {
                        (
                            SemanticTokenType::METHOD,
                            vec![SemanticTokenModifier::DEFAULT_LIBRARY],
                        )
                    } else {
                        (SemanticTokenType::METHOD, vec![])
                    };
                    raw_tokens.push((id.range, token_type, modifiers));
                } else {
                    collect_expr_tokens(expr, raw_tokens, known_classes);
                }
            } else {
                collect_expr_tokens(expr, raw_tokens, known_classes);
            }

            for i in 0..ops.len() {
                let op = &ops[i];
                let next_is_call = matches!(ops.get(i + 1), Some(PostfixOp::Call(_, _)));
                match op {
                    PostfixOp::Deref(id) => {
                        let token_type = if id.name == "me" || id.name == "isclass" {
                            SemanticTokenType::KEYWORD
                        } else if next_is_call {
                            SemanticTokenType::METHOD
                        } else {
                            SemanticTokenType::PROPERTY
                        };
                        raw_tokens.push((id.range, token_type, vec![]))
                    }
                    PostfixOp::Call(args, _) => {
                        for arg in args {
                            collect_expr_tokens(arg, raw_tokens, known_classes);
                        }
                    }
                    PostfixOp::Index(args, _) => {
                        for arg in args {
                            collect_expr_tokens(arg, raw_tokens, known_classes);
                        }
                    }
                    PostfixOp::Unary(_, _) => {}
                }
            }
        }
        Expr::Cast { ty, expr, .. } => {
            collect_type_tokens(ty, raw_tokens);
            collect_expr_tokens(expr, raw_tokens, known_classes);
        }
        Expr::NewObject { ty, args, .. } => {
            collect_type_tokens(ty, raw_tokens);
            for arg in args {
                collect_expr_tokens(arg, raw_tokens, known_classes);
            }
        }
        Expr::NewArray { ty, size, .. } => {
            collect_type_tokens(ty, raw_tokens);
            collect_expr_tokens(size, raw_tokens, known_classes);
        }
        Expr::Literal(lit) => {
            let range = trainz_ast::find::HasRange::range(lit);
            let (token_type, modifiers) = match lit {
                trainz_ast::gs::Literal::String(_) | trainz_ast::gs::Literal::Char(_, _) => {
                    (SemanticTokenType::STRING, vec![])
                }
                trainz_ast::gs::Literal::Float(_, _)
                | trainz_ast::gs::Literal::Int(_, _)
                | trainz_ast::gs::Literal::Hex(_, _) => (SemanticTokenType::NUMBER, vec![]),
                trainz_ast::gs::Literal::Bool(_, _) | trainz_ast::gs::Literal::Null(_) => {
                    (SemanticTokenType::KEYWORD, vec![])
                }
            };
            raw_tokens.push((range, token_type, modifiers));
        }
        Expr::IsClass(range) => {
            raw_tokens.push((*range, SemanticTokenType::KEYWORD, vec![]));
        }
        Expr::Identifier(id) => {
            let (token_type, modifiers) = if id.name == "me" {
                (
                    SemanticTokenType::KEYWORD,
                    vec![SemanticTokenModifier::READONLY],
                )
            } else if id.name == "isclass" {
                (
                    SemanticTokenType::METHOD,
                    vec![SemanticTokenModifier::DEFAULT_LIBRARY],
                )
            } else if id.name == "inherited" {
                (
                    SemanticTokenType::METHOD,
                    vec![SemanticTokenModifier::DEFAULT_LIBRARY],
                )
            } else if known_classes.contains(&id.name) {
                (SemanticTokenType::CLASS, vec![])
            } else {
                (SemanticTokenType::VARIABLE, vec![])
            };
            raw_tokens.push((id.range, token_type, modifiers));
        }
        Expr::Grouped(expr, _) => collect_expr_tokens(expr, raw_tokens, known_classes),
    }
}
