use crate::gs::expr::collect_expr_tokens;
use crate::gs::types::collect_type_tokens;
use tower_lsp_server::ls_types::{Range, SemanticTokenModifier, SemanticTokenType};
use trainz_ast::gs::{LoopBody, Stmt};

#[tracing::instrument]
pub fn collect_stmt_tokens(
    stmt: &Stmt,
    raw_tokens: &mut Vec<(Range, SemanticTokenType, Vec<SemanticTokenModifier>)>,
    known_classes: &std::collections::HashSet<String>,
) {
    match stmt {
        Stmt::Label(id, colon_range, _) => {
            raw_tokens.push((id.range, SemanticTokenType::VARIABLE, vec![]));
            raw_tokens.push((*colon_range, SemanticTokenType::OPERATOR, vec![]));
        }
        Stmt::Decl(decl) => {
            collect_type_tokens(&decl.ty, raw_tokens);
            for name in &decl.names {
                raw_tokens.push((
                    name.range,
                    SemanticTokenType::VARIABLE,
                    vec![
                        SemanticTokenModifier::DECLARATION,
                        SemanticTokenModifier::DEFINITION,
                    ],
                ));
            }
            for val in &decl.values {
                collect_expr_tokens(val, raw_tokens, known_classes);
            }
        }
        Stmt::Return(expr, kw_range, _) => {
            raw_tokens.push((*kw_range, SemanticTokenType::KEYWORD, vec![]));
            if let Some(e) = expr {
                collect_expr_tokens(e, raw_tokens, known_classes);
            }
        }
        Stmt::Break(kw_range, _) => {
            raw_tokens.push((*kw_range, SemanticTokenType::KEYWORD, vec![]))
        }
        Stmt::Continue(kw_range, _) => {
            raw_tokens.push((*kw_range, SemanticTokenType::KEYWORD, vec![]))
        }
        Stmt::Goto(id, kw_range, _) => {
            raw_tokens.push((*kw_range, SemanticTokenType::KEYWORD, vec![]));
            raw_tokens.push((id.range, SemanticTokenType::VARIABLE, vec![]));
        }
        Stmt::Expr(expr) => collect_expr_tokens(expr, raw_tokens, known_classes),
        Stmt::If(if_stmt) => {
            raw_tokens.push((if_stmt.keyword_if_range, SemanticTokenType::KEYWORD, vec![]));
            collect_expr_tokens(&if_stmt.cond, raw_tokens, known_classes);

            for s in &if_stmt.then_block.statements {
                collect_stmt_tokens(s, raw_tokens, known_classes);
            }
            if let Some(else_kw_range) = if_stmt.keyword_else_range {
                raw_tokens.push((else_kw_range, SemanticTokenType::KEYWORD, vec![]));
            }
            if let Some(else_block) = &if_stmt.else_block {
                for s in &else_block.statements {
                    collect_stmt_tokens(s, raw_tokens, known_classes);
                }
            }
        }
        Stmt::While(while_stmt) => {
            raw_tokens.push((
                while_stmt.keyword_while_range,
                SemanticTokenType::KEYWORD,
                vec![],
            ));
            collect_expr_tokens(&while_stmt.cond, raw_tokens, known_classes);
            match &while_stmt.body {
                LoopBody::Block(b) => {
                    for s in &b.statements {
                        collect_stmt_tokens(s, raw_tokens, known_classes);
                    }
                }
                LoopBody::Empty(_) => {}
            }
        }
        Stmt::For(for_stmt) => {
            raw_tokens.push((
                for_stmt.keyword_for_range,
                SemanticTokenType::KEYWORD,
                vec![],
            ));
            collect_expr_tokens(&for_stmt.init.target, raw_tokens, known_classes);
            collect_expr_tokens(&for_stmt.init.value, raw_tokens, known_classes);
            collect_expr_tokens(&for_stmt.cond, raw_tokens, known_classes);
            if let Some(step) = &for_stmt.step {
                collect_expr_tokens(step, raw_tokens, known_classes);
            }
            match &for_stmt.body {
                LoopBody::Block(b) => {
                    for s in &b.statements {
                        collect_stmt_tokens(s, raw_tokens, known_classes);
                    }
                }
                LoopBody::Empty(_) => {}
            }
        }
        Stmt::Wait(wait_stmt) => {
            raw_tokens.push((
                wait_stmt.keyword_wait_range,
                SemanticTokenType::KEYWORD,
                vec![],
            ));
            for s in &wait_stmt.body.statements {
                collect_stmt_tokens(s, raw_tokens, known_classes);
            }
        }
        Stmt::On(on_stmt) => {
            raw_tokens.push((on_stmt.keyword_on_range, SemanticTokenType::KEYWORD, vec![]));
            raw_tokens.push((on_stmt.event.range, SemanticTokenType::STRING, vec![]));
            raw_tokens.push((on_stmt.target.range, SemanticTokenType::STRING, vec![]));
            if let Some(id) = &on_stmt.identifier {
                raw_tokens.push((id.range, SemanticTokenType::VARIABLE, vec![]));
            }
            for s in &on_stmt.body.statements {
                collect_stmt_tokens(s, raw_tokens, known_classes);
            }
        }
        Stmt::Switch(switch_stmt) => {
            raw_tokens.push((
                switch_stmt.keyword_switch_range,
                SemanticTokenType::KEYWORD,
                vec![],
            ));
            collect_expr_tokens(&switch_stmt.expr, raw_tokens, known_classes);
            for case in &switch_stmt.cases {
                raw_tokens.push((case.keyword_case_range, SemanticTokenType::KEYWORD, vec![]));
                collect_expr_tokens(&case.value, raw_tokens, known_classes);
                for s in &case.body.statements {
                    collect_stmt_tokens(s, raw_tokens, known_classes);
                }
            }
            if let Some(default_kw_range) = switch_stmt.keyword_default_range {
                raw_tokens.push((default_kw_range, SemanticTokenType::KEYWORD, vec![]));
            }
            if let Some(default) = &switch_stmt.default {
                for s in &default.statements {
                    collect_stmt_tokens(s, raw_tokens, known_classes);
                }
            }
        }
        Stmt::Block(block) => {
            for s in &block.statements {
                collect_stmt_tokens(s, raw_tokens, known_classes);
            }
        }
    }
}
