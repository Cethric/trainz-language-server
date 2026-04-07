use gs_parser::comments::grammar::{CommentsParser, Rule};
use pest::Parser;

fn main() {
    let src = r#"
//=============================================================================
// Name: GetDebugName
// Desc: Returns a debug name for identifying this object in logs etc. This is
//       not guaranteed to be human-readable, but will be where possible.
//=============================================================================

public define int DIRECTION_LEFT      = 0;    //!< Left junction direction state.
  public define int DIRECTION_FORWARD   = 1;    //!< Forward junction direction state.
"#;
    let pairs = CommentsParser::parse(Rule::comment_program, src).unwrap();
    for pair in pairs {
        println!("{:?}", pair.as_rule());
    }
}
