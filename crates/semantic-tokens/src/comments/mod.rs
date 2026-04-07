pub mod tests;

use gs_ast::comments::{Comment, CommentProgram};
use tower_lsp_server::ls_types::{Range, SemanticTokenModifier, SemanticTokenType};

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
                if c.range.start.line == c.range.end.line {
                    raw_tokens.push((c.range, SemanticTokenType::COMMENT, vec![]));
                } else {
                    let lines: Vec<&str> = c.text.lines().collect();
                    for (i, line) in lines.iter().enumerate() {
                        let line_num = c.range.start.line + i as u32;
                        let start_char = if i == 0 { c.range.start.character } else { 0 };
                        let end_char = if i == lines.len() - 1 {
                            c.range.end.character
                        } else {
                            line.len() as u32
                        };

                        raw_tokens.push((
                            Range {
                                start: tower_lsp_server::ls_types::Position {
                                    line: line_num,
                                    character: start_char,
                                },
                                end: tower_lsp_server::ls_types::Position {
                                    line: line_num,
                                    character: end_char,
                                },
                            },
                            SemanticTokenType::COMMENT,
                            vec![],
                        ));
                    }
                }
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
