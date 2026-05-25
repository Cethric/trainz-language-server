#[cfg(test)]
pub mod tests;

use crate::util::add_folding_range;
use tower_lsp_server::ls_types::FoldingRange;
use trainz_ast::comments::{Comment, CommentProgram};

#[tracing::instrument(skip(program))]
pub fn comments_folding_range(program: &CommentProgram) -> Vec<FoldingRange> {
    let mut result = vec![];

    for comment in &program.comments {
        match comment {
            Comment::BlockComment(c) if c.range.start.line < c.range.end.line => {
                add_folding_range(c.range, &mut result, Some("/* ... */".to_string()));
            }
            Comment::GroupComment(c) if c.range.start.line < c.range.end.line => {
                let first_line = c
                    .comments
                    .first()
                    .map(|l| l.text.clone())
                    .unwrap_or_else(|| "// ...".to_string());
                add_folding_range(c.range, &mut result, Some(format!("{} ...", first_line)));
            }
            _ => {}
        }
    }

    result
}
