use clap::Parser;
use rayon::ThreadPoolBuilder;
use shadow_rs::shadow;
use std::path::PathBuf;
use tokio::main;
use tower_lsp_server::{LspService, Server};
use tracing::{debug, info};
use trainz_common::logging::{BoxMakeWriter, setup_logger};
use trainz_language_server::state::TrainzLanguageServer;

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

    /// Log level
    #[arg(long, env = "TRAINZ_LANGUAGE_SERVER_LOG_LEVEL")]
    log_level: Option<String>,

    /// Path to the asset cache sqlite file
    #[arg(long, env = "TRAINZ_LANGUAGE_SERVER_ASSET_CACHE")]
    asset_cache: Option<PathBuf>,

    /// Path to the TDX asset cache directory
    #[arg(long, env = "TRAINZ_LANGUAGE_SERVER_TDX_CACHE")]
    tdx_cache: Option<PathBuf>,

    /// Path to a folder for defining extensions overrides
    #[arg(long, env = "TRAINZ_LANGUAGE_SERVER_EXTENSIONS_OVERRIDES")]
    extensions_overrides: Option<PathBuf>,
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
    let log_level = args
        .log_level
        .clone()
        .or_else(|| Some(args.verbosity.to_string()));
    setup_logger(log_level, Some(writer));

    info!(
        "Launching Trainz Language Server v{} - {:?} - validation_path: {:?}",
        PKG_VERSION, args, args.validation_path
    );

    let validation_path = args.validation_path;
    let search_paths = args.search_paths;
    let asset_cache = args.asset_cache;
    let tdx_cache = args.tdx_cache;
    let extensions_overrides = args.extensions_overrides;

    let (service, socket) = LspService::build(|client| {
        TrainzLanguageServer::new(
            client,
            validation_path,
            search_paths.clone(),
            PKG_VERSION,
            asset_cache,
            tdx_cache,
            extensions_overrides,
        )
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
