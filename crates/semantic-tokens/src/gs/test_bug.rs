use crate::gs::expr::collect_expr_tokens;
use tower_lsp_server::ls_types::{Range, SemanticTokenModifier, SemanticTokenType};
use trainz_ast::gs::Expr;
use trainz_parser::gs::parse;

#[test]
fn test_str_tokens() {
    let source = "void Test() { Str.Tokens(pid, \"_\"); }";
    let program = parse(source).unwrap();
    println!("{:#?}", program);
}
