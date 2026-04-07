use crate::gs::semantic_tokens;
use gs_ast::gs::process::process_gs_ast;
use gs_parser::gs::grammar::{GameScriptParser, Rule};
use pest::Parser;

#[test]
fn test_parse_link_prop() {
    let src = "class Test { public void LinkPropertyValue(string pid) { inherited(pid); } };";
    let pairs = GameScriptParser::parse(Rule::program, src).unwrap();
    let program = process_gs_ast(pairs, src);
    let tokens = crate::process_raw_tokens(semantic_tokens(&program));
    println!("{:#?}", program.classes[0].methods[0]);
    println!("{:#?}", tokens);
}
