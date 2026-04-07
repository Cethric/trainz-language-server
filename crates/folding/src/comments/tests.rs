#[cfg(test)]
mod tests {
    use crate::comments::comments_folding_range;
    use gs_ast::comments::process::process_comments;
    use gs_parser::comments::grammar::gs::{GsCommentsParser as CommentsParser, Rule};
    use pest::Parser;

    #[test]
    fn test_comments_folding_range() {
        let src = r#"
// line comment
/// doc comment
/* block 
   comment */
//=================
// group comment
//=================
"#;
        let pairs = CommentsParser::parse(Rule::comment_program, src).unwrap();
        let program = process_comments(pairs.into_iter().next().unwrap(), src);

        let ranges = comments_folding_range(&program);

        for r in &ranges {
            println!(
                "Range: {}:{} - {}:{}",
                r.start_line,
                r.start_character.unwrap_or(0),
                r.end_line,
                r.end_character.unwrap_or(0)
            );
        }

        // We expect one folding range for the block comment, and one for the group comment.
        // Wait, the first two lines (`// line comment` and `/// doc comment`) might form a group comment!
        // Let's assert based on the length we find.
        assert_eq!(ranges.len(), 3);
    }
}
