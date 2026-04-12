use clap::Parser;
use clio::*;
use std::io::{Read, Write};
use tokio::main;
use tracing::{debug, error};
use trainz_ast::gs::process::process_trainz_ast;
use trainz_common::logging::{BoxMakeWriter, setup_logger};
use trainz_formatter::format_program;
use trainz_parser::gs::parse;

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

    /// Log file path
    #[arg(long)]
    log_file: Option<std::path::PathBuf>,
}

#[main]
async fn main() {
    let mut args = Args::parse();
    let writer = if let Some(path) = &args.log_file {
        let file = std::fs::File::create(path).expect("failed to create log file");
        BoxMakeWriter::new(move || {
            file.try_clone()
                .expect("failed to clone log file descriptor")
        })
    } else {
        BoxMakeWriter::new(std::io::stderr)
    };
    setup_logger(Some(args.verbosity.into()), Some(writer));

    let mut source = "".to_string();
    let read = args.input.read_to_string(&mut source).unwrap_or(0);
    assert_eq!(source.len(), read);
    let pairs = parse(source.as_str());
    if let Ok(pairs) = pairs {
        let ast = process_trainz_ast(pairs, source.as_str());
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
