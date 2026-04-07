use crate::gs::expr::collect_expr_tokens;
use gs_ast::gs::Expr;
use gs_parser::gs::parse;
use tower_lsp_server::ls_types::{Range, SemanticTokenModifier, SemanticTokenType};

#[test]
fn test_str_tokens() {
    let source = "void Test() { Str.Tokens(pid, \"_\"); }";
    let program = parse(source).unwrap();
    println!("{:#?}", program);
}
