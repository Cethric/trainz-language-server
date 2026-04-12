use dashmap::{DashMap, DashSet};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use tower_lsp_server::Client;
use tower_lsp_server::ls_types::{Diagnostic, DocumentSymbol, FoldingRange, SemanticToken};
use tracing::{info, trace};
use trainz_ast::cache::AstCache;
use trainz_ast::gs::Program;
use trainz_ast::soup::Soup;
use trainz_soup_validators::Validators;

#[derive(Debug, Clone)]
pub enum ParsedFileType {
    Soup(Arc<Soup>),
    GameScript(Arc<Program>),
}

#[derive(Debug, Clone)]
pub struct ParsedFile {
    pub count: usize,
    pub parsed: ParsedFileType,
    pub comments: Arc<trainz_ast::comments::CommentProgram>,
    pub semantic_tokens: Arc<OnceLock<Vec<SemanticToken>>>,
    pub document_symbols: Arc<OnceLock<Vec<DocumentSymbol>>>,
    pub diagnostics: Arc<OnceLock<Vec<Diagnostic>>>,
    pub folding_ranges: Arc<OnceLock<Vec<FoldingRange>>>,
}

#[derive(Debug)]
pub struct GameScriptLanguageServer {
    pub client: Client,
    pub search_paths: Vec<PathBuf>,
    pub validation_path: Option<PathBuf>,
    pub validators: Arc<OnceLock<Validators>>,
    pub parsed_files: DashMap<String, ParsedFile>,
    pub currently_processing: DashSet<String>,
    pub ast_cache: AstCache,
    pub workspace_folders: DashSet<PathBuf>,
    pub version: String,
}

impl GameScriptLanguageServer {
    pub fn new(
        client: Client,
        validation_path: Option<PathBuf>,
        search_paths: Vec<PathBuf>,
        version: &str,
    ) -> Self {
        info!("Create GameScriptLanguageServer {}", version);

        trace!("Search paths {:?}", search_paths);
        trace!("Validation path {:?}", validation_path);

        Self {
            client,
            search_paths,
            validation_path,
            validators: Arc::new(OnceLock::new()),
            parsed_files: DashMap::new(),
            currently_processing: DashSet::new(),
            ast_cache: AstCache::new(),
            workspace_folders: DashSet::new(),
            version: String::from(version),
        }
    }

    pub fn workspace_folders(&self) -> Vec<PathBuf> {
        self.workspace_folders
            .iter()
            .map(|folder| folder.clone())
            .collect::<Vec<PathBuf>>()
    }
}
