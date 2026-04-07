use gs_ast::gs::{Expr, PostfixOp, Stmt, Program};
use gs_parser::gs::parse;

fn main() {
    let source = "void Test() { Str.Tokens(pid, \"_\"); }";
    let program = parse(source).unwrap();
    println!("{:#?}", program);
}
