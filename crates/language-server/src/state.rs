use dashmap::{DashMap, DashSet};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use tokio::sync::{RwLock, Semaphore};
use tower_lsp_server::Client;
use tower_lsp_server::ls_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, Diagnostic, DocumentSymbol, FoldingRange,
    SemanticToken, TextEdit, Uri, WorkspaceEdit,
};
use tracing::{debug, error, info, trace, warn};
use trainz_acs_text_validators::{RulesRoot, load_validators};
use trainz_ast::acs_text::{AcsText, Kuid};
use trainz_ast::cache::AstCache;
use trainz_ast::gs::Program;
use trainz_tdx::TdxValue;

#[derive(Debug, Clone)]
pub struct Project {
    pub root: PathBuf,
    pub config_txt: PathBuf,
    pub script_files: DashSet<PathBuf>,
    pub chump_files: DashSet<PathBuf>,
    pub assets: DashSet<PathBuf>,
}

#[derive(Debug, Clone)]
pub enum ParsedFileType {
    AcsText(Arc<AcsText>),
    GameScript(Arc<Program>),
    AcsBinary(Arc<Vec<(String, TdxValue)>>),
}

pub struct RecursiveIncludeResolver<'a> {
    pub current_program: &'a Program,
    pub parsed_files: &'a DashMap<String, ParsedFile>,
}

impl<'a> trainz_ast::gs::type_eval::ClassResolver for RecursiveIncludeResolver<'a> {
    fn find_class(&self, name: &str) -> Option<trainz_ast::gs::ClassDef> {
        let mut visited = HashSet::new();
        self.find_recursive(self.current_program, name, &mut visited)
    }
}

impl<'a> trainz_ast::gs::dependency_graph::ProgramResolver for RecursiveIncludeResolver<'a> {
    fn resolve_program(&self, path: &str) -> Option<Arc<trainz_ast::gs::Program>> {
        if let Some(file) = self.parsed_files.get(path)
            && let ParsedFileType::GameScript(program) = &file.value().parsed
        {
            return Some(program.clone());
        }
        None
    }
}

impl<'a> RecursiveIncludeResolver<'a> {
    fn find_recursive(
        &self,
        program: &trainz_ast::gs::Program,
        name: &str,
        visited: &mut HashSet<String>,
    ) -> Option<trainz_ast::gs::ClassDef> {
        if let Some(cls) = program.classes.get(name) {
            return Some(cls.clone());
        }

        for include in &program.includes {
            if let Some(path) = &include.path {
                let path_str = path.to_string_lossy().to_string();
                if !visited.insert(path_str.clone()) {
                    continue;
                }

                let included_program = self.parsed_files.get(&path_str).and_then(|file| {
                    if let ParsedFileType::GameScript(included_program) = &file.value().parsed {
                        Some(included_program.clone())
                    } else {
                        None
                    }
                });

                if let Some(included_program) = included_program
                    && let Some(cls) = self.find_recursive(&included_program, name, visited)
                {
                    return Some(cls);
                }
            }
        }
        None
    }
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
pub struct GameScriptState {
    pub search_paths: Vec<PathBuf>,
}

#[derive(Debug)]
pub struct ACSState {
    pub validation_path: Option<PathBuf>,
    pub extensions_overrides_path: Option<PathBuf>,
    pub graph: RwLock<Option<RulesRoot>>,
}

impl ACSState {
    pub async fn load_graph(&self) {
        if let Some(validation_path) = &self.validation_path {
            debug!(
                "load_graph: loading from path={}",
                validation_path.display()
            );
            self.load_graph_from_path(validation_path).await;
        } else {
            warn!("No validation path specified");
        }
    }

    pub async fn load_graph_from_path(&self, validation_path: &Path) {
        debug!(
            "load_graph_from_path: validation_path={}",
            validation_path.display()
        );
        let extension_overrides_path = self.extensions_overrides_path.as_deref();
        let graph = load_validators(validation_path, extension_overrides_path).await;
        if let Ok(graph) = graph {
            let mut guard = self.graph.write().await;
            info!(
                "Loaded {} top level nodes from {}",
                graph.get_top_level_nodes().len(),
                validation_path.display()
            );
            guard.replace(graph);
        } else if let Err(err) = graph {
            error!(
                "Failed to load validators from {}: {}",
                validation_path.display(),
                err
            )
        }
    }
}

#[derive(Debug)]
pub struct TrainzLanguageServer {
    pub client: Client,
    pub asset_cache_path: Option<PathBuf>,
    pub tdx_cache_path: Option<PathBuf>,
    pub parsed_files: DashMap<String, ParsedFile>,
    pub counts: DashMap<String, AtomicUsize>,
    pub currently_processing: DashSet<String>,
    pub ast_cache: AstCache,
    pub workspace_folders: DashSet<PathBuf>,
    pub projects: DashMap<PathBuf, Arc<Project>>,
    pub version: String,
    pub processing_semaphore: Semaphore,
    pub gs_state: GameScriptState,
    pub acs_state: ACSState,
}

impl TrainzLanguageServer {
    pub fn new(
        client: Client,
        validation_path: Option<PathBuf>,
        search_paths: Vec<PathBuf>,
        version: &str,
        asset_cache_path: Option<PathBuf>,
        tdx_cache_path: Option<PathBuf>,
        extensions_overrides_path: Option<PathBuf>,
    ) -> Self {
        info!("Create TrainzLanguageServer {}", version);

        trace!("Search paths {:?}", search_paths);
        trace!("Validation path {:?}", validation_path);
        trace!("Asset cache path {:?}", asset_cache_path);
        trace!("TDX cache path {:?}", tdx_cache_path);
        trace!("Extensions overrides path {:?}", extensions_overrides_path);

        Self {
            client,
            asset_cache_path,
            tdx_cache_path,
            parsed_files: DashMap::new(),
            counts: DashMap::new(),
            currently_processing: DashSet::new(),
            ast_cache: AstCache::new(),
            workspace_folders: DashSet::new(),
            projects: DashMap::new(),
            version: String::from(version),
            processing_semaphore: Semaphore::new(num_cpus::get()),
            gs_state: GameScriptState { search_paths },
            acs_state: ACSState {
                validation_path,
                extensions_overrides_path,
                graph: RwLock::new(None),
            },
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

    pub fn create_kuid_version_action(
        &self,
        title: String,
        acs_text: &AcsText,
        target: &Kuid,
        new_kuid: Kuid,
        uri: &Uri,
    ) -> CodeActionOrCommand {
        let mut changes = std::collections::HashMap::new();
        let mut edits = vec![];

        let all_kuids = acs_text.find_all_kuids();
        for k in all_kuids {
            if k.user_id == target.user_id && k.content_id == target.content_id {
                edits.push(TextEdit {
                    range: k.range,
                    new_text: new_kuid.to_string(),
                });
            }
        }

        changes.insert(uri.clone(), edits);

        CodeActionOrCommand::CodeAction(CodeAction {
            title,
            kind: Some(CodeActionKind::REFACTOR_REWRITE),
            edit: Some(WorkspaceEdit {
                changes: Some(changes),
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}
