use dashmap::{DashMap, DashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use tokio::sync::Semaphore;
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

#[derive(Debug)]
pub struct ParsedFile {
    pub parsed: ParsedFileType,
    pub comments: Arc<trainz_ast::comments::CommentProgram>,
    pub semantic_tokens: Arc<OnceLock<Vec<SemanticToken>>>,
    pub document_symbols: Arc<OnceLock<Vec<DocumentSymbol>>>,
    pub diagnostics: Arc<OnceLock<Vec<Diagnostic>>>,
    pub folding_ranges: Arc<OnceLock<Vec<FoldingRange>>>,
}

impl Clone for ParsedFile {
    fn clone(&self) -> Self {
        Self {
            parsed: self.parsed.clone(),
            comments: self.comments.clone(),
            semantic_tokens: self.semantic_tokens.clone(),
            document_symbols: self.document_symbols.clone(),
            diagnostics: self.diagnostics.clone(),
            folding_ranges: self.folding_ranges.clone(),
        }
    }
}

#[derive(Debug)]
pub struct GameScriptLanguageServer {
    pub client: Client,
    pub search_paths: Vec<PathBuf>,
    pub validation_path: Option<PathBuf>,
    pub validators: Arc<OnceLock<Validators>>,
    pub parsed_files: DashMap<String, ParsedFile>,
    pub counts: DashMap<String, AtomicUsize>,
    pub currently_processing: DashSet<String>,
    pub ast_cache: AstCache,
    pub workspace_folders: DashSet<PathBuf>,
    pub version: String,
    pub processing_semaphore: Semaphore,
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
            counts: DashMap::new(),
            currently_processing: DashSet::new(),
            ast_cache: AstCache::new(),
            workspace_folders: DashSet::new(),
            version: String::from(version),
            processing_semaphore: Semaphore::new(num_cpus::get()),
        }
    }

    pub fn increment_count(&self, path: &str) {
        self.counts
            .entry(path.to_string())
            .or_insert_with(|| AtomicUsize::new(0))
            .fetch_add(1, Ordering::SeqCst);
    }

    pub fn decrement_count(&self, path: &str) {
        let mut remove = false;
        if let Some(count) = self.counts.get(path)
            && count.fetch_sub(1, Ordering::SeqCst) <= 1
        {
            remove = true;
        }

        if remove {
            let file = self.parsed_files.remove(path);
            self.counts.remove(path);

            if let Some((_, file)) = file
                && let ParsedFileType::GameScript(program) = file.parsed
            {
                for include in &program.includes {
                    if let Some(include_path) = &include.path {
                        self.decrement_count(&include_path.to_string_lossy());
                    }
                }
            }
        }
    }

    pub fn workspace_folders(&self) -> Vec<PathBuf> {
        self.workspace_folders
            .iter()
            .map(|folder| folder.clone())
            .collect::<Vec<PathBuf>>()
    }
}
