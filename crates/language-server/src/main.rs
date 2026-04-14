use clap::Parser;
use rayon::ThreadPoolBuilder;
use shadow_rs::shadow;
use std::env;
use std::path::PathBuf;
use tokio::main;
use tower_lsp_server::{LspService, Server};
use tracing::debug;
use trainz_common::logging::{BoxMakeWriter, setup_logger};
use trainz_language_server::state::GameScriptLanguageServer;

pub mod process;
pub mod state;

use crate::build::{CLAP_LONG_VERSION, PKG_VERSION};

shadow!(build);

#[derive(Parser, Debug)]
#[command(long_about = None)]
#[command(version = CLAP_LONG_VERSION, about = "GameScript Language Server")]
struct Args {
    #[command(flatten)]
    verbosity: clap_verbosity_flag::Verbosity,

    /// Path to the directory containing acs_text validators
    #[arg(
        short = 'p',
        long,
        env = "TRAINZ_LANGUAGE_SERVER_ACS_TEXT_VALIDATION_PATH"
    )]
    validation_path: Option<PathBuf>,

    /// Paths to search for Trainz scripts (separated by ;)
    #[arg(
        short,
        long,
        value_delimiter = ';',
        env = "TRAINZ_LANGUAGE_SERVER_SCRIPT_SEARCH_PATHS"
    )]
    search_paths: Vec<PathBuf>,

    /// Log file path
    #[arg(long, env = "TRAINZ_LANGUAGE_SERVER_LOG_FILE")]
    log_file: Option<PathBuf>,
}

#[main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let args = Args::parse();
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

    let validation_path = args.validation_path.or_else(|| {
        env::var("TRAINZ_LANGUAGE_SERVER_ACS_TEXT_VALIDATION_PATH")
            .ok()
            .map(PathBuf::from)
    });

    let search_paths = args.search_paths;

    let (service, socket) = LspService::build(|client| {
        GameScriptLanguageServer::new(client, validation_path, search_paths.clone(), PKG_VERSION)
    })
    .finish();

    debug!("Starting LSP server");
    let threads = num_cpus::get();
    ThreadPoolBuilder::new()
        .num_threads(threads)
        .build_global()
        .unwrap();
    Server::new(stdin, stdout, socket)
        .concurrency_level(threads)
        .serve(service)
        .await;
}
