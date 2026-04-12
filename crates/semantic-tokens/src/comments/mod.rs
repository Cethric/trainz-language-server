pub mod tests;

use tower_lsp_server::ls_types::{Range, SemanticTokenModifier, SemanticTokenType};
use trainz_ast::comments::{Comment, CommentProgram};

#[tracing::instrument]
pub fn comments_semantic_tokens(
    program: &CommentProgram,
) -> Vec<(Range, SemanticTokenType, Vec<SemanticTokenModifier>)> {
    let mut raw_tokens: Vec<(Range, SemanticTokenType, Vec<SemanticTokenModifier>)> = vec![];

    for comment in &program.comments {
        match comment {
            Comment::LineComment(c) => {
                let is_doc = c.text.starts_with("//!");
                let mut modifiers = vec![];
                if is_doc {
                    modifiers.push(SemanticTokenModifier::DOCUMENTATION);
                }
                raw_tokens.push((c.range, SemanticTokenType::COMMENT, modifiers));
            }
            Comment::BlockComment(c) => {
                raw_tokens.push((c.range, SemanticTokenType::COMMENT, vec![]));
            }
            Comment::GroupComment(c) => {
                let first_text = c.comments.first().map(|l| l.text.as_str()).unwrap_or("");
                let last_text = c.comments.last().map(|l| l.text.as_str()).unwrap_or("");

                let is_doc = first_text.starts_with("//!")
                    || (first_text.starts_with("//==") && last_text.starts_with("//=="));

                let mut modifiers = vec![];
                if is_doc {
                    modifiers.push(SemanticTokenModifier::DOCUMENTATION);
                }
                for line in &c.comments {
                    raw_tokens.push((line.range, SemanticTokenType::COMMENT, modifiers.clone()));
                }
            }
        }
    }

    raw_tokens
}
