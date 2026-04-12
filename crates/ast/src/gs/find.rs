use crate::Position;
use crate::find::{HasRange, position_in_range};
use crate::gs::program::Program;
use crate::gs::{
    Block, ClassDef, Expr, FieldDef, Identifier, LoopBody, MethodDef, PostfixOp, Stmt, Type,
    TypeOrVoid,
};
use rayon::prelude::*;
use tracing::trace;

pub fn find_postfix_at_position(program: &Program, pos: Position) -> Option<(&Expr, usize)> {
    program
        .classes
        .par_iter()
        .find_map_any(|(_, class)| find_postfix_in_class(class, pos))
}

fn find_postfix_in_class(class: &ClassDef, pos: Position) -> Option<(&Expr, usize)> {
    for field in class.fields.values() {
        if let Some(init) = &field.initializer
            && let Some(res) = find_postfix_in_expr(init, pos)
        {
            return Some(res);
        }
    }
    for methods in class.methods.values() {
        for method in methods {
            if let Some(body) = &method.body
                && let Some(res) = find_postfix_in_block(body, pos)
            {
                return Some(res);
            }
        }
    }
    None
}

fn find_postfix_in_block(block: &Block, pos: Position) -> Option<(&Expr, usize)> {
    for stmt in &block.statements {
        if position_in_range(pos, stmt.range())
            && let Some(res) = find_postfix_in_stmt(stmt, pos)
        {
            return Some(res);
        }
    }
    None
}

fn find_postfix_in_stmt(stmt: &Stmt, pos: Position) -> Option<(&Expr, usize)> {
    match stmt {
        Stmt::Return(expr, _, _) => expr.as_ref().and_then(|e| find_postfix_in_expr(e, pos)),
        Stmt::Expr(expr) => find_postfix_in_expr(expr, pos),
        Stmt::Decl(decl) => {
            for val in &decl.values {
                if let Some(res) = find_postfix_in_expr(val, pos) {
                    return Some(res);
                }
            }
            None
        }
        Stmt::If(if_stmt) => find_postfix_in_expr(&if_stmt.cond, pos)
            .or_else(|| find_postfix_in_block(&if_stmt.then_block, pos))
            .or_else(|| {
                if_stmt
                    .else_block
                    .as_ref()
                    .and_then(|b| find_postfix_in_block(b, pos))
            }),
        Stmt::While(while_stmt) => {
            find_postfix_in_expr(&while_stmt.cond, pos).or_else(|| match &while_stmt.body {
                LoopBody::Empty(_) => None,
                LoopBody::Block(block) => find_postfix_in_block(block, pos),
            })
        }
        Stmt::For(for_stmt) => find_postfix_in_expr(&for_stmt.init.target, pos)
            .or_else(|| find_postfix_in_expr(&for_stmt.init.value, pos))
            .or_else(|| find_postfix_in_expr(&for_stmt.cond, pos))
            .or_else(|| {
                for_stmt
                    .step
                    .as_ref()
                    .and_then(|s| find_postfix_in_expr(s, pos))
            })
            .or_else(|| match &for_stmt.body {
                LoopBody::Empty(_) => None,
                LoopBody::Block(block) => find_postfix_in_block(block, pos),
            }),
        Stmt::Wait(wait_stmt) => find_postfix_in_block(&wait_stmt.body, pos),
        Stmt::On(on_stmt) => find_postfix_in_block(&on_stmt.body, pos),
        Stmt::Switch(switch_stmt) => find_postfix_in_expr(&switch_stmt.expr, pos).or_else(|| {
            for case in &switch_stmt.cases {
                if let Some(res) = find_postfix_in_expr(&case.value, pos)
                    .or_else(|| find_postfix_in_block(&case.body, pos))
                {
                    return Some(res);
                }
            }
            switch_stmt
                .default
                .as_ref()
                .and_then(|d| find_postfix_in_block(d, pos))
        }),
        Stmt::Block(block) => {
            if position_in_range(pos, block.range) {
                find_postfix_in_block(block, pos)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn find_postfix_in_expr(expr: &Expr, pos: Position) -> Option<(&Expr, usize)> {
    match expr {
        Expr::Assign { left, right, .. } => {
            find_postfix_in_expr(left, pos).or_else(|| find_postfix_in_expr(right, pos))
        }
        Expr::LogicalOr { left, right, .. } => {
            find_postfix_in_expr(left, pos).or_else(|| find_postfix_in_expr(right, pos))
        }
        Expr::LogicalAnd { left, right, .. } => {
            find_postfix_in_expr(left, pos).or_else(|| find_postfix_in_expr(right, pos))
        }
        Expr::Equality { left, right, .. } => {
            find_postfix_in_expr(left, pos).or_else(|| find_postfix_in_expr(right, pos))
        }
        Expr::Comparison { left, right, .. } => {
            find_postfix_in_expr(left, pos).or_else(|| find_postfix_in_expr(right, pos))
        }
        Expr::Bitwise { left, right, .. } => {
            find_postfix_in_expr(left, pos).or_else(|| find_postfix_in_expr(right, pos))
        }
        Expr::BinaryMath { left, right, .. } => {
            find_postfix_in_expr(left, pos).or_else(|| find_postfix_in_expr(right, pos))
        }
        Expr::Unary { expr, .. } => find_postfix_in_expr(expr, pos),
        Expr::Postfix {
            expr: inner, ops, ..
        } => {
            if let Some(res) = find_postfix_in_expr(inner, pos) {
                return Some(res);
            }
            for (idx, op) in ops.iter().enumerate() {
                match op {
                    PostfixOp::Deref(id) => {
                        if position_in_range(pos, id.range) {
                            return Some((expr, idx));
                        }
                    }
                    PostfixOp::Call(args, range) => {
                        if position_in_range(pos, *range) {
                            for arg in args {
                                if let Some(res) = find_postfix_in_expr(arg, pos) {
                                    return Some(res);
                                }
                            }
                        }
                    }
                    PostfixOp::Index(args, range) => {
                        if position_in_range(pos, *range) {
                            for arg in args {
                                if let Some(res) = find_postfix_in_expr(arg, pos) {
                                    return Some(res);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            None
        }
        Expr::Cast { expr: inner, .. } => find_postfix_in_expr(inner, pos),
        Expr::NewObject { args, .. } => {
            for arg in args {
                if let Some(res) = find_postfix_in_expr(arg, pos) {
                    return Some(res);
                }
            }
            None
        }
        Expr::NewArray { size, .. } => find_postfix_in_expr(size, pos),
        Expr::Grouped(inner, _) => find_postfix_in_expr(inner, pos),
        _ => None,
    }
}

pub fn find_method_at_position<'a>(program: &'a Program, pos: Position) -> Option<&'a MethodDef> {
    program.classes.values().find_map(|cls| {
        if position_in_range(pos, cls.range) {
            for methods in cls.methods.values() {
                for method in methods {
                    if position_in_range(pos, method.range) {
                        return Some(method);
                    }
                }
            }
        }
        None
    })
}

pub fn find_class_at_position<'a>(program: &'a Program, pos: Position) -> Option<&'a ClassDef> {
    program
        .classes
        .values()
        .find(|cls| position_in_range(pos, cls.range))
}

pub fn find_field_by_id_range<'a>(
    program: &'a Program,
    range: crate::Range,
) -> Option<&'a FieldDef> {
    program
        .classes
        .values()
        .find_map(|cls| cls.fields.values().find(|field| field.name.range == range))
}

pub fn find_method_by_id_range<'a>(
    program: &'a Program,
    range: crate::Range,
) -> Option<&'a MethodDef> {
    program.classes.values().find_map(|cls| {
        cls.methods
            .values()
            .find_map(|methods| methods.iter().find(|method| method.name.range == range))
    })
}

pub fn find_param_by_id_range<'a>(
    program: &'a Program,
    range: crate::Range,
) -> Option<&'a crate::gs::Param> {
    program.classes.values().find_map(|cls| {
        cls.methods.values().find_map(|methods| {
            methods
                .iter()
                .find_map(|method| method.params.iter().find(|param| param.name.range == range))
        })
    })
}

pub fn find_id_at_position(program: &Program, pos: Position) -> Option<&Identifier> {
    trace!("find_id_at_position: searching for position {:?}", pos);
    program.classes.par_iter().find_map_any(|(_, class)| {
        trace!(
            "find_id_at_position: checking class {} with range {:?}",
            class.name.name, class.range
        );
        find_in_class(class, pos)
    })
}

fn find_in_class(class: &ClassDef, pos: Position) -> Option<&Identifier> {
    if position_in_range(pos, class.name.range) {
        return Some(&class.name);
    }
    for sup in &class.superclasses {
        if position_in_range(pos, sup.range) {
            return Some(sup);
        }
    }
    for field in class.fields.values() {
        if let Some(id) = find_in_field(field, pos) {
            return Some(id);
        }
    }
    for methods in class.methods.values() {
        for method in methods {
            if let Some(id) = find_in_method(method, pos) {
                return Some(id);
            }
        }
    }
    None
}

fn find_in_field(field: &FieldDef, pos: Position) -> Option<&Identifier> {
    if let Some(id) = find_in_type(&field.ty, pos) {
        return Some(id);
    }
    if position_in_range(pos, field.name.range) {
        return Some(&field.name);
    }
    if let Some(init) = &field.initializer
        && let Some(id) = find_in_expr(init, pos)
    {
        return Some(id);
    }
    None
}

fn find_in_method(method: &MethodDef, pos: Position) -> Option<&Identifier> {
    trace!(
        "find_in_method: checking method {} with range {:?}",
        method.name.name, method.range
    );
    if let TypeOrVoid::Type(ty) = &method.return_type {
        if let Some(id) = find_in_type(ty, pos) {
            return Some(id);
        }
    }
    if position_in_range(pos, method.name.range) {
        return Some(&method.name);
    }
    for param in &method.params {
        if let Some(id) = find_in_type(&param.ty, pos) {
            return Some(id);
        }
        if position_in_range(pos, param.name.range) {
            return Some(&param.name);
        }
    }
    if let Some(body) = &method.body {
        find_in_block(body, pos)
    } else {
        None
    }
}

fn find_in_block(block: &Block, pos: Position) -> Option<&Identifier> {
    trace!(
        "find_in_block: checking block with range {:?}, statements: {}",
        block.range,
        block.statements.len()
    );
    for stmt in &block.statements {
        trace!(
            "find_in_block: checking statement rule {:?} with range {:?}",
            stmt,
            stmt.range()
        );
        if position_in_range(pos, stmt.range())
            && let Some(id) = find_in_stmt(stmt, pos)
        {
            return Some(id);
        }
    }
    None
}

fn find_in_stmt(stmt: &Stmt, pos: Position) -> Option<&Identifier> {
    trace!("find_in_stmt checking rule: {:?}", stmt);
    match stmt {
        Stmt::Label(id, _, _) => {
            if position_in_range(pos, id.range) {
                Some(id)
            } else {
                None
            }
        }
        Stmt::Decl(decl) => {
            if let Some(id) = find_in_type(&decl.ty, pos) {
                return Some(id);
            }
            for name in &decl.names {
                if position_in_range(pos, name.range) {
                    return Some(name);
                }
            }
            for val in &decl.values {
                if let Some(id) = find_in_expr(val, pos) {
                    return Some(id);
                }
            }
            None
        }
        Stmt::Return(expr, _, _) => expr.as_ref().and_then(|e| find_in_expr(e, pos)),
        Stmt::Break(_, _) => None,
        Stmt::Continue(_, _) => None,
        Stmt::Goto(id, _, _) => {
            if position_in_range(pos, id.range) {
                Some(id)
            } else {
                None
            }
        }
        Stmt::Expr(expr) => find_in_expr(expr, pos),
        Stmt::If(if_stmt) => {
            if let Some(id) = find_in_expr(&if_stmt.cond, pos) {
                return Some(id);
            }
            if let Some(id) = find_in_block(&if_stmt.then_block, pos) {
                return Some(id);
            }
            if let Some(else_block) = &if_stmt.else_block {
                return find_in_block(else_block, pos);
            }
            None
        }
        Stmt::While(while_stmt) => {
            if let Some(id) = find_in_expr(&while_stmt.cond, pos) {
                return Some(id);
            }
            match &while_stmt.body {
                LoopBody::Empty(_) => None,
                LoopBody::Block(block) => find_in_block(block, pos),
            }
        }
        Stmt::For(for_stmt) => {
            if let Some(id) = find_in_expr(&for_stmt.init.target, pos) {
                return Some(id);
            }
            if let Some(id) = find_in_expr(&for_stmt.init.value, pos) {
                return Some(id);
            }
            if let Some(id) = find_in_expr(&for_stmt.cond, pos) {
                return Some(id);
            }
            if let Some(step) = &for_stmt.step
                && let Some(id) = find_in_expr(step, pos)
            {
                return Some(id);
            }
            match &for_stmt.body {
                LoopBody::Empty(_) => None,
                LoopBody::Block(block) => find_in_block(block, pos),
            }
        }
        Stmt::Wait(wait_stmt) => find_in_block(&wait_stmt.body, pos),
        Stmt::On(on_stmt) => {
            if let Some(id) = &on_stmt.identifier
                && position_in_range(pos, id.range)
            {
                return Some(id);
            }
            find_in_block(&on_stmt.body, pos)
        }
        Stmt::Switch(switch_stmt) => {
            if let Some(id) = find_in_expr(&switch_stmt.expr, pos) {
                return Some(id);
            }
            for case in &switch_stmt.cases {
                if let Some(id) = find_in_expr(&case.value, pos) {
                    return Some(id);
                }
                if let Some(id) = find_in_block(&case.body, pos) {
                    return Some(id);
                }
            }
            if let Some(default) = &switch_stmt.default {
                return find_in_block(default, pos);
            }
            None
        }
        Stmt::Block(block) => {
            if position_in_range(pos, block.range) {
                find_in_block(block, pos)
            } else {
                None
            }
        }
    }
}

fn find_in_expr(expr: &Expr, pos: Position) -> Option<&Identifier> {
    trace!("find_in_expr: checking expr type");
    match expr {
        Expr::Assign { left, right, .. } => {
            find_in_expr(left, pos).or_else(|| find_in_expr(right, pos))
        }
        Expr::LogicalOr { left, right, .. } => {
            find_in_expr(left, pos).or_else(|| find_in_expr(right, pos))
        }
        Expr::LogicalAnd { left, right, .. } => {
            find_in_expr(left, pos).or_else(|| find_in_expr(right, pos))
        }
        Expr::Equality { left, right, .. } => {
            find_in_expr(left, pos).or_else(|| find_in_expr(right, pos))
        }
        Expr::Comparison { left, right, .. } => {
            find_in_expr(left, pos).or_else(|| find_in_expr(right, pos))
        }
        Expr::Bitwise { left, right, .. } => {
            find_in_expr(left, pos).or_else(|| find_in_expr(right, pos))
        }
        Expr::BinaryMath { left, right, .. } => {
            find_in_expr(left, pos).or_else(|| find_in_expr(right, pos))
        }
        Expr::Unary { expr, .. } => find_in_expr(expr, pos),
        Expr::Postfix { expr, ops, .. } => {
            if let Some(id) = find_in_expr(expr, pos) {
                return Some(id);
            }
            for op in ops {
                match op {
                    PostfixOp::Deref(id) => {
                        if position_in_range(pos, id.range) {
                            return Some(id);
                        }
                    }
                    PostfixOp::Call(args, range) => {
                        if position_in_range(pos, *range) {
                            for arg in args {
                                if let Some(id) = find_in_expr(arg, pos) {
                                    return Some(id);
                                }
                            }
                        }
                    }
                    PostfixOp::Index(args, range) => {
                        if position_in_range(pos, *range) {
                            for arg in args {
                                if let Some(id) = find_in_expr(arg, pos) {
                                    return Some(id);
                                }
                            }
                        }
                    }
                    PostfixOp::Unary(_, _) => {}
                }
            }
            None
        }
        Expr::Cast { ty, expr, .. } => find_in_type(ty, pos).or_else(|| find_in_expr(expr, pos)),
        Expr::NewObject { ty, args, .. } => {
            if let Some(id) = find_in_type(ty, pos) {
                return Some(id);
            }
            for arg in args {
                if let Some(id) = find_in_expr(arg, pos) {
                    return Some(id);
                }
            }
            None
        }
        Expr::NewArray { ty, size, .. } => {
            find_in_type(ty, pos).or_else(|| find_in_expr(size, pos))
        }
        Expr::Literal(_) => None,
        Expr::IsClass(id) => {
            if position_in_range(pos, id.range) {
                Some(id)
            } else {
                None
            }
        }
        Expr::Identifier(id) => {
            if position_in_range(pos, id.range) {
                Some(id)
            } else {
                None
            }
        }
        Expr::Grouped(expr, _) => find_in_expr(expr, pos),
    }
}

fn find_in_type(ty: &Type, pos: Position) -> Option<&Identifier> {
    match ty {
        Type::Named(id) => {
            if position_in_range(pos, id.range) {
                Some(id)
            } else {
                None
            }
        }
        Type::Array(inner, _) => find_in_type(inner, pos),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gs::process::process_trainz_ast;

    #[test]
    fn test_find_in_stmt_edge_cases() {
        let code = r#"class Test {
            void method() {
                switch (myVar) {
                    case 1: {
                        int innerVar;
                        break;
                    }
                }
                while (false);
                wait() {
                    on "Event", "Name" {
                        int onVar;
                    }
                }
            }
        };"#;
        let pairs = trainz_parser::gs::parse(code).unwrap();
        let program = process_trainz_ast(pairs, code);

        let find_pos = |name: &str| -> Position {
            for (i, line) in code.lines().enumerate() {
                if let Some(idx) = line.find(name) {
                    return Position {
                        line: i as u32,
                        character: idx as u32,
                    };
                }
            }
            panic!("Could not find string {}", name);
        };

        // find `myVar`
        let id = find_id_at_position(&program, find_pos("myVar")).unwrap();
        assert_eq!(id.name, "myVar");

        // find `onVar`
        let id = find_id_at_position(&program, find_pos("onVar")).unwrap();
        assert_eq!(id.name, "onVar");
    }
}
