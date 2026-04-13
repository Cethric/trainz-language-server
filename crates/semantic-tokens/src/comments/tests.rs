use crate::comments::comments_semantic_tokens;
use pest::Parser;
use trainz_ast::comments::process::process_comments;
use trainz_parser::comments::grammar::gs::{GsCommentsParser as CommentsParser, Rule};

#[test]
fn test_comments_semantic_tokens() {
    let src = r#"
// line comment
/// doc comment
/* block comment */
//=================
// group comment
//=================
"#;
    let pairs = CommentsParser::parse(Rule::comment_program, src).unwrap();
    let program = process_comments(pairs.into_iter().next().unwrap(), src);

    let tokens = crate::process_raw_tokens(comments_semantic_tokens(&program), Some(src));

    // We expect line comment, doc comment, block comment, and the group comment lines.
    // Group comment has 3 lines.
    assert!(!tokens.is_empty());
}

#[test]
fn test_multi_line_block_comment_tokens() {
    let src = "/*\n multi-line\n block comment\n*/";
    let pairs = CommentsParser::parse(Rule::comment_program, src).unwrap();
    let program = process_comments(pairs.into_iter().next().unwrap(), src);

    let raw_tokens = comments_semantic_tokens(&program);
    let tokens = crate::process_raw_tokens(raw_tokens, Some(src));

    // Should have 4 tokens, one for each line
    assert_eq!(
        tokens.len(),
        4,
        "Expected 4 tokens for a 4-line block comment, got {:#?}",
        tokens
    );

    // Check lengths (if they are correctly split, they should have non-zero lengths)
    assert_eq!(tokens[0].length, 2); // "/*"
    assert_eq!(tokens[1].length, 11); // " multi-line"
    assert_eq!(tokens[2].length, 14); // " block comment"
    assert_eq!(tokens[3].length, 2); // "*/"
}
