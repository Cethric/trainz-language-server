use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "acs_text/grammar.pest"]
pub struct AuranConfigAcsTextParser;
