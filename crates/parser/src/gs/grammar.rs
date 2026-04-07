use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "gs/grammar.pest"]
pub struct GameScriptParser;
