use crate::find::HasRange;
use crate::gs::process::helpers::{process_identifier, process_type};
use crate::gs::{
    BitwiseOp, ComparisonOp, EqualityOp, Expr, Literal, MathOp, PostfixOp, StringLiteral, Type,
    UnaryPostfixOp, UnaryPrefixOp,
};
use pest::iterators::Pair;
use tracing::trace;
use trainz_common::range::{combine_ranges, pair_to_range};
use trainz_parser::gs::grammar::Rule;
use trainz_parser::gs::pratt::PRATT;

#[tracing::instrument]
pub fn process_expr(pair: Pair<Rule>) -> Expr {
    let range = pair_to_range(&pair);
    let rule = pair.as_rule();
    match rule {
        Rule::assignment_expr => process_expr_pratt(pair),
        Rule::postfix_expr => process_postfix_expr(pair),
        Rule::unary_expr | Rule::operator_unary_lhs => process_unary_expr(pair),
        Rule::operator_unary_not
        | Rule::operator_unary_not_not
        | Rule::operator_unary_inverse
        | Rule::operator_unary_increment
        | Rule::operator_unary_decrement
        | Rule::operator_unary_positive
        | Rule::operator_unary_negative
        | Rule::operator_math_add
        | Rule::operator_math_subtract => {
            // These should only be handled as part of unary_expr.
            // Returning an "empty" expression to avoid creating symbols for them.
            Expr::Identifier(crate::gs::Identifier {
                name: "".to_string(),
                range,
            })
        }
        Rule::primary_expr
        | Rule::array_literal
        | Rule::inherited_method
        | Rule::statement_method
        | Rule::statement_cast
        | Rule::new_object
        | Rule::new_array
        | Rule::keyword_me
        | Rule::keyword_is_class
        | Rule::keyword_inherited => process_primary_expr(pair),
        Rule::variable => {
            let mut inner = pair.clone().into_inner();
            if let Some(first) = inner.next()
                && first.as_rule() == Rule::identifier
            {
                return Expr::Identifier(crate::gs::literal::Identifier {
                    name: first.as_str().to_string(),
                    range: pair_to_range(&first),
                });
            }
            let str = pair.as_str().to_string();
            Expr::Identifier(crate::gs::literal::Identifier { name: str, range })
        }
        Rule::identifier => Expr::Identifier(crate::gs::literal::Identifier {
            name: pair.as_str().to_string(),
            range,
        }),
        Rule::literal
        | Rule::string_value
        | Rule::char_value
        | Rule::float
        | Rule::hex
        | Rule::integer
        | Rule::null
        | Rule::boolean => process_literal(pair),
        Rule::grouped_expr => {
            let inner = pair.into_inner().nth(1).unwrap();
            Expr::Grouped(Box::new(process_expr(inner)), range)
        }
        Rule::brace_open
        | Rule::brace_close
        | Rule::bracket_open
        | Rule::bracket_close
        | Rule::paren_open
        | Rule::paren_close
        | Rule::semicolon => Expr::Identifier(crate::gs::literal::Identifier {
            name: pair.as_str().to_string(),
            range,
        }),
        _ => {
            let mut inner = pair.clone().into_inner();
            if let Some(first) = inner.next() {
                // Special case: if the rule is just a wrapper, recurse
                // But avoid infinite recursion if it's the same rule
                if first.as_rule() != rule {
                    return process_expr(first);
                }
            }
            Expr::Identifier(crate::gs::literal::Identifier {
                name: format!("UNKNOWN_RULE_{:?}", rule),
                range,
            })
        }
    }
}

#[tracing::instrument]
fn process_expr_pratt(pair: Pair<Rule>) -> Expr {
    PRATT
        .map_primary(|primary| process_expr(primary))
        .map_infix(|lhs, op, rhs| {
            let op_range = pair_to_range(&op);
            let op_str = op.as_str();
            let range = combine_ranges(lhs.range(), rhs.range());
            match op.as_rule() {
                Rule::operator_assignment => Expr::Assign {
                    left: Box::new(lhs),
                    right: Box::new(rhs),
                    range,
                    op_range,
                },
                Rule::operator_logical_or => Expr::LogicalOr {
                    left: Box::new(lhs),
                    right: Box::new(rhs),
                    range,
                    op_range,
                },
                Rule::operator_logical_and => Expr::LogicalAnd {
                    left: Box::new(lhs),
                    right: Box::new(rhs),
                    range,
                    op_range,
                },
                Rule::operator_equality_equal | Rule::operator_equality_not_equal => {
                    let op = if op_str == "==" {
                        EqualityOp::Eq
                    } else {
                        EqualityOp::Ne
                    };
                    Expr::Equality {
                        op,
                        left: Box::new(lhs),
                        right: Box::new(rhs),
                        range,
                        op_range,
                    }
                }
                Rule::operator_comparison_less_than
                | Rule::operator_comparison_greater_than
                | Rule::operator_comparison_less_equal
                | Rule::operator_comparison_greater_equal => {
                    let op = match op_str {
                        "<" => ComparisonOp::Lt,
                        ">" => ComparisonOp::Gt,
                        "<=" => ComparisonOp::Le,
                        ">=" => ComparisonOp::Ge,
                        _ => unreachable!(),
                    };
                    Expr::Comparison {
                        op,
                        left: Box::new(lhs),
                        right: Box::new(rhs),
                        range,
                        op_range,
                    }
                }
                Rule::operator_bitwise_shift_left
                | Rule::operator_bitwise_shift_right
                | Rule::operator_bitwise_and
                | Rule::operator_bitwise_or
                | Rule::operator_bitwise_xor => {
                    let op = match op_str {
                        "<<" => BitwiseOp::Shl,
                        ">>" => BitwiseOp::Shr,
                        "&" => BitwiseOp::And,
                        "|" => BitwiseOp::Or,
                        "^" => BitwiseOp::Not,
                        _ => unreachable!(),
                    };
                    Expr::Bitwise {
                        op,
                        left: Box::new(lhs),
                        right: Box::new(rhs),
                        range,
                        op_range,
                    }
                }
                Rule::operator_math_add
                | Rule::operator_math_subtract
                | Rule::operator_math_multiply
                | Rule::operator_math_divide
                | Rule::operator_math_modulo => {
                    let op = match op_str {
                        "+" => MathOp::Add,
                        "-" => MathOp::Sub,
                        "*" => MathOp::Mul,
                        "/" => MathOp::Div,
                        "%" => MathOp::Mod,
                        _ => unreachable!(),
                    };
                    Expr::BinaryMath {
                        op,
                        left: Box::new(lhs),
                        right: Box::new(rhs),
                        range,
                        op_range,
                    }
                }
                _ => unreachable!("Unexpected operator rule: {:?}", op.as_rule()),
            }
        })
        .parse(pair.into_inner())
}

#[tracing::instrument]
fn process_unary_expr(pair: Pair<Rule>) -> Expr {
    let range = pair_to_range(&pair);
    let mut inner = pair.clone().into_inner();
    let first = inner.next();
    if first.is_none() {
        // If it's an atomic rule (like operator_unary_negative), it has no inner pairs.
        // We should skip these in process_expr.
        return Expr::Identifier(crate::gs::Identifier {
            name: format!("UNKNOWN_RULE_{:?}", pair.as_rule()),
            range,
        });
    }
    let first_pair = first.unwrap();
    let op_rule = match first_pair.as_rule() {
        Rule::operator_unary_lhs => first_pair.into_inner().next().unwrap(),
        Rule::operator_unary_not
        | Rule::operator_unary_not_not
        | Rule::operator_unary_inverse
        | Rule::operator_unary_increment
        | Rule::operator_unary_decrement
        | Rule::operator_unary_positive
        | Rule::operator_unary_negative
        | Rule::operator_math_add
        | Rule::operator_math_subtract => first_pair,
        _ => {
            // If the first pair is not an operator, it might be the operand itself (e.g. if we are called on a unary_expr that has no prefix)
            // Or it's a wrapper like unary_expr, recurse to find the operator or operand.
            if first_pair.clone().into_inner().count() > 0 {
                return process_expr(first_pair);
            }
            return Expr::Identifier(crate::gs::Identifier {
                name: first_pair.as_str().to_string(),
                range: pair_to_range(&first_pair),
            });
        }
    };

    let op_rule_type = op_rule.as_rule();
    match op_rule_type {
        Rule::operator_unary_not
        | Rule::operator_unary_not_not
        | Rule::operator_unary_inverse
        | Rule::operator_unary_increment
        | Rule::operator_unary_decrement
        | Rule::operator_unary_positive
        | Rule::operator_unary_negative
        | Rule::operator_math_add
        | Rule::operator_math_subtract => {
            let op_range = pair_to_range(&op_rule);
            let op_str = op_rule.as_str();
            let op = match op_rule_type {
                Rule::operator_unary_not | Rule::operator_unary_inverse => UnaryPrefixOp::Not,
                Rule::operator_unary_not_not => UnaryPrefixOp::NotNot,
                Rule::operator_unary_increment => UnaryPrefixOp::Inc,
                Rule::operator_unary_decrement => UnaryPrefixOp::Dec,
                Rule::operator_unary_positive | Rule::operator_math_add => UnaryPrefixOp::Plus,
                Rule::operator_unary_negative | Rule::operator_math_subtract => {
                    UnaryPrefixOp::Minus
                }
                _ => unreachable!("Unexpected unary prefix operator: {}", op_str),
            };

            // Re-iterate from start to find the operand
            let inner = pair.into_inner();
            let mut next_pair = None;
            for p in inner {
                let r = p.as_rule();
                if r != Rule::operator_unary_lhs
                    && r != Rule::operator_unary_not
                    && r != Rule::operator_unary_not_not
                    && r != Rule::operator_unary_inverse
                    && r != Rule::operator_unary_increment
                    && r != Rule::operator_unary_decrement
                    && r != Rule::operator_unary_positive
                    && r != Rule::operator_unary_negative
                    && r != Rule::operator_math_add
                    && r != Rule::operator_math_subtract
                    && r != Rule::operator_assignment
                    && r != Rule::line_comment
                    && r != Rule::block_comment
                {
                    next_pair = Some(p);
                    break;
                }
            }

            if let Some(next_pair) = next_pair {
                let expr = process_expr(next_pair);
                if let (UnaryPrefixOp::Minus, Expr::Literal(lit)) = (&op, &expr) {
                    match lit {
                        Literal::Int(val, _) => {
                            return Expr::Literal(Literal::Int(-val, range));
                        }
                        Literal::Float(val, _) => {
                            return Expr::Literal(Literal::Float(-val, range));
                        }
                        _ => {}
                    }
                }
                Expr::Unary {
                    op,
                    expr: Box::new(expr),
                    range,
                    op_range,
                }
            } else {
                Expr::Identifier(crate::gs::Identifier {
                    name: format!("UNKNOWN_RULE_{:?}", op_rule_type),
                    range: op_range,
                })
            }
        }
        _ => process_expr(op_rule),
    }
}

#[tracing::instrument]
fn process_postfix_expr(pair: Pair<Rule>) -> Expr {
    let range = pair_to_range(&pair);
    let mut inner = pair.into_inner();
    let first = inner.next().unwrap();
    let base = process_expr(first);

    let mut ops = Vec::new();
    while let Some(op_pair) = inner.next() {
        if op_pair.as_rule() == Rule::deref {
            let next_expr = process_expr(
                inner
                    .next()
                    .expect("deref must be followed by primary_expr"),
            );
            match next_expr {
                Expr::Identifier(id) => ops.push(PostfixOp::Deref(id)),
                Expr::IsClass(id) => ops.push(PostfixOp::Deref(id)),
                _ => panic!(
                    "Deref must be followed by identifier or isclass, got {:?}",
                    next_expr
                ),
            }
        } else {
            let op = process_postfix_op(op_pair);
            ops.push(op);
        }
    }

    if ops.is_empty() {
        base
    } else {
        Expr::Postfix {
            expr: Box::new(base),
            ops,
            range,
        }
    }
}

#[tracing::instrument]
fn process_postfix_op(pair: Pair<Rule>) -> PostfixOp {
    let range = pair_to_range(&pair);
    match pair.as_rule() {
        Rule::statement_method_call => {
            let mut args = vec![];
            for inner_pair in pair.into_inner() {
                match inner_pair.as_rule() {
                    Rule::statement_method_arguments => {
                        for arg in inner_pair.into_inner() {
                            if arg.as_rule() == Rule::comma {
                                continue;
                            }
                            args.push(process_expr(arg));
                        }
                    }
                    Rule::paren_open | Rule::paren_close | Rule::comma => continue,
                    _ => {
                        // In case of a single argument that isn't wrapped in arguments rule (though grammar says it is)
                        args.push(process_expr(inner_pair));
                    }
                }
            }
            PostfixOp::Call(args, range)
        }
        Rule::array_subscript => {
            let mut indices = vec![];
            for idx in pair.into_inner() {
                if idx.as_rule() == Rule::bracket_open
                    || idx.as_rule() == Rule::bracket_close
                    || idx.as_rule() == Rule::comma
                {
                    continue;
                }
                indices.push(process_expr(idx));
            }
            PostfixOp::Index(indices, range)
        }
        Rule::operator_unary_rhs
        | Rule::operator_unary_increment
        | Rule::operator_unary_decrement => {
            let op = if pair.as_str() == "++" {
                UnaryPostfixOp::Inc
            } else {
                UnaryPostfixOp::Dec
            };
            PostfixOp::Unary(op, range)
        }
        _ => unreachable!("Unexpected postfix op: {:?}", pair.as_rule()),
    }
}

#[tracing::instrument]
fn process_cast(pair: Pair<Rule>) -> Expr {
    let range = pair_to_range(&pair);
    let mut inner = pair.into_inner();
    let cast_rule = inner.next().unwrap();
    match cast_rule.as_rule() {
        Rule::static_cast => {
            let mut static_inner = cast_rule.into_inner();
            let keyword_cast_pair = static_inner.next().unwrap();
            let keyword_cast_range = pair_to_range(&keyword_cast_pair);
            static_inner.next(); // angle_open
            let ty_pair = static_inner.next().unwrap();
            let ty = process_type(ty_pair);
            static_inner.next(); // angle_close

            // Check for paren_open/paren_close or just unary_expr
            let next = static_inner.next().unwrap();
            let expr = if next.as_rule() == Rule::paren_open {
                let expr_inner = static_inner.next().unwrap();
                static_inner.next(); // paren_close
                process_expr(expr_inner)
            } else {
                process_expr(next)
            };

            Expr::Cast {
                ty,
                expr: Box::new(expr),
                range,
                keyword_cast_range: Some(keyword_cast_range),
            }
        }
        Rule::dynamic_cast => {
            let mut dynamic_inner = cast_rule.into_inner();
            dynamic_inner.next(); // paren_open
            let ty_pair = dynamic_inner.next().unwrap();
            let ty = process_type(ty_pair);
            dynamic_inner.next(); // paren_close
            let expr_pair = dynamic_inner.next().unwrap();
            let expr = process_expr(expr_pair);

            Expr::Cast {
                ty,
                expr: Box::new(expr),
                range,
                keyword_cast_range: None,
            }
        }
        _ => Expr::Identifier(crate::gs::Identifier {
            name: format!("UNKNOWN_CAST_{:?}", cast_rule.as_rule()),
            range,
        }),
    }
}

#[tracing::instrument]
fn process_new_object(pair: Pair<Rule>) -> Expr {
    let range = pair_to_range(&pair);
    let mut inner = pair.into_inner();
    let keyword_new_pair = inner.next().unwrap();
    let keyword_new_range = pair_to_range(&keyword_new_pair);
    let ty_pair = inner.next().unwrap();
    let ty = process_type(ty_pair);

    let mut args = vec![];
    while let Some(next) = inner.next() {
        match next.as_rule() {
            Rule::new_object_arguments => {
                for arg_pair in next.into_inner() {
                    if arg_pair.as_rule() == Rule::comma {
                        continue;
                    }
                    args.push(process_expr(arg_pair));
                }
            }
            Rule::paren_open | Rule::paren_close | Rule::comma => continue,
            _ => args.push(process_expr(next)),
        }
    }

    Expr::NewObject {
        ty,
        args,
        range,
        keyword_new_range,
    }
}

#[tracing::instrument]
fn process_new_array(pair: Pair<Rule>) -> Expr {
    let range = pair_to_range(&pair);
    let mut inner = pair.into_inner();
    let keyword_new_pair = inner.next().unwrap();
    let keyword_new_range = pair_to_range(&keyword_new_pair);
    let ty_pair = inner.next().unwrap();
    let mut ty = process_type(ty_pair);

    let mut sizes = vec![];
    while let Some(next) = inner.next() {
        if next.as_rule() == Rule::bracket_open {
            let size_pair = inner.next().expect("size expected after bracket_open");
            sizes.push(process_expr(size_pair));
            // Skip bracket_close
            inner.next();
        }
    }

    // Wrap the base type for each additional dimension beyond the first.
    for _ in 0..sizes.len().saturating_sub(1) {
        ty = Type::Array(Box::new(ty), range);
    }

    Expr::NewArray {
        ty,
        size: Box::new(
            sizes
                .first()
                .cloned()
                .unwrap_or(Expr::Literal(Literal::Int(0, range))),
        ),
        range,
        keyword_new_range,
    }
}

#[tracing::instrument]
fn process_primary_expr(pair: Pair<Rule>) -> Expr {
    let range = pair_to_range(&pair);
    match pair.as_rule() {
        Rule::statement_cast => return process_cast(pair),
        Rule::new_object => return process_new_object(pair),
        Rule::new_array => return process_new_array(pair),
        _ => {}
    }
    let mut inner_pairs = pair.clone().into_inner();
    let inner = if let Some(first) = inner_pairs.next() {
        first
    } else {
        return Expr::Identifier(crate::gs::Identifier {
            name: pair.as_str().to_string(),
            range,
        });
    };
    trace!(
        "process_primary_expr: rule {:?} (parent: {:?})",
        inner.as_rule(),
        pair.as_rule()
    );
    match inner.as_rule() {
        Rule::unary_expr | Rule::postfix_expr | Rule::primary_expr => process_expr(inner),
        Rule::static_cast | Rule::dynamic_cast => process_cast(pair),
        Rule::variable => {
            let res = Expr::Identifier(process_identifier(
                inner.clone().into_inner().next().unwrap(),
            ));
            trace!(
                "process_primary_expr: created variable {} at {:?}",
                match &res {
                    Expr::Identifier(id) => &id.name,
                    _ => "",
                },
                range
            );
            res
        }
        Rule::keyword_is_class => Expr::IsClass(crate::gs::Identifier {
            name: "isclass".to_string(),
            range,
        }),
        Rule::keyword_me => Expr::Identifier(crate::gs::Identifier {
            name: "me".to_string(),
            range,
        }),
        Rule::literal => process_literal(inner),
        Rule::grouped_expr => process_expr(inner),
        Rule::statement_method => {
            // statement_method = { statement_method_name ~ statement_method_call }
            // Represent as Postfix(Identifier, [Call])
            let mut inner = inner.clone().into_inner();
            let name_pair = inner.next().unwrap();
            let name = process_identifier(name_pair);
            let call_pair = inner.next().unwrap();
            let op = process_postfix_op(call_pair);
            let res = Expr::Postfix {
                expr: Box::new(Expr::Identifier(name.clone())),
                ops: vec![op],
                range,
            };
            trace!(
                "process_primary_expr: created statement_method {} at {:?}",
                name.name, range
            );
            res
        }
        Rule::inherited_method => {
            // inherited_method = { keyword_inherited ~ statement_method_call }
            // We can represent this as Postfix(Identifier("inherited"), [Call])
            let mut inner = inner.into_inner();
            let keyword_pair = inner.next().unwrap();
            let keyword_range = pair_to_range(&keyword_pair);
            let call_pair = inner.next().unwrap();
            let op = process_postfix_op(call_pair);
            let res = Expr::Postfix {
                expr: Box::new(Expr::Identifier(crate::gs::Identifier {
                    name: "inherited".to_string(),
                    range: keyword_range,
                })),
                ops: vec![op],
                range,
            };
            trace!(
                "process_primary_expr: created inherited at {:?}",
                keyword_range
            );
            res
        }
        Rule::keyword_inherited => {
            let res = Expr::Identifier(crate::gs::Identifier {
                name: "inherited".to_string(),
                range,
            });
            trace!("process_primary_expr: created inherited at {:?}", range);
            res
        }
        Rule::array_literal => {
            let mut elements = vec![];
            for p in inner.into_inner() {
                elements.push(process_expr(p));
            }
            // For now, let's represent array literal as a call to "array" or just a special expression
            // Since we don't have Expr::ArrayLiteral in the AST yet, let's use a placeholder or add it.
            // Looking at crates/ast/src/gs/expr.rs... it doesn't have ArrayLiteral.
            // I'll use a Postfix call to a virtual "array" function for now to avoid changing the AST too much
            // if I don't have to.
            Expr::Postfix {
                expr: Box::new(Expr::Identifier(crate::gs::Identifier {
                    name: "array".to_string(),
                    range,
                })),
                ops: vec![PostfixOp::Call(elements, range)],
                range,
            }
        }
        _ => {
            trace!("process_primary_expr: unknown rule {:?}", inner.as_rule());
            Expr::Identifier(crate::gs::Identifier {
                name: format!("UNKNOWN_RULE_{:?}", inner.as_rule()),
                range,
            })
        }
    }
}

#[tracing::instrument]
fn process_literal(pair: Pair<Rule>) -> Expr {
    let (inner, inner_rule) = if let Some(first) = pair.clone().into_inner().next() {
        let rule = first.as_rule();
        (first, rule)
    } else {
        (pair.clone(), pair.as_rule())
    };

    let inner_range = pair_to_range(&inner);
    let lit = match inner_rule {
        Rule::string | Rule::string_value => Literal::String(StringLiteral {
            value: inner.as_str().to_string(),
            range: inner_range,
        }),
        Rule::char | Rule::char_value => {
            Literal::Char(inner.as_str().chars().next().unwrap(), inner_range)
        }
        Rule::float => {
            let s = inner.as_str().to_lowercase();
            let s = if s.ends_with('f') {
                &s[..s.len() - 1]
            } else {
                &s
            };
            Literal::Float(s.parse().unwrap(), inner_range)
        }
        Rule::integer => Literal::Int(inner.as_str().parse().unwrap(), inner_range),
        Rule::hex => Literal::Hex(
            u64::from_str_radix(
                inner
                    .as_str()
                    .trim_start_matches("0x")
                    .trim_start_matches("0X"),
                16,
            )
            .unwrap(),
            inner_range,
        ),
        Rule::boolean => Literal::Bool(inner.as_str() == "true", inner_range),
        Rule::null => Literal::Null(inner_range),
        Rule::literal => return process_literal(inner),
        _ => unreachable!("Unexpected literal rule: {:?}", inner_rule),
    };
    Expr::Literal(lit)
}
