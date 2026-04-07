use gs_parser::comments::grammar::{CommentsParser, Rule};
use gs_ast::comments::process_comments;
use pest::Parser;

fn main() {
    let src = r#"
//=============================================================================
// Name: GetDebugName
// Desc: Returns a debug name for identifying this object in logs etc. This is
//       not guaranteed to be human-readable, but will be where possible.
//=============================================================================
"#;
    let pairs = CommentsParser::parse(Rule::comment_program, src).unwrap();
    let program = process_comments(pairs.into_iter().next().unwrap(), src);
    println!("{:#?}", program);
}
