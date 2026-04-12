use tower_lsp_server::ls_types::{DocumentSymbol, SymbolKind};
use trainz_ast::gs::{Block, Stmt};
use trainz_common::range::clamp_range;

use super::expr::process_expr;

#[allow(deprecated)]
#[tracing::instrument]
pub(crate) fn process_block(body: &Block) -> Vec<DocumentSymbol> {
    let mut symbols = vec![];
    for statement in body.statements.clone() {
        match &statement {
            Stmt::Label(label, _, _) => {
                symbols.push(DocumentSymbol {
                    name: label.name.clone(),
                    detail: None,
                    kind: SymbolKind::VARIABLE,
                    tags: None,
                    deprecated: None,
                    range: label.range,
                    selection_range: label.range,
                    children: None,
                });
            }
            Stmt::Decl(decl) => {
                for (name, value) in decl.names.iter().zip(decl.values.iter()) {
                    symbols.push(DocumentSymbol {
                        name: name.name.clone(),
                        detail: Some(format!("{:?}", decl.ty)),
                        kind: SymbolKind::VARIABLE,
                        tags: None,
                        deprecated: None,
                        range: decl.range,
                        selection_range: clamp_range(&decl.range, name.range),
                        children: None,
                    });
                    symbols.extend(process_expr(value));
                }
                // Handle names without initial values
                if decl.names.len() > decl.values.len() {
                    for name in &decl.names[decl.values.len()..] {
                        symbols.push(DocumentSymbol {
                            name: name.name.clone(),
                            detail: Some(format!("{:?}", decl.ty)),
                            kind: SymbolKind::VARIABLE,
                            tags: None,
                            deprecated: None,
                            range: decl.range,
                            selection_range: clamp_range(&decl.range, name.range),
                            children: None,
                        });
                    }
                }
            }
            Stmt::If(if_stmt) => {
                let mut children = process_expr(&if_stmt.cond);
                let then_children = process_block(&if_stmt.then_block);
                children.extend(then_children);
                if let Some(else_block) = &if_stmt.else_block {
                    children.extend(process_block(else_block));
                }
                symbols.push(DocumentSymbol {
                    name: "if".to_string(),
                    detail: None,
                    kind: SymbolKind::NAMESPACE,
                    tags: None,
                    deprecated: None,
                    range: if_stmt.range,
                    selection_range: if_stmt.range,
                    children: if children.is_empty() {
                        None
                    } else {
                        Some(children)
                    },
                });
            }
            Stmt::While(while_stmt) => {
                let mut children = process_expr(&while_stmt.cond);
                match &while_stmt.body {
                    trainz_ast::gs::LoopBody::Empty(_) => {}
                    trainz_ast::gs::LoopBody::Block(block) => children.extend(process_block(block)),
                };
                symbols.push(DocumentSymbol {
                    name: "while".to_string(),
                    detail: None,
                    kind: SymbolKind::NAMESPACE,
                    tags: None,
                    deprecated: None,
                    range: while_stmt.range,
                    selection_range: while_stmt.range,
                    children: if children.is_empty() {
                        None
                    } else {
                        Some(children)
                    },
                });
            }
            Stmt::For(for_stmt) => {
                let mut children = process_expr(&for_stmt.init.target);
                children.extend(process_expr(&for_stmt.init.value));
                children.extend(process_expr(&for_stmt.cond));
                if let Some(step) = &for_stmt.step {
                    children.extend(process_expr(step));
                }
                match &for_stmt.body {
                    trainz_ast::gs::LoopBody::Empty(_) => {}
                    trainz_ast::gs::LoopBody::Block(block) => children.extend(process_block(block)),
                };
                symbols.push(DocumentSymbol {
                    name: "for".to_string(),
                    detail: None,
                    kind: SymbolKind::NAMESPACE,
                    tags: None,
                    deprecated: None,
                    range: for_stmt.range,
                    selection_range: for_stmt.range,
                    children: if children.is_empty() {
                        None
                    } else {
                        Some(children)
                    },
                });
            }
            Stmt::Wait(wait_stmt) => {
                let children = process_block(&wait_stmt.body);
                symbols.push(DocumentSymbol {
                    name: "wait".to_string(),
                    detail: None,
                    kind: SymbolKind::NAMESPACE,
                    tags: None,
                    deprecated: None,
                    range: wait_stmt.range,
                    selection_range: wait_stmt.range,
                    children: if children.is_empty() {
                        None
                    } else {
                        Some(children)
                    },
                });
            }
            Stmt::On(on_stmt) => {
                let children = process_block(&on_stmt.body);
                symbols.push(DocumentSymbol {
                    name: format!("on {}", on_stmt.event.value),
                    detail: None,
                    kind: SymbolKind::NAMESPACE,
                    tags: None,
                    deprecated: None,
                    range: on_stmt.range,
                    selection_range: on_stmt.range,
                    children: if children.is_empty() {
                        None
                    } else {
                        Some(children)
                    },
                });
            }
            Stmt::Switch(switch_stmt) => {
                let mut children = process_expr(&switch_stmt.expr);
                for case in &switch_stmt.cases {
                    let case_children = process_block(&case.body);
                    children.push(DocumentSymbol {
                        name: "case".to_string(),
                        detail: None,
                        kind: SymbolKind::NAMESPACE,
                        tags: None,
                        deprecated: None,
                        range: case.range,
                        selection_range: case.range,
                        children: if case_children.is_empty() {
                            None
                        } else {
                            Some(case_children)
                        },
                    });
                }
                if let Some(default) = &switch_stmt.default {
                    let default_children = process_block(default);
                    children.push(DocumentSymbol {
                        name: "default".to_string(),
                        detail: None,
                        kind: SymbolKind::NAMESPACE,
                        tags: None,
                        deprecated: None,
                        range: default.range,
                        selection_range: default.range,
                        children: if default_children.is_empty() {
                            None
                        } else {
                            Some(default_children)
                        },
                    });
                }
                symbols.push(DocumentSymbol {
                    name: "switch".to_string(),
                    detail: None,
                    kind: SymbolKind::NAMESPACE,
                    tags: None,
                    deprecated: None,
                    range: switch_stmt.range,
                    selection_range: switch_stmt.range,
                    children: if children.is_empty() {
                        None
                    } else {
                        Some(children)
                    },
                });
            }
            Stmt::Block(block) => {
                symbols.extend(process_block(block));
            }
            Stmt::Expr(expr) => {
                symbols.extend(process_expr(expr));
            }
            Stmt::Return(expr, _, range) => {
                let mut children = vec![];
                if let Some(expr) = expr {
                    children.extend(process_expr(expr));
                }
                symbols.extend(children.clone());
                symbols.push(DocumentSymbol {
                    name: "return".to_string(),
                    detail: None,
                    kind: SymbolKind::VARIABLE,
                    tags: None,
                    deprecated: None,
                    range: *range,
                    selection_range: *range,
                    children: if children.is_empty() {
                        None
                    } else {
                        Some(children)
                    },
                });
            }
            _ => {}
        }
    }

    symbols
}
