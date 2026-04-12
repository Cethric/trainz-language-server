use crate::util::add_folding_range;
use tower_lsp_server::ls_types::FoldingRange;
use trainz_ast::gs::{Block, LoopBody, Stmt};

#[tracing::instrument]
pub fn collect_block_folding_ranges(block: &Block, result: &mut Vec<FoldingRange>) {
    for stmt in &block.statements {
        match stmt {
            Stmt::If(if_stmt) => {
                let range = tower_lsp_server::ls_types::Range {
                    start: if_stmt.then_block.range.start,
                    end: if_stmt.then_block.range.end,
                };
                add_folding_range(range, result, Some(String::from("{ ... }")));
                collect_block_folding_ranges(&if_stmt.then_block, result);
                if let Some(else_block) = &if_stmt.else_block {
                    let range = tower_lsp_server::ls_types::Range {
                        start: else_block.range.start,
                        end: else_block.range.end,
                    };
                    add_folding_range(range, result, Some(String::from("{ ... }")));
                    collect_block_folding_ranges(else_block, result);
                }
            }
            Stmt::While(while_stmt) => {
                if let LoopBody::Block(body_block) = &while_stmt.body {
                    let range = tower_lsp_server::ls_types::Range {
                        start: body_block.range.start,
                        end: body_block.range.end,
                    };
                    add_folding_range(range, result, Some(String::from("{ ... }")));
                    collect_block_folding_ranges(body_block, result);
                }
            }
            Stmt::For(for_stmt) => {
                if let LoopBody::Block(body_block) = &for_stmt.body {
                    let range = tower_lsp_server::ls_types::Range {
                        start: body_block.range.start,
                        end: body_block.range.end,
                    };
                    add_folding_range(range, result, Some(String::from("{ ... }")));
                    collect_block_folding_ranges(body_block, result);
                }
            }
            Stmt::Wait(wait_stmt) => {
                let range = tower_lsp_server::ls_types::Range {
                    start: wait_stmt.body.range.start,
                    end: wait_stmt.body.range.end,
                };
                add_folding_range(range, result, Some(String::from("{ ... }")));
                collect_block_folding_ranges(&wait_stmt.body, result);
            }
            Stmt::On(on_stmt) => {
                let range = tower_lsp_server::ls_types::Range {
                    start: on_stmt.body.range.start,
                    end: on_stmt.body.range.end,
                };
                add_folding_range(range, result, Some(String::from("{ ... }")));
                collect_block_folding_ranges(&on_stmt.body, result);
            }
            Stmt::Switch(switch_stmt) => {
                for case in &switch_stmt.cases {
                    let range = tower_lsp_server::ls_types::Range {
                        start: case.body.range.start,
                        end: case.body.range.end,
                    };
                    add_folding_range(range, result, Some(String::from("{ ... }")));
                    collect_block_folding_ranges(&case.body, result);
                }
                if let Some(default_block) = &switch_stmt.default {
                    let range = tower_lsp_server::ls_types::Range {
                        start: default_block.range.start,
                        end: default_block.range.end,
                    };
                    add_folding_range(range, result, Some(String::from("{ ... }")));
                    collect_block_folding_ranges(default_block, result);
                }
            }
            Stmt::Block(inner_block) => {
                let range = tower_lsp_server::ls_types::Range {
                    start: inner_block.range.start,
                    end: inner_block.range.end,
                };
                add_folding_range(range, result, Some(String::from("{ ... }")));
                collect_block_folding_ranges(inner_block, result);
            }
            _ => {}
        }
    }
}
