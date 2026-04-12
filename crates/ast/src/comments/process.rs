use crate::comments::{BlockComment, Comment, CommentProgram, GroupComment, LineComment};
use crate::{Position, Range};

#[tracing::instrument]
pub fn process_comments<R>(pair: pest::iterators::Pair<R>, src: &str) -> CommentProgram
where
    R: pest::RuleType,
{
    let mut comments = Vec::new();
    let span = pair.as_span();
    let start_pos = span.start_pos().line_col();
    let end_pos = span.end_pos().line_col();
    let range = Range {
        start: Position {
            line: start_pos.0 as u32 - 1,
            character: start_pos.1 as u32 - 1,
        },
        end: Position {
            line: end_pos.0 as u32 - 1,
            character: end_pos.1 as u32 - 1,
        },
    };

    let lines: Vec<&str> = src.lines().collect();
    let mut group_buffer: Vec<LineComment> = Vec::new();

    let flush_group = |buffer: &mut Vec<LineComment>, comments: &mut Vec<Comment>| {
        if !buffer.is_empty() {
            let start = buffer.first().unwrap().range.start;
            let end = buffer.last().unwrap().range.end;
            comments.push(Comment::GroupComment(GroupComment {
                comments: buffer.clone(),
                range: Range { start, end },
            }));
            buffer.clear();
        }
    };

    for inner in pair.into_inner() {
        let rule_name = format!("{:?}", inner.as_rule());
        match rule_name.as_str() {
            "line_comment" => {
                let span = inner.as_span();
                let sp = span.start_pos().line_col();
                let ep = span.end_pos().line_col();

                let line_idx = sp.0 - 1;
                let col_idx = sp.1 - 1;

                let mut is_standalone = false;
                if let Some(line_str) = lines.get(line_idx) {
                    let prefix: String = line_str.chars().take(col_idx).collect();
                    if prefix.trim().is_empty() {
                        is_standalone = true;
                    }
                }

                let line_comment = LineComment {
                    text: inner.as_str().to_string(),
                    range: Range {
                        start: Position {
                            line: sp.0 as u32 - 1,
                            character: sp.1 as u32 - 1,
                        },
                        end: Position {
                            line: ep.0 as u32 - 1,
                            character: ep.1 as u32 - 1,
                        },
                    },
                };

                if is_standalone {
                    if let Some(last) = group_buffer.last() {
                        if last.range.end.line + 1 == line_comment.range.start.line {
                            group_buffer.push(line_comment);
                        } else {
                            flush_group(&mut group_buffer, &mut comments);
                            group_buffer.push(line_comment);
                        }
                    } else {
                        group_buffer.push(line_comment);
                    }
                } else {
                    flush_group(&mut group_buffer, &mut comments);
                    comments.push(Comment::LineComment(line_comment));
                }
            }
            "block_comment" => {
                flush_group(&mut group_buffer, &mut comments);
                let span = inner.as_span();
                let sp = span.start_pos().line_col();
                let ep = span.end_pos().line_col();
                comments.push(Comment::BlockComment(BlockComment {
                    text: inner.as_str().to_string(),
                    range: Range {
                        start: Position {
                            line: sp.0 as u32 - 1,
                            character: sp.1 as u32 - 1,
                        },
                        end: Position {
                            line: ep.0 as u32 - 1,
                            character: ep.1 as u32 - 1,
                        },
                    },
                }));
            }
            _ => {}
        }
    }

    flush_group(&mut group_buffer, &mut comments);

    CommentProgram { comments, range }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pest::Parser;
    use rayon::prelude::*;
    use trainz_parser::comments::grammar::gs::{GsCommentsParser, Rule as GsRule};
    use trainz_parser::comments::grammar::soup::{Rule as SoupRule, SoupCommentsParser};

    #[test]
    fn test_group_comments() {
        let src = r#"
//=============================================================================
// Name: GetDebugName
// Desc: Returns a debug name for identifying this object in logs etc. This is
//       not guaranteed to be human-readable, but will be where possible.
//=============================================================================

public define int DIRECTION_LEFT      = 0;    //!< Left junction direction state.
  public define int DIRECTION_FORWARD   = 1;    //!< Forward junction direction state.
"#;
        let pairs = GsCommentsParser::parse(GsRule::comment_program, src).unwrap();
        let program = process_comments(pairs.into_iter().next().unwrap(), src);
        println!("{:#?}", program);

        let groups: Vec<_> = program
            .comments
            .par_iter()
            .filter_map(|c| match c {
                Comment::GroupComment(g) => Some(g),
                _ => None,
            })
            .collect();
        let lines: Vec<_> = program
            .comments
            .par_iter()
            .filter_map(|c| match c {
                Comment::LineComment(l) => Some(l),
                _ => None,
            })
            .collect();

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].comments.len(), 5);
        assert_eq!(
            groups[0].range.start,
            groups[0].comments.first().unwrap().range.start
        );
        assert_eq!(
            groups[0].range.end,
            groups[0].comments.last().unwrap().range.end
        );
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn test_soup_comments() {
        let src = r#"
; This is a soup comment
; and another one
key value
; trailing
"#;
        let pairs = SoupCommentsParser::parse(SoupRule::comment_program, src).unwrap();
        let program = process_comments(pairs.into_iter().next().unwrap(), src);

        let groups: Vec<_> = program
            .comments
            .par_iter()
            .filter_map(|c| match c {
                Comment::GroupComment(g) => Some(g),
                _ => None,
            })
            .collect();
        let lines: Vec<_> = program
            .comments
            .par_iter()
            .filter_map(|c| match c {
                Comment::LineComment(l) => Some(l),
                _ => None,
            })
            .collect();

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].comments.len(), 2);
        assert_eq!(groups[1].comments.len(), 1);
        assert_eq!(lines.len(), 0);
    }

    #[test]
    fn test_standalone_block_comment() {
        let src = "/* a block comment\non multiple lines */";
        let pairs = GsCommentsParser::parse(GsRule::comment_program, src).unwrap();
        let program = process_comments(pairs.into_iter().next().unwrap(), src);

        assert_eq!(program.comments.len(), 1);
        match &program.comments[0] {
            Comment::BlockComment(b) => {
                assert_eq!(b.range.start.line, 0);
                assert_eq!(b.range.end.line, 1);
            }
            _ => panic!("Expected BlockComment"),
        }
    }
}
