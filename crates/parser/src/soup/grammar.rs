use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "soup/grammar.pest"]
pub struct AuranConfigSoupParser;
