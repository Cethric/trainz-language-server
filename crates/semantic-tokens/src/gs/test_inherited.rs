use crate::gs::semantic_tokens;
use pest::Parser;
use trainz_ast::gs::process::process_trainz_ast;
use trainz_parser::gs::grammar::{GameScriptParser, Rule};

#[test]
fn test_parse_link_prop() {
    let src = "class Test { public void LinkPropertyValue(string pid) { inherited(pid); } };";
    let pairs = GameScriptParser::parse(Rule::program, src).unwrap();
    let program = process_trainz_ast(pairs, src);
    let tokens = crate::process_raw_tokens(semantic_tokens(&program), Some(src));
    println!("{:#?}", program.classes.values().next().unwrap().methods.values().next().unwrap()[0]);
    println!("{:#?}", tokens);
}
