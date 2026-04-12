use crate::gs::process::helpers::process_identifier;
use crate::gs::{
    BitwiseOp, ComparisonOp, EqualityOp, Expr, Literal, MathOp, PostfixOp, StringLiteral,
    UnaryPostfixOp, UnaryPrefixOp,
};
use pest::iterators::Pair;
use tracing::trace;
use trainz_common::range::pair_to_range;
use trainz_parser::gs::grammar::Rule;

pub fn process_expr(pair: Pair<Rule>) -> Expr {
    let range = pair_to_range(&pair);
    let rule = pair.as_rule();
    match rule {
        Rule::assignment_expr => process_assignment_expr(pair),
        Rule::multiplicative_expr
        | Rule::additive_expr
        | Rule::bitwise_expr
        | Rule::comparison_expr
        | Rule::equality_expr
        | Rule::logical_and_expr
        | Rule::logical_or_expr
        | Rule::postfix_expr
        | Rule::unary_expr
        | Rule::operator_unary_lhs => {
            let inner = pair.clone().into_inner();
            let mut non_comment_inner = inner.clone().filter(|p| {
                p.as_rule() != Rule::line_comment && p.as_rule() != Rule::block_comment
            });
            if non_comment_inner.clone().count() == 1 {
                return process_expr(non_comment_inner.next().unwrap());
            }
            match rule {
                Rule::logical_or_expr => process_binary(pair, Rule::operator_logical_or),
                Rule::logical_and_expr => process_binary(pair, Rule::operator_logical_and),
                Rule::equality_expr => process_binary(pair, Rule::operator_equality),
                Rule::comparison_expr => process_binary(pair, Rule::operator_comparison),
                Rule::bitwise_expr => process_binary(pair, Rule::operator_bitwise),
                Rule::additive_expr => process_binary(pair, Rule::operator_math_add),
                Rule::multiplicative_expr => process_binary(pair, Rule::operator_math_multiply),
                Rule::postfix_expr => process_postfix_expr(pair),
                Rule::unary_expr | Rule::operator_unary_lhs => process_unary_expr(pair),
                _ => unreachable!(),
            }
        }
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
        | Rule::statement_method => process_primary_expr(pair),
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

fn process_assignment_expr(pair: Pair<Rule>) -> Expr {
    let range = pair_to_range(&pair);
    let mut inner = pair.into_inner();
    let left = process_expr(inner.next().unwrap());
    if let Some(op) = inner.next() {
        let op_range = pair_to_range(&op);
        let right = process_expr(inner.next().unwrap());
        Expr::Assign {
            left: Box::new(left),
            right: Box::new(right),
            range,
            op_range,
        }
    } else {
        left
    }
}

fn process_binary(pair: Pair<Rule>, _op_rule: Rule) -> Expr {
    let range = pair_to_range(&pair);
    let mut inner = pair.into_inner();
    let first = inner.next();
    if first.is_none() {
        return Expr::Identifier(crate::gs::literal::Identifier {
            name: "EMPTY_BINARY".to_string(),
            range,
        });
    }
    let mut res = process_expr(first.unwrap());
    while let Some(op_pair) = inner.next() {
        let next = inner.next();
        if next.is_none() {
            break;
        }
        let right = process_expr(next.unwrap());

        let mut op_pair_iter = op_pair.clone().into_inner();
        let op_token_pair = op_pair_iter.next().unwrap_or(op_pair.clone());
        let op_range = pair_to_range(&op_token_pair);

        let op_str = op_pair.as_str();
        match op_pair.as_rule() {
            Rule::operator_logical_or => {
                res = Expr::LogicalOr {
                    left: Box::new(res),
                    right: Box::new(right),
                    range,
                    op_range,
                }
            }
            Rule::operator_logical_and => {
                res = Expr::LogicalAnd {
                    left: Box::new(res),
                    right: Box::new(right),
                    range,
                    op_range,
                }
            }
            Rule::operator_equality
            | Rule::operator_equality_equal
            | Rule::operator_equality_not_equal => {
                let op = if op_str == "==" {
                    EqualityOp::Eq
                } else {
                    EqualityOp::Ne
                };
                res = Expr::Equality {
                    op,
                    left: Box::new(res),
                    right: Box::new(right),
                    range,
                    op_range,
                };
            }
            Rule::operator_comparison
            | Rule::operator_comparison_less_than
            | Rule::operator_comparison_greater_than
            | Rule::operator_comparison_less_equal
            | Rule::operator_comparison_greater_equal => {
                let op = match op_str {
                    "<=" => ComparisonOp::Le,
                    ">=" => ComparisonOp::Ge,
                    "<" => ComparisonOp::Lt,
                    ">" => ComparisonOp::Gt,
                    _ => unreachable!("Unexpected comparison operator str: {}", op_str),
                };
                res = Expr::Comparison {
                    op,
                    left: Box::new(res),
                    right: Box::new(right),
                    range,
                    op_range,
                };
            }
            Rule::operator_bitwise
            | Rule::operator_bitwise_shift_left
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
                    _ => unreachable!("Unexpected bitwise operator str: {}", op_str),
                };
                res = Expr::Bitwise {
                    op,
                    left: Box::new(res),
                    right: Box::new(right),
                    range,
                    op_range,
                };
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
                    _ => unreachable!("Unexpected math operator str: {}", op_str),
                };
                res = Expr::BinaryMath {
                    op,
                    left: Box::new(res),
                    right: Box::new(right),
                    range,
                    op_range,
                };
            }
            Rule::deref => {
                // If we get a deref here, it's likely a member access being parsed in a binary context
                // This shouldn't happen with the current grammar but let's handle it gracefully.
                if let Expr::Identifier(id) = right {
                    res = Expr::Postfix {
                        expr: Box::new(res),
                        ops: vec![PostfixOp::Deref(id)],
                        range,
                    };
                } else {
                    res = right;
                }
            }
            Rule::paren_open => {
                // This might happen if process_binary is called on something that isn't really a binary expr with these operators
                // but Pest gave us these pairs. For now, just skip it.
                continue;
            }
            Rule::variable
            | Rule::assignment_expr
            | Rule::logical_or_expr
            | Rule::logical_and_expr
            | Rule::equality_expr
            | Rule::comparison_expr
            | Rule::bitwise_expr
            | Rule::additive_expr
            | Rule::multiplicative_expr
            | Rule::unary_expr
            | Rule::postfix_expr
            | Rule::primary_expr => {
                // This could happen if an expression is parsed in a binary context without an operator pair between them
                // We'll just replace the current result with this expression.
                res = process_expr(op_pair);
            }
            Rule::line_comment
            | Rule::block_comment
            | Rule::statement_method_arguments
            | Rule::comma
            | Rule::paren_close
            | Rule::bracket_close
            | Rule::brace_close
            | Rule::array_subscript => {
                continue;
            }
            Rule::keyword_me => {
                res = Expr::Identifier(crate::gs::Identifier {
                    name: "me".to_string(),
                    range: pair_to_range(&op_pair),
                });
            }
            _ => unreachable!("Unexpected operator: {:?}", op_pair.as_rule()),
        }
    }
    res
}

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
            if let Expr::Identifier(id) = next_expr {
                ops.push(PostfixOp::Deref(id));
            } else {
                panic!("Deref must be followed by identifier, got {:?}", next_expr);
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

fn process_primary_expr(pair: Pair<Rule>) -> Expr {
    let range = pair_to_range(&pair);
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
        Rule::keyword_is_class => Expr::IsClass(range),
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
