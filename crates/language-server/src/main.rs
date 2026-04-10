use clap::Parser;
use log::debug;
use shadow_rs::shadow;
use std::env;
use std::path::PathBuf;
use tokio::main;
use tower_lsp_server::{LspService, Server};
use trainz_common::logging::setup_logger;
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

    /// Path to the directory containing soup validators
    #[arg(short = 'p', long, env = "TRAINZ_LANGUAGE_SERVER_SOUP_VALIDATION_PATH")]
    validation_path: Option<PathBuf>,

    /// Paths to search for Trainz scripts (separated by ;)
    #[arg(
        short,
        long,
        value_delimiter = ';',
        env = "TRAINZ_LANGUAGE_SERVER_SCRIPT_SEARCH_PATHS"
    )]
    search_paths: Vec<PathBuf>,
}

#[main]
async fn main() {
    setup_logger(None);

    let args = Args::parse();
    let validation_path = args.validation_path.or_else(|| {
        env::var("TRAINZ_LANGUAGE_SERVER_SOUP_VALIDATION_PATH")
            .ok()
            .map(PathBuf::from)
    });

    let search_paths = args.search_paths;

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| {
        GameScriptLanguageServer::new(client, validation_path, search_paths.clone(), PKG_VERSION)
    });

    debug!("Starting LSP server");
    Server::new(stdin, stdout, socket).serve(service).await;
}
