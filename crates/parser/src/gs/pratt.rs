use pest::pratt_parser::Assoc;
use pest::pratt_parser::{Op, PrattParser};

use crate::gs::grammar::Rule;

lazy_static::lazy_static! {
    pub static ref PRATT: PrattParser<Rule> = PrattParser::new()
        // -------------------------
        // Prefix (unary LHS)
        // -------------------------
        .op(Op::prefix(Rule::operator_unary_not))
        .op(Op::prefix(Rule::operator_unary_not_not))
        .op(Op::prefix(Rule::operator_unary_increment))
        .op(Op::prefix(Rule::operator_unary_decrement))
        .op(Op::prefix(Rule::operator_unary_positive))
        .op(Op::prefix(Rule::operator_unary_negative))

        // -------------------------
        // Postfix (highest precedence)
        // postfix_op = deref | call | index | ++ | --
        // -------------------------
        .op(Op::postfix(Rule::operator_unary_rhs))      // ++ --
        .op(Op::postfix(Rule::deref))                   // .identifier
        .op(Op::postfix(Rule::paren_open))              // function call (...)
        .op(Op::postfix(Rule::bracket_open))            // array index [...]

        // -------------------------
        // Multiplicative
        // -------------------------
        .op(Op::infix(Rule::operator_math_multiply, Assoc::Left)
            | Op::infix(Rule::operator_math_divide, Assoc::Left)
            | Op::infix(Rule::operator_math_modulo, Assoc::Left))

        // -------------------------
        // Additive
        // -------------------------
        .op(Op::infix(Rule::operator_math_add, Assoc::Left)
            | Op::infix(Rule::operator_math_subtract, Assoc::Left))

        // -------------------------
        // Bitwise
        // -------------------------
        .op(Op::infix(Rule::operator_bitwise, Assoc::Left))

        // -------------------------
        // Comparison
        // -------------------------
        .op(Op::infix(Rule::operator_comparison, Assoc::Left))

        // -------------------------
        // Equality
        // -------------------------
        .op(Op::infix(Rule::operator_equality, Assoc::Left))

        // -------------------------
        // Logical AND / OR
        // -------------------------
        .op(Op::infix(Rule::operator_logical_and, Assoc::Left))
        .op(Op::infix(Rule::operator_logical_or, Assoc::Left))

        // -------------------------
        // Assignment (=) — lowest precedence, right associative
        // -------------------------
        .op(Op::infix(Rule::assignment_expr, Assoc::Right));
}
