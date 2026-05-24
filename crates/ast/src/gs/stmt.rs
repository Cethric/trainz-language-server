use crate::find::HasRange;
use crate::gs::expr::Expr;
use crate::gs::literal::{Identifier, StringLiteral};
use crate::gs::types::Type;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub statements: Vec<Stmt>,
    pub scope_id: usize,
    pub range: crate::Range,
}

impl HasRange for Block {
    fn range(&self) -> crate::Range {
        self.range
    }
}

impl HasRange for Stmt {
    fn range(&self) -> crate::Range {
        match self {
            Stmt::Label(_, _, range) => *range,
            Stmt::Decl(decl) => decl.range,
            Stmt::Return(_, _, range) => *range,
            Stmt::Break(_, range) => *range,
            Stmt::Continue(_, range) => *range,
            Stmt::Goto(_, _, range) => *range,
            Stmt::Expr(expr) => expr.range(),
            Stmt::If(if_stmt) => if_stmt.range,
            Stmt::While(while_stmt) => while_stmt.range,
            Stmt::For(for_stmt) => for_stmt.range,
            Stmt::Wait(wait_stmt) => wait_stmt.range,
            Stmt::On(on_stmt) => on_stmt.range,
            Stmt::Switch(switch_stmt) => switch_stmt.range,
            Stmt::Block(block) => block.range,
        }
    }
}

/// Represents a statement in a GS program.
///
/// # Examples
///
/// ```rust
/// use trainz_ast::gs::stmt::Stmt;
/// use trainz_ast::Range;
///
/// let stmt = Stmt::Break(Range::default(), Range::default());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Stmt {
    /// A labeled statement (e.g., `label: ;` or `label:`)
    Label(Identifier, crate::Range, crate::Range), // id, colon range, stmt range
    /// A variable declaration (e.g., `int x = 1;`)
    Decl(Decl),
    /// A return statement (e.g., `return 1;` or `return;`)
    Return(Option<Expr>, crate::Range, crate::Range), // expr, keyword range, stmt range
    /// A break statement (e.g., `break;`)
    Break(crate::Range, crate::Range), // keyword range, stmt range
    /// A continue statement (e.g., `continue;`)
    Continue(crate::Range, crate::Range), // keyword range, stmt range
    /// A goto statement (e.g., `goto label;`)
    Goto(Identifier, crate::Range, crate::Range), // identifier, keyword range, stmt range
    /// An expression statement (e.g., `x = 1;`)
    Expr(Expr),
    /// An if statement (e.g., `if (cond) { ... }`)
    If(Box<IfStmt>),
    /// A while loop (e.g., `while (cond) { ... }`)
    While(Box<WhileStmt>),
    /// A for loop (e.g., `for (init; cond; step) { ... }`)
    For(Box<ForStmt>),
    /// A wait statement (e.g., `wait() { ... }`)
    Wait(Box<WaitStmt>),
    /// An on-event statement (e.g., `on("event", "target") { ... }`)
    On(Box<OnStmt>),
    /// A switch statement (e.g., `switch (expr) { case 1: ... }`)
    Switch(Box<SwitchStmt>),
    /// A block statement (e.g., `{ ... }`)
    Block(Block),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decl {
    pub ty: Type,
    pub names: Vec<Identifier>,
    pub values: Vec<Expr>, // may be fewer than names
    pub range: crate::Range,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfStmt {
    pub cond: Expr,
    pub keyword_if_range: crate::Range,
    pub then_block: Block,
    pub keyword_else_range: Option<crate::Range>,
    pub else_block: Option<Block>,
    pub range: crate::Range,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhileStmt {
    pub cond: Expr,
    pub keyword_while_range: crate::Range,
    pub body: LoopBody,
    pub range: crate::Range,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForStmt {
    pub init: AssignStmt,
    pub cond: Expr,
    pub step: Option<Expr>,
    pub keyword_for_range: crate::Range,
    pub body: LoopBody,
    pub range: crate::Range,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitStmt {
    pub keyword_wait_range: crate::Range,
    pub body: Block,
    pub range: crate::Range,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnStmt {
    pub keyword_on_range: crate::Range,
    pub event: StringLiteral,
    pub target: StringLiteral,
    pub identifier: Option<Identifier>,
    pub body: Block,
    pub range: crate::Range,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchStmt {
    pub keyword_switch_range: crate::Range,
    pub expr: Expr,
    pub cases: Vec<Case>,
    pub keyword_default_range: Option<crate::Range>,
    pub default: Option<Block>,
    pub range: crate::Range,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Case {
    pub keyword_case_range: crate::Range,
    pub value: Expr, // postfix_expr in grammar
    pub body: Block,
    pub range: crate::Range,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoopBody {
    Empty(crate::Range), // when only semicolon
    Block(Block),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignStmt {
    pub target: Expr, // postfix_expr
    pub value: Expr,  // includes keyword_boolean/new_array/new_object/expr
    pub range: crate::Range,
}
