use clap::Parser;
use clio::*;
use gs_ast::gs::process::process_gs_ast;
use gs_common::setup_logger;
use gs_formatter::format_program;
use gs_parser::gs::parse;
use log::{debug, error};
use std::io::{Read, Write};
use tokio::main;

use crate::build::CLAP_LONG_VERSION;
use shadow_rs::shadow;

shadow!(build);

#[derive(Parser, Debug)]
#[command(long_about = None)]
#[command(version = CLAP_LONG_VERSION, about = "GameScript formatter")]
struct Args {
    #[command(flatten)]
    verbosity: clap_verbosity_flag::Verbosity,
    /// Input file, use '-' for stdin
    #[clap(value_parser, default_value = "-")]
    input: Input,

    /// Output file '-' for stdout
    #[clap(long, short, value_parser, default_value = "-")]
    output: Output,
}

#[main]
async fn main() {
    let mut args = Args::parse();
    setup_logger(Some(args.verbosity.into()));

    let mut source = "".to_string();
    let read = args.input.read_to_string(&mut source).unwrap_or(0);
    assert_eq!(source.len(), read);
    let pairs = parse(source.as_str());
    if let Ok(pairs) = pairs {
        let ast = process_gs_ast(pairs, source.as_str());
        // debug!("Processed AST: {:#?}", ast);

        let formatted = format_program(&ast);
        debug!("Formatted Code: {:?}", args.output.to_string());
        args.output
            .write_all(formatted.as_bytes())
            .expect("Failed to write formatted program");
    } else {
        error!("Failed to parse program: {:?}", pairs.unwrap_err());
    }
}
