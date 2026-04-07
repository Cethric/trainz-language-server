use crate::find::HasRange;
use crate::gs::literal::{Identifier, Literal};
use crate::gs::types::Type;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Expr {
    Assign {
        left: Box<Expr>,
        right: Box<Expr>,
        range: crate::Range,
        op_range: crate::Range,
    },
    LogicalOr {
        left: Box<Expr>,
        right: Box<Expr>,
        range: crate::Range,
        op_range: crate::Range,
    },
    LogicalAnd {
        left: Box<Expr>,
        right: Box<Expr>,
        range: crate::Range,
        op_range: crate::Range,
    },
    Equality {
        op: EqualityOp,
        left: Box<Expr>,
        right: Box<Expr>,
        range: crate::Range,
        op_range: crate::Range,
    },
    Comparison {
        op: ComparisonOp,
        left: Box<Expr>,
        right: Box<Expr>,
        range: crate::Range,
        op_range: crate::Range,
    },
    Bitwise {
        op: BitwiseOp,
        left: Box<Expr>,
        right: Box<Expr>,
        range: crate::Range,
        op_range: crate::Range,
    },
    BinaryMath {
        op: MathOp,
        left: Box<Expr>,
        right: Box<Expr>,
        range: crate::Range,
        op_range: crate::Range,
    },
    Unary {
        op: UnaryPrefixOp,
        expr: Box<Expr>,
        range: crate::Range,
        op_range: crate::Range,
    },
    Postfix {
        expr: Box<Expr>,
        ops: Vec<PostfixOp>,
        range: crate::Range,
    },
    Cast {
        ty: Type,
        expr: Box<Expr>,
        range: crate::Range,
    },
    NewObject {
        ty: Type,
        args: Vec<Expr>,
        range: crate::Range,
    },
    NewArray {
        ty: Type,
        size: Box<Expr>, // integer | postfix_expr
        range: crate::Range,
    },
    Literal(Literal),
    IsClass(crate::Range),
    Identifier(Identifier),
    Grouped(Box<Expr>, crate::Range),
}

impl HasRange for Expr {
    fn range(&self) -> crate::Range {
        match self {
            Expr::Assign { range, .. } => *range,
            Expr::LogicalOr { range, .. } => *range,
            Expr::LogicalAnd { range, .. } => *range,
            Expr::Equality { range, .. } => *range,
            Expr::Comparison { range, .. } => *range,
            Expr::Bitwise { range, .. } => *range,
            Expr::BinaryMath { range, .. } => *range,
            Expr::Unary { range, .. } => *range,
            Expr::Postfix { range, .. } => *range,
            Expr::Cast { range, .. } => *range,
            Expr::NewObject { range, .. } => *range,
            Expr::NewArray { range, .. } => *range,
            Expr::Literal(lit) => lit.range(),
            Expr::IsClass(range) => *range,
            Expr::Identifier(id) => id.range,
            Expr::Grouped(_, range) => *range,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EqualityOp {
    Eq,
    Ne,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComparisonOp {
    Le,
    Ge,
    Lt,
    Gt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BitwiseOp {
    Shl,
    Shr,
    And,
    Or,
    Not,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MathOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnaryPrefixOp {
    Not,
    NotNot,
    Inc,
    Dec,
    Plus,
    Minus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnaryPostfixOp {
    Inc,
    Dec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PostfixOp {
    Deref(Identifier),
    Call(Vec<Expr>, crate::Range),
    Index(Vec<Expr>, crate::Range), // array_subscript normalized to Vec<Expr?>
    Unary(UnaryPostfixOp, crate::Range),
}
