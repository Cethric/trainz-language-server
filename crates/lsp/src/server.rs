use async_recursion::async_recursion;
use dashmap::{DashMap, DashSet};
use gs_ast::cache::AstCache;
use gs_ast::gs::process::process_gs_ast;
use gs_ast::gs::{Include, Program};
use gs_ast::soup::Soup;
use gs_ast::soup::process::process_soup_ast;
use gs_completions::gs::gs_completions;
use gs_completions::soup::soup_completions;
use gs_definition::gs::definitions::gs_goto_definition;
use gs_definition::gs::references::gs_find_references;
use gs_definition::soup::definitions::soup_goto_definition;
use gs_diagnostics::gs::gs_diagnostics;
use gs_diagnostics::soup::soup_diagnostics;
use gs_folding::gs::gs_folding_range;
use gs_folding::soup::soup_folding_range;
use gs_hover::soup::soup_hover;
use gs_parser::gs::parse;
use gs_parser::soup::parse_soup;
use gs_semantic_tokens::gs::semantic_tokens;
use gs_semantic_tokens::soup::soup_semantic_tokens;
use gs_symboliser::gs::gs_symboliser;
use gs_symboliser::soup::soup_symboliser;
use log::{error, trace};
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use tower_lsp_server::jsonrpc::Error;
use tower_lsp_server::ls_types::{
    CompletionOptions, CompletionOptionsCompletionItem, CompletionParams, CompletionResponse,
    DefinitionOptions, Diagnostic, DiagnosticOptions, DiagnosticServerCapabilities,
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DocumentDiagnosticParams, DocumentDiagnosticReport, DocumentDiagnosticReportResult,
    DocumentLink, DocumentLinkOptions, DocumentLinkParams, DocumentSymbol, DocumentSymbolOptions,
    DocumentSymbolParams, DocumentSymbolResponse, FoldingRange, FoldingRangeParams,
    FoldingRangeProviderCapability, FullDocumentDiagnosticReport, GotoDefinitionParams,
    GotoDefinitionResponse, Hover, HoverOptions, HoverParams, HoverProviderCapability,
    InitializeParams, InitializeResult, InitializedParams, Location, MessageType, OneOf,
    PositionEncodingKind, ProgressToken, ReferenceOptions, ReferenceParams,
    RelatedFullDocumentDiagnosticReport, RelatedUnchangedDocumentDiagnosticReport, SaveOptions,
    SemanticToken, SemanticTokens, SemanticTokensFullOptions, SemanticTokensLegend,
    SemanticTokensOptions, SemanticTokensParams, SemanticTokensResult,
    SemanticTokensServerCapabilities, ServerCapabilities, ServerInfo, SignatureHelp,
    SignatureHelpOptions, SignatureHelpParams, StaticTextDocumentColorProviderOptions,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions,
    TextDocumentSyncSaveOptions, UnchangedDocumentDiagnosticReport, Uri, WorkDoneProgressOptions,
    WorkspaceDiagnosticParams, WorkspaceDiagnosticReport, WorkspaceDiagnosticReportResult,
};
use tower_lsp_server::{Bounded, Client, LanguageServer, NotCancellable, OngoingProgress};

fn find_include_path(
    include: &str,
    base_path: &Path,
    workspace_folders: &Vec<PathBuf>,
    search_paths: &Vec<PathBuf>,
) -> Option<PathBuf> {
    let mut paths: Vec<PathBuf> = search_paths
        .par_iter()
        .map(|search| search.join(include))
        .collect();

    let workspace_paths: Vec<PathBuf> = workspace_folders
        .par_iter()
        .map(|folder| folder.join(include))
        .collect();

    paths.extend(workspace_paths);
    paths.push(base_path.join(include));

    paths
        .into_par_iter()
        .filter(|path| path.exists())
        .map(|path| path.to_path_buf())
        .collect::<Vec<_>>()
        .into_iter()
        .next()
}

#[derive(Debug, Clone)]
pub(crate) enum ParsedFileType {
    Soup(Arc<Soup>),
    GameScript(Arc<Program>),
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedFile {
    pub count: usize,
    pub parsed: ParsedFileType,
    pub comments: Arc<gs_ast::comments::CommentProgram>,
    pub semantic_tokens: Arc<OnceLock<Vec<SemanticToken>>>,
    pub document_symbols: Arc<OnceLock<Vec<DocumentSymbol>>>,
    pub diagnostics: Arc<OnceLock<Vec<Diagnostic>>>,
    pub folding_ranges: Arc<OnceLock<Vec<FoldingRange>>>,
}

#[derive(Debug)]
pub struct GameScriptLanguageServer {
    pub client: Client,
    search_paths: Vec<PathBuf>,
    validation_path: Option<PathBuf>,
    validators: Arc<OnceLock<gs_diagnostics::soup::Validators>>,
    parsed_files: DashMap<String, ParsedFile>,
    currently_processing: DashSet<String>,
    ast_cache: AstCache,
}

impl GameScriptLanguageServer {
    async fn workspace_folders(&self) -> Vec<PathBuf> {
        if let Some(workspace_folders) = self.client.workspace_folders().await.ok() {
            if let Some(workspace_folders) = workspace_folders {
                workspace_folders
                    .par_iter()
                    .map(|folder| folder.uri.to_file_path())
                    .filter_map(|path| path)
                    .map(|path| path.to_path_buf())
                    .collect::<Vec<PathBuf>>()
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    }
}

impl GameScriptLanguageServer {
    #[async_recursion]
    async fn process_gs_include(
        &self,
        include: Include,
        workspace_folders: &Vec<PathBuf>,
        progress: &OngoingProgress<Bounded, NotCancellable>,
    ) {
        if let Some(path) = include.path {
            let path_str = path.to_string_lossy().to_string();
            if self.parsed_files.contains_key(&path_str) {
                trace!("Include already processed {:?}", path);
                return;
            }

            // Try load from cache
            if let Some(program) = self.ast_cache.load(&path) {
                trace!("Found include in cache {:?}", path);
                // We still need content for comments and other processing,
                // but the task says "try and load from cache and then try and parse the file"
                // which implies we might avoid parsing.
                // However, the current ParsedFile structure needs comments too.
                // Let's see if we can read the file and if it's unchanged, use the cached AST.
                // For now, I'll follow the instruction to try load and then try parse if failed.

                if let Ok(document) = fs::read(path.clone()) {
                    let source = String::from_utf8_lossy(&document);

                    let comments =
                        if let Ok(pairs) = gs_parser::comments::parse_gs_comments(&source) {
                            Arc::new(gs_ast::comments::process::process_comments(
                                pairs.into_iter().next().unwrap(),
                                &source,
                            ))
                        } else {
                            Arc::new(gs_ast::comments::CommentProgram {
                                comments: vec![],
                                range: Default::default(),
                            })
                        };

                    self.parsed_files.insert(
                        path_str,
                        ParsedFile {
                            count: 0,
                            parsed: ParsedFileType::GameScript(Arc::new(program)),
                            comments,
                            semantic_tokens: Arc::new(OnceLock::new()),
                            document_symbols: Arc::new(OnceLock::new()),
                            diagnostics: Arc::new(OnceLock::new()),
                            folding_ranges: Arc::new(OnceLock::new()),
                        },
                    );
                    return;
                }
            }

            trace!("Processing include file {:?}", path);

            let document = fs::read(path.clone());
            if let Ok(document) = document {
                let source = String::from_utf8_lossy(&document);

                self.process_gs_file(path.as_path(), &source, workspace_folders, false, progress)
                    .await;
            }
        }
    }
}

struct ProcessingGuard<'a> {
    set: &'a DashSet<String>,
    path: String,
}

impl<'a> ProcessingGuard<'a> {
    fn new(set: &'a DashSet<String>, path: String) -> Option<Self> {
        if set.insert(path.clone()) {
            Some(Self { set, path })
        } else {
            None
        }
    }
}

impl Drop for ProcessingGuard<'_> {
    fn drop(&mut self) {
        self.set.remove(&self.path);
    }
}

impl GameScriptLanguageServer {
    pub async fn process_gs_file(
        &self,
        path: &Path,
        content: &str,
        workspace_folders: &Vec<PathBuf>,
        changed: bool,
        progress: &OngoingProgress<Bounded, NotCancellable>,
    ) {
        let _guard = if let Some(path_str) = path.to_str() {
            match ProcessingGuard::new(&self.currently_processing, path_str.to_string()) {
                Some(g) => Some(g),
                None => {
                    trace!("File is already being processed {:?}", path);
                    return;
                }
            }
        } else {
            None
        };

        self.process_gs_file_inner(path, content, workspace_folders, changed, progress)
            .await;
    }

    #[async_recursion]
    async fn process_gs_file_inner(
        &self,
        path: &Path,
        content: &str,
        workspace_folders: &Vec<PathBuf>,
        changed: bool,
        progress: &OngoingProgress<Bounded, NotCancellable>,
    ) {
        let mut includes: Vec<Include> = vec![];

        if let Some(path_str) = path.to_str() {
            let base_path = path.parent().unwrap();

            if self.parsed_files.contains_key(path_str) && !changed {
                trace!("File already processed {:?}", path);
                return;
            }
            trace!("Processing file {:?}", path);

            let pairs = parse(content);
            if let Ok(pairs) = pairs {
                trace!("File parsed {:?}", path);

                let mut parsed = process_gs_ast(pairs, content);

                // Save to cache
                let _ = self.ast_cache.save(path, &parsed);

                // Resolve include paths
                for include in &mut parsed.includes {
                    include.path = find_include_path(
                        &include.name,
                        base_path,
                        workspace_folders,
                        &self.search_paths,
                    );
                }

                let mut counter = 1;
                let mut early_exit = false;
                if let Some(orig) = self.parsed_files.get(path_str) {
                    counter = orig.value().count;
                    early_exit = true;
                }

                let parsed_arc = Arc::new(parsed);

                let comments_arc = match gs_parser::comments::parse_gs_comments(content) {
                    Ok(mut pairs) => {
                        if let Some(pair) = pairs.next() {
                            Arc::new(gs_ast::comments::process::process_comments(pair, content))
                        } else {
                            Arc::new(gs_ast::comments::CommentProgram {
                                comments: vec![],
                                range: Default::default(),
                            })
                        }
                    }
                    Err(_) => Arc::new(gs_ast::comments::CommentProgram {
                        comments: vec![],
                        range: Default::default(),
                    }),
                };

                self.parsed_files.insert(
                    path_str.to_string(),
                    ParsedFile {
                        count: counter,
                        parsed: ParsedFileType::GameScript(parsed_arc.clone()),
                        comments: comments_arc,
                        semantic_tokens: Arc::new(OnceLock::new()),
                        document_symbols: Arc::new(OnceLock::new()),
                        diagnostics: Arc::new(OnceLock::new()),
                        folding_ranges: Arc::new(OnceLock::new()),
                    },
                );

                if early_exit {
                    trace!("Early exit");
                    return;
                }

                includes.extend(parsed_arc.includes.clone());
            } else if let Err(e) = pairs {
                error!("Failed to parse file: {:?}", e)
            }
        } else {
            unreachable!("path is not a string");
        }

        let total = includes.len();
        for (i, include) in includes.into_iter().enumerate() {
            let remaining = total - i;
            let percentage = if total > 0 {
                (i as u32 * 90) / total as u32
            } else {
                90
            };
            progress
                .report_with_message(
                    &format!("Updating {} ({} remaining)", include.name, remaining),
                    10 + percentage,
                )
                .await;
            self.process_gs_include(include, workspace_folders, progress)
                .await;
        }
    }

    pub async fn process_soup_file(
        &self,
        path: &Path,
        content: &str,
        workspace_folders: &Vec<PathBuf>,
        changed: bool,
        progress: &OngoingProgress<Bounded, NotCancellable>,
    ) {
        let _guard = if let Some(path_str) = path.to_str() {
            match ProcessingGuard::new(&self.currently_processing, path_str.to_string()) {
                Some(g) => Some(g),
                None => {
                    trace!("File is already being processed {:?}", path);
                    return;
                }
            }
        } else {
            None
        };

        self.process_soup_file_inner(path, content, workspace_folders, changed, progress)
            .await;
    }

    async fn process_soup_file_inner(
        &self,
        path: &Path,
        content: &str,
        workspace_folders: &Vec<PathBuf>,
        changed: bool,
        progress: &OngoingProgress<Bounded, NotCancellable>,
    ) {
        if let Some(path_str) = path.to_str() {
            let base_path = path.parent().unwrap();

            if self.parsed_files.contains_key(path_str) && !changed {
                trace!("File already processed {:?}", path);
                return;
            }
            trace!("Processing file {:?}", path);

            let pairs = parse_soup(content);
            if let Ok(pairs) = pairs {
                trace!("File parsed {:?}", path);

                let parsed = process_soup_ast(
                    pairs,
                    content,
                    base_path,
                    workspace_folders,
                    &self.search_paths,
                );

                let mut counter = 1;
                if let Some(orig) = self.parsed_files.get(path_str) {
                    counter = orig.value().count;
                }

                let parsed_arc = Arc::new(parsed);

                let comments_arc = match gs_parser::comments::parse_soup_comments(content) {
                    Ok(mut pairs) => {
                        if let Some(pair) = pairs.next() {
                            Arc::new(gs_ast::comments::process::process_comments(pair, content))
                        } else {
                            Arc::new(gs_ast::comments::CommentProgram {
                                comments: vec![],
                                range: Default::default(),
                            })
                        }
                    }
                    Err(_) => Arc::new(gs_ast::comments::CommentProgram {
                        comments: vec![],
                        range: Default::default(),
                    }),
                };

                self.parsed_files.insert(
                    path_str.to_string(),
                    ParsedFile {
                        count: counter,
                        parsed: ParsedFileType::Soup(parsed_arc),
                        comments: comments_arc,
                        semantic_tokens: Arc::new(OnceLock::new()),
                        document_symbols: Arc::new(OnceLock::new()),
                        diagnostics: Arc::new(OnceLock::new()),
                        folding_ranges: Arc::new(OnceLock::new()),
                    },
                );
            } else if let Err(e) = pairs {
                error!("Failed to parse file: {:?}", e)
            }
        } else {
            unreachable!("path is not a string");
        }

        progress.report_with_message("Processed file", 100).await;
    }
}

const GAME_SCRIPT_LANGUAGE_ID: &str = "game-script";
const SOUP_LANGUAGE_ID: &str = "soup";

impl LanguageServer for GameScriptLanguageServer {
    async fn initialize(
        &self,
        params: InitializeParams,
    ) -> tower_lsp_server::jsonrpc::Result<InitializeResult> {
        trace!("Initializing {:?}", params);

        self.client
            .log_message(
                MessageType::INFO,
                "Initializing GameScript Language Server".to_string(),
            )
            .await;

        let capabilities = ServerCapabilities {
            position_encoding: Some(PositionEncodingKind::UTF16),
            text_document_sync: Some(TextDocumentSyncCapability::Options(
                TextDocumentSyncOptions {
                    open_close: Some(true),
                    change: Some(TextDocumentSyncKind::FULL),
                    will_save: None,
                    will_save_wait_until: None,
                    save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                        include_text: Some(true),
                    })),
                },
            )),
            diagnostic_provider: Some(DiagnosticServerCapabilities::Options(DiagnosticOptions {
                identifier: Some("game-script".to_string()),
                inter_file_dependencies: true,
                workspace_diagnostics: true,
                work_done_progress_options: WorkDoneProgressOptions {
                    work_done_progress: Some(true),
                },
            })),
            semantic_tokens_provider: Some(
                SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
                    work_done_progress_options: WorkDoneProgressOptions {
                        work_done_progress: Some(true),
                    },
                    full: Some(SemanticTokensFullOptions::Bool(true)),
                    range: None,
                    legend: {
                        let (token_types, token_modifiers) =
                            gs_semantic_tokens::legend::get_legend();
                        SemanticTokensLegend {
                            token_modifiers,
                            token_types,
                        }
                    },
                }),
            ),
            document_symbol_provider: Some(OneOf::Right(DocumentSymbolOptions {
                label: Some(String::from("GameScript")),
                work_done_progress_options: WorkDoneProgressOptions {
                    work_done_progress: Some(true),
                },
            })),
            document_link_provider: Some(DocumentLinkOptions {
                resolve_provider: Some(true),
                work_done_progress_options: WorkDoneProgressOptions {
                    work_done_progress: Some(true),
                },
            }),
            completion_provider: Some(CompletionOptions {
                resolve_provider: Some(false),
                trigger_characters: None,
                all_commit_characters: None,
                work_done_progress_options: WorkDoneProgressOptions {
                    work_done_progress: Some(true),
                },
                completion_item: Some(CompletionOptionsCompletionItem {
                    label_details_support: Some(true),
                }),
            }),
            signature_help_provider: Some(SignatureHelpOptions {
                trigger_characters: None,
                retrigger_characters: None,
                work_done_progress_options: WorkDoneProgressOptions {
                    work_done_progress: Some(true),
                },
            }),
            hover_provider: Some(HoverProviderCapability::Options(HoverOptions {
                work_done_progress_options: WorkDoneProgressOptions {
                    work_done_progress: Some(true),
                },
            })),
            definition_provider: Some(OneOf::Right(DefinitionOptions {
                work_done_progress_options: WorkDoneProgressOptions {
                    work_done_progress: Some(true),
                },
            })),
            references_provider: Some(OneOf::Right(ReferenceOptions {
                work_done_progress_options: WorkDoneProgressOptions {
                    work_done_progress: Some(true),
                },
            })),
            folding_range_provider: Some(FoldingRangeProviderCapability::Options(
                StaticTextDocumentColorProviderOptions {
                    document_selector: None,
                    id: None,
                },
            )),
            ..Default::default()
        };

        Ok(InitializeResult {
            capabilities,
            server_info: Some(ServerInfo {
                name: String::from("GameScript LSP"),
                version: Some("0.1.0".to_string()),
            }),
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        trace!("gs lsp initialised");

        if let Some(validation_path) = &self.validation_path {
            let validators = gs_diagnostics::soup::load_validators(validation_path);
            let _ = self.validators.set(validators);
        }

        self.client
            .log_message(MessageType::INFO, "gs lsp initialised")
            .await;
    }

    async fn shutdown(&self) -> tower_lsp_server::jsonrpc::Result<()> {
        trace!("Shutting down");
        self.parsed_files.clear();
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let document_path = params.text_document.uri.to_file_path();
        if let Some(document_path) = document_path {
            if !document_path.exists() {
                return;
            }

            // Bust cache
            self.ast_cache.bust(&document_path);

            let path = document_path.to_string_lossy().to_string();

            if let Some(mut exists) = self.parsed_files.get_mut(&path) {
                exists.count += 1;
            }

            let workspace_folders = self.workspace_folders().await;

            let progress = self
                .client
                .progress(ProgressToken::String(path), "Processing files")
                .with_percentage(0)
                .with_message(format!("Processing file: {:?}", document_path.file_name()))
                .begin()
                .await;

            if params.text_document.language_id == GAME_SCRIPT_LANGUAGE_ID {
                self.process_gs_file(
                    &document_path,
                    &params.text_document.text,
                    &workspace_folders,
                    true,
                    &progress,
                )
                .await;
            } else if params.text_document.language_id == SOUP_LANGUAGE_ID {
                self.process_soup_file(
                    &document_path,
                    &params.text_document.text,
                    &workspace_folders,
                    true,
                    &progress,
                )
                .await;
            }
            trace!("did_open: {:?}", document_path);
            progress.finish().await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let document_path = params.text_document.uri.to_file_path();
        if let Some(document_path) = document_path {
            if !document_path.exists() {
                return;
            }
            let path = document_path.to_string_lossy().to_string();

            trace!("did_close {:?} {:?}", path, document_path);

            if let Some(mut exists) = self.parsed_files.get_mut(&path) {
                if exists.count > 0 {
                    exists.count -= 1;
                }
            }
        }

        self.parsed_files.retain(|_, val| val.count > 0);
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(document_path) = params.text_document.uri.to_file_path() {
            if !document_path.exists() {
                return;
            }

            // Bust cache
            self.ast_cache.bust(&document_path);

            let path = document_path.to_string_lossy().to_string();
            let text = params.content_changes.iter().next().unwrap().text.as_str();

            let workspace_folders = self.workspace_folders().await;

            let progress = self
                .client
                .progress(ProgressToken::String(String::from(path)), "Updating file")
                .with_percentage(0)
                .with_message(format!("Updating file: {:?}", document_path.file_name()))
                .begin()
                .await;
            if let Some(path_str) = document_path.to_str() {
                let file_type = {
                    let file = self.parsed_files.get(path_str);
                    file.map(|f| f.parsed.clone())
                };
                if let Some(file_type) = file_type {
                    if let ParsedFileType::Soup(_soup) = file_type {
                        self.process_soup_file(
                            &document_path,
                            text,
                            &workspace_folders,
                            true,
                            &progress,
                        )
                        .await;
                    } else if let ParsedFileType::GameScript(_program) = file_type {
                        self.process_gs_file(
                            &document_path,
                            text,
                            &workspace_folders,
                            true,
                            &progress,
                        )
                        .await;
                    }
                }
            }
            trace!("did_change {:?}", document_path);
            progress.finish().await;
        }
    }

    async fn diagnostic(
        &self,
        params: DocumentDiagnosticParams,
    ) -> tower_lsp_server::jsonrpc::Result<DocumentDiagnosticReportResult> {
        let work_done_token = params.work_done_progress_params.work_done_token.clone();
        let progress = if let Some(token) = work_done_token {
            let progress = self
                .client
                .progress(token, "Calculating diagnostics")
                .with_percentage(0)
                .begin()
                .await;
            Some(progress)
        } else {
            None
        };

        let mut result: Vec<Diagnostic> = vec![];
        let document_path = params.text_document.uri.to_file_path();
        if let Some(document_path) = document_path {
            if !document_path.exists() {
                if let Some(progress) = progress {
                    progress.finish().await;
                }
                return Ok(DocumentDiagnosticReportResult::Report(
                    DocumentDiagnosticReport::Unchanged(RelatedUnchangedDocumentDiagnosticReport {
                        related_documents: None,
                        unchanged_document_diagnostic_report: UnchangedDocumentDiagnosticReport {
                            result_id: String::from(""),
                        },
                    }),
                ));
            }
            let path = document_path.to_string_lossy().to_string();
            trace!("Document path: {:?}", path);

            if let Some(progress) = &progress {
                progress
                    .report_with_message("Accessing parsed file", 25)
                    .await;
            }

            let (parsed_file_type, diagnostics_lock) = if let Some(document) =
                self.parsed_files.get(&path)
            {
                (document.parsed.clone(), document.diagnostics.clone())
            } else {
                if let Some(progress) = progress {
                    progress.finish().await;
                }
                return Ok(DocumentDiagnosticReportResult::Report(
                    DocumentDiagnosticReport::Unchanged(RelatedUnchangedDocumentDiagnosticReport {
                        related_documents: None,
                        unchanged_document_diagnostic_report: UnchangedDocumentDiagnosticReport {
                            result_id: String::from(""),
                        },
                    }),
                ));
            };

            if let Some(progress) = &progress {
                progress
                    .report_with_message("Running diagnostic analysis", 50)
                    .await;
            }

            let diagnostics = diagnostics_lock.get_or_init(|| match &parsed_file_type {
                ParsedFileType::GameScript(program) => gs_diagnostics(program),
                ParsedFileType::Soup(soup) => {
                    if let Some(validators) = self.validators.get() {
                        soup_diagnostics(soup, validators, Some(&document_path))
                    } else if let Some(validation_path) = &self.validation_path {
                        // Fallback if not yet initialized or failed to load
                        let validators = gs_diagnostics::soup::load_validators(validation_path);
                        soup_diagnostics(soup, &validators, Some(&document_path))
                    } else {
                        vec![]
                    }
                }
            });

            if let Some(progress) = &progress {
                progress.report_with_message("Collecting results", 90).await;
            }

            result.extend(diagnostics.clone());
        }

        if let Some(progress) = progress {
            progress.finish().await;
        }

        Ok(DocumentDiagnosticReportResult::Report(
            DocumentDiagnosticReport::Full(RelatedFullDocumentDiagnosticReport {
                related_documents: None,
                full_document_diagnostic_report: FullDocumentDiagnosticReport {
                    result_id: None,
                    items: result,
                },
            }),
        ))
    }

    async fn workspace_diagnostic(
        &self,
        _params: WorkspaceDiagnosticParams,
    ) -> tower_lsp_server::jsonrpc::Result<WorkspaceDiagnosticReportResult> {
        Ok(WorkspaceDiagnosticReportResult::Report(
            WorkspaceDiagnosticReport { items: vec![] },
        ))
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> tower_lsp_server::jsonrpc::Result<Option<SemanticTokensResult>> {
        trace!("Semantic Tokens Full {:?}", params);

        let work_done_token = params.work_done_progress_params.work_done_token.clone();
        let progress = if let Some(token) = work_done_token {
            let progress = self
                .client
                .progress(token, "Calculating semantic tokens")
                .begin()
                .await;
            Some(progress)
        } else {
            None
        };

        let document_path = params.text_document.uri.to_file_path().ok_or_else(|| {
            // If progress is active, it's better to finish it here if we want to follow the diagnostic's pattern.
            // However, returning Err early is tricky. Let's see if we can handle the error more gracefully.
            Error::invalid_request()
        });

        let document_path = match document_path {
            Ok(p) => p,
            Err(e) => {
                if let Some(progress) = progress {
                    progress.finish().await;
                }
                return Err(e);
            }
        };

        if !document_path.exists() {
            if let Some(progress) = progress {
                progress.finish().await;
            }
            return Err(Error::invalid_request());
        }
        let path = document_path.to_string_lossy().to_string();
        trace!("semantic_tokens_full {:?} {:?}", path, document_path);

        let (parsed_file_type, comments, semantic_tokens_lock) =
            if let Some(document) = self.parsed_files.get(&path) {
                (
                    document.parsed.clone(),
                    document.comments.clone(),
                    document.semantic_tokens.clone(),
                )
            } else {
                if let Some(progress) = progress {
                    progress.finish().await;
                }
                return Ok(None);
            };

        if semantic_tokens_lock.get().is_none() {
            let validators = self.validators.get().cloned();
            let tokens = tokio::task::spawn_blocking(move || {
                let mut raw_tokens = match &parsed_file_type {
                    ParsedFileType::GameScript(program) => semantic_tokens(program),
                    ParsedFileType::Soup(soup) => soup_semantic_tokens(soup, validators.as_ref()),
                };
                let mut comment_tokens =
                    gs_semantic_tokens::comments::comments_semantic_tokens(&comments);
                raw_tokens.append(&mut comment_tokens);
                gs_semantic_tokens::process_raw_tokens(raw_tokens)
            })
            .await
            .map_err(|e| {
                error!("Error in semantic_tokens_full: {:?}", e);
                Error::internal_error()
            })?;
            let _ = semantic_tokens_lock.set(tokens);
        }

        let tokens = semantic_tokens_lock.get().unwrap();

        let result = Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: tokens.clone(),
        }));

        if let Some(progress) = progress {
            progress.finish().await;
        }

        Ok(result)
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> tower_lsp_server::jsonrpc::Result<Option<DocumentSymbolResponse>> {
        trace!("Document Symbol {:?}", params);

        let work_done_token = params.work_done_progress_params.work_done_token.clone();
        let progress = if let Some(token) = work_done_token {
            let progress = self
                .client
                .progress(token, "Calculating document symbols")
                .begin()
                .await;
            Some(progress)
        } else {
            None
        };

        let document_path = params
            .text_document
            .uri
            .to_file_path()
            .ok_or_else(|| Error::invalid_request());

        let document_path = match document_path {
            Ok(p) => p,
            Err(e) => {
                if let Some(progress) = progress {
                    progress.finish().await;
                }
                return Err(e);
            }
        };

        if !document_path.exists() {
            if let Some(progress) = progress {
                progress.finish().await;
            }
            return Err(Error::invalid_request());
        }
        let path = document_path.to_string_lossy().to_string();
        trace!("document_symbol {:?} {:?}", path, document_path);

        let (parsed_file_type, document_symbols_lock) =
            if let Some(document) = self.parsed_files.get(&path) {
                (document.parsed.clone(), document.document_symbols.clone())
            } else {
                if let Some(progress) = progress {
                    progress.finish().await;
                }
                return Ok(None);
            };

        if document_symbols_lock.get().is_none() {
            let validators = self.validators.get().cloned();
            let symbols = tokio::task::spawn_blocking(move || match &parsed_file_type {
                ParsedFileType::GameScript(program) => gs_symboliser(program),
                ParsedFileType::Soup(soup) => soup_symboliser(soup, validators.as_ref()),
            })
            .await
            .map_err(|e| {
                error!("Error in document_symbol: {:?}", e);
                Error::internal_error()
            })?;
            let _ = document_symbols_lock.set(symbols);
        }

        let symbols = document_symbols_lock.get().unwrap();

        let result = Some(DocumentSymbolResponse::Nested(symbols.clone()));

        if let Some(progress) = progress {
            progress.finish().await;
        }

        Ok(result)
    }

    async fn document_link(
        &self,
        params: DocumentLinkParams,
    ) -> tower_lsp_server::jsonrpc::Result<Option<Vec<DocumentLink>>> {
        trace!("Document Link {:?}", params);

        let work_done_token = params.work_done_progress_params.work_done_token.clone();
        let progress = if let Some(token) = work_done_token {
            let progress = self
                .client
                .progress(token, "Calculating document links")
                .begin()
                .await;
            Some(progress)
        } else {
            None
        };

        let document_path = params
            .text_document
            .uri
            .to_file_path()
            .ok_or_else(|| Error::invalid_request());

        let document_path = match document_path {
            Ok(p) => p,
            Err(e) => {
                if let Some(progress) = progress {
                    progress.finish().await;
                }
                return Err(e);
            }
        };

        if !document_path.exists() {
            if let Some(progress) = progress {
                progress.finish().await;
            }
            return Err(Error::invalid_request());
        }
        let path = document_path.to_string_lossy().to_string();
        trace!("document_link {:?} {:?}", path, document_path);

        let mut result = None;
        if let Some(document) = self.parsed_files.get(&path) {
            if let ParsedFileType::GameScript(program) = &document.parsed {
                let links = program
                    .includes
                    .par_iter()
                    .map(|include| DocumentLink {
                        range: include.range,
                        target: include
                            .path
                            .clone()
                            .and_then(|path| Uri::from_file_path(path)),
                        tooltip: Some(format!(
                            "Path: {}",
                            include
                                .path
                                .clone()
                                .map(|path| path.to_str().unwrap_or(&include.name).to_string())
                                .unwrap_or(include.name.clone())
                        )),
                        data: None,
                    })
                    .collect();
                result = Some(links);
            }
        }

        if let Some(progress) = progress {
            progress.finish().await;
        }

        Ok(result)
    }

    async fn document_link_resolve(
        &self,
        _params: DocumentLink,
    ) -> tower_lsp_server::jsonrpc::Result<DocumentLink> {
        todo!()
    }

    async fn completion(
        &self,
        params: CompletionParams,
    ) -> tower_lsp_server::jsonrpc::Result<Option<CompletionResponse>> {
        trace!("Completion {:?}", params);

        let work_done_token = params.work_done_progress_params.work_done_token.clone();
        let progress = if let Some(token) = work_done_token {
            let progress = self
                .client
                .progress(token, "Calculating completions")
                .begin()
                .await;
            Some(progress)
        } else {
            None
        };

        let document_path = params
            .text_document_position
            .text_document
            .uri
            .to_file_path()
            .ok_or_else(|| Error::invalid_request());

        let document_path = match document_path {
            Ok(p) => p,
            Err(e) => {
                if let Some(progress) = progress {
                    progress.finish().await;
                }
                return Err(e);
            }
        };

        if !document_path.exists() {
            if let Some(progress) = progress {
                progress.finish().await;
            }
            return Err(Error::invalid_request());
        }
        let path = document_path.to_string_lossy().to_string();

        let mut result = None;
        if let Some(file_info) = self.parsed_files.get(&path) {
            if let ParsedFileType::Soup(_soup) = &file_info.parsed {
                result = Some(CompletionResponse::Array(soup_completions(
                    _soup,
                    params,
                    self.validation_path.clone(),
                )));
            } else if let ParsedFileType::GameScript(_program) = &file_info.parsed {
                result = Some(CompletionResponse::Array(gs_completions(_program, params)));
            }
        }

        if let Some(progress) = progress {
            progress.finish().await;
        }

        Ok(result)
    }

    async fn references(
        &self,
        params: ReferenceParams,
    ) -> tower_lsp_server::jsonrpc::Result<Option<Vec<Location>>> {
        trace!("References {:?}", params);

        let work_done_token = params.work_done_progress_params.work_done_token.clone();
        let progress = if let Some(token) = work_done_token {
            let progress = self
                .client
                .progress(token, "Finding references")
                .begin()
                .await;
            Some(progress)
        } else {
            None
        };

        let document_path = params
            .text_document_position
            .text_document
            .uri
            .to_file_path()
            .ok_or_else(|| Error::invalid_request());

        let document_path = match document_path {
            Ok(p) => p,
            Err(e) => {
                if let Some(progress) = progress {
                    progress.finish().await;
                }
                return Err(e);
            }
        };

        if !document_path.exists() {
            if let Some(progress) = progress {
                progress.finish().await;
            }
            return Err(Error::invalid_request());
        }
        let path = document_path.to_string_lossy().to_string();

        let mut result = None;
        if let Some(document) = self.parsed_files.get(&path) {
            match &document.parsed {
                ParsedFileType::GameScript(program) => {
                    let gs_programs = DashMap::new();
                    let references_params = params.clone();
                    let program_clone = program.clone();

                    for entry in self.parsed_files.iter() {
                        if let ParsedFileType::GameScript(p) = &entry.value().parsed {
                            gs_programs.insert(entry.key().clone(), p.clone());
                        }
                    }

                    let locations = tokio::task::spawn_blocking(move || {
                        gs_find_references(program_clone, references_params, &gs_programs)
                    })
                    .await
                    .unwrap();

                    if let Some(locations) = locations {
                        result = Some(locations);
                    }
                }
                ParsedFileType::Soup(_) => {}
            }
        }

        if let Some(progress) = progress {
            progress.finish().await;
        }

        Ok(result)
    }

    async fn signature_help(
        &self,
        params: SignatureHelpParams,
    ) -> tower_lsp_server::jsonrpc::Result<Option<SignatureHelp>> {
        trace!("Signature Help {:?}", params);

        let work_done_token = params.work_done_progress_params.work_done_token.clone();
        let progress = if let Some(token) = work_done_token {
            let progress = self
                .client
                .progress(token, "Calculating signature help")
                .begin()
                .await;
            Some(progress)
        } else {
            None
        };

        if let Some(progress) = progress {
            progress.finish().await;
        }

        Ok(None)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> tower_lsp_server::jsonrpc::Result<Option<GotoDefinitionResponse>> {
        trace!("Goto Definition {:?}", params);

        let work_done_token = params.work_done_progress_params.work_done_token.clone();
        let progress = if let Some(token) = work_done_token {
            let progress = self
                .client
                .progress(token, "Calculating definition")
                .begin()
                .await;
            Some(progress)
        } else {
            None
        };

        let document_path = params
            .text_document_position_params
            .text_document
            .uri
            .to_file_path()
            .ok_or_else(|| Error::invalid_request());

        let document_path = match document_path {
            Ok(p) => p,
            Err(e) => {
                if let Some(progress) = progress {
                    progress.finish().await;
                }
                return Err(e);
            }
        };

        if !document_path.exists() {
            if let Some(progress) = progress {
                progress.finish().await;
            }
            return Err(Error::invalid_request());
        }
        let path = document_path.to_string_lossy().to_string();
        let position = params.text_document_position_params.position;

        let mut result = None;
        if let Some(document) = self.parsed_files.get(&path) {
            match &document.parsed {
                ParsedFileType::GameScript(program) => {
                    let gs_programs = DashMap::new();
                    let program_clone = program.clone();
                    let uri_clone = params
                        .text_document_position_params
                        .text_document
                        .uri
                        .clone();

                    for entry in self.parsed_files.iter() {
                        if let ParsedFileType::GameScript(p) = &entry.value().parsed {
                            gs_programs.insert(entry.key().clone(), p.clone());
                        }
                    }

                    let locations = tokio::task::spawn_blocking(move || {
                        gs_goto_definition(program_clone, position, uri_clone, &gs_programs)
                    })
                    .await
                    .unwrap();

                    if let Some(locations) = locations {
                        result = Some(locations);
                    }
                }
                ParsedFileType::Soup(soup) => {
                    let validators = self.validators.get().cloned().unwrap_or_default();
                    let base_path = document_path.parent();
                    if let Some(locations) = soup_goto_definition(
                        soup,
                        position,
                        params
                            .text_document_position_params
                            .text_document
                            .uri
                            .clone(),
                        &validators,
                        base_path,
                    ) {
                        result = Some(GotoDefinitionResponse::Array(locations));
                    }
                }
            }
        }

        if let Some(progress) = progress {
            progress.finish().await;
        }

        Ok(result)
    }

    async fn hover(&self, params: HoverParams) -> tower_lsp_server::jsonrpc::Result<Option<Hover>> {
        trace!("Hover {:?}", params);

        let work_done_token = params.work_done_progress_params.work_done_token.clone();
        let progress = if let Some(token) = work_done_token {
            let progress = self
                .client
                .progress(token, "Calculating hover")
                .with_percentage(0)
                .begin()
                .await;
            Some(progress)
        } else {
            None
        };

        let mut hover_result = None;

        let document_path = params
            .text_document_position_params
            .text_document
            .uri
            .to_file_path();

        if let Some(path) = &document_path {
            if let Some(document) = self.parsed_files.get(&path.to_string_lossy().to_string()) {
                if let ParsedFileType::Soup(soup) = &document.parsed {
                    if let Some(progress) = &progress {
                        progress
                            .report_with_message("Searching Soup file", 25)
                            .await;
                    }
                    if let Some(validators) = self.validators.get() {
                        hover_result = soup_hover(soup, params.clone(), validators);
                    } else if let Some(validation_path) = &self.validation_path {
                        let validators = gs_diagnostics::soup::load_validators(validation_path);
                        hover_result = soup_hover(soup, params.clone(), &validators);
                    }
                }
            }
        }

        if hover_result.is_some() {
            if let Some(progress) = progress {
                progress.finish().await;
            }
            return Ok(hover_result);
        }

        let definition_params = GotoDefinitionParams {
            text_document_position_params: params.text_document_position_params.clone(),
            work_done_progress_params: params.work_done_progress_params.clone(),
            partial_result_params: tower_lsp_server::ls_types::PartialResultParams::default(),
        };

        if let Some(progress) = &progress {
            progress
                .report_with_message("Looking up definition", 50)
                .await;
        }

        if let Ok(Some(goto_response)) = self.goto_definition(definition_params).await {
            if let Some(progress) = &progress {
                progress
                    .report_with_message("Extracting comments", 75)
                    .await;
            }
            let (target, origin_range) = match goto_response {
                tower_lsp_server::ls_types::GotoDefinitionResponse::Scalar(loc) => {
                    (Some((loc.uri, loc.range)), None)
                }
                tower_lsp_server::ls_types::GotoDefinitionResponse::Array(locs) => {
                    let loc = locs.into_iter().next();
                    (loc.clone().map(|l| (l.uri, l.range)), None)
                }
                tower_lsp_server::ls_types::GotoDefinitionResponse::Link(links) => {
                    let link = links.into_iter().next();
                    (
                        link.clone().map(|l| (l.target_uri, l.target_range)),
                        link.and_then(|l| l.origin_selection_range),
                    )
                }
            };

            if let Some((target_uri, target_range)) = target {
                if let Some(target_path) = target_uri.to_file_path() {
                    let path_str = target_path.to_string_lossy().to_string();
                    if let Some(document) = self.parsed_files.get(&path_str) {
                        use gs_ast::find::HasRange;
                        let comments = &document.comments;
                        let definition_line = target_range.start.line;

                        let mut preceding_comment = None;
                        if definition_line == 0 {
                            if let Some(first_comment) = comments.comments.first() {
                                // Assume it's the "file comment" if it starts within the first 2 lines
                                if first_comment.range().start.line <= 2 {
                                    preceding_comment = Some(first_comment);
                                }
                            }
                        } else {
                            for comment in &comments.comments {
                                let range = comment.range();
                                if range.end.line < definition_line {
                                    preceding_comment = Some(comment);
                                } else {
                                    if range.start.line >= definition_line {
                                        break;
                                    }
                                }
                            }
                        }

                        if let Some(comment) = preceding_comment {
                            // Only include if it is immediately preceding (e.g. within 2 lines)
                            let range = comment.range();
                            if definition_line == 0
                                || definition_line.saturating_sub(range.end.line) <= 2
                            {
                                let raw_text = match comment {
                                    gs_ast::comments::Comment::LineComment(c) => c.text.clone(),
                                    gs_ast::comments::Comment::BlockComment(c) => c.text.clone(),
                                    gs_ast::comments::Comment::GroupComment(c) => c
                                        .comments
                                        .iter()
                                        .map(|lc| lc.text.as_str())
                                        .collect::<Vec<_>>()
                                        .join("\n"),
                                };

                                let text = raw_text
                                    .lines()
                                    .filter_map(|line| {
                                        let mut t = line.trim();
                                        if t.starts_with("/*") {
                                            t = t[2..].trim_start();
                                        }
                                        if t.ends_with("*/") {
                                            t = t[..t.len() - 2].trim_end();
                                        }
                                        if t.starts_with("//!") {
                                            t = t[3..].trim_start();
                                        } else if t.starts_with("//") {
                                            t = t[2..].trim_start();
                                        } else if t.starts_with('*') {
                                            t = t[1..].trim_start();
                                        }

                                        let t = t.trim();
                                        if !t.is_empty() && t.chars().all(|c| c == '=') {
                                            return None;
                                        }
                                        Some(t)
                                    })
                                    .collect::<Vec<&str>>()
                                    .join("\n\n");

                                hover_result = Some(Hover {
                                    contents: tower_lsp_server::ls_types::HoverContents::Markup(
                                        tower_lsp_server::ls_types::MarkupContent {
                                            kind: tower_lsp_server::ls_types::MarkupKind::Markdown,
                                            value: text,
                                        },
                                    ),
                                    range: origin_range,
                                });
                            }
                        }
                    }
                }
            }
        }

        if let Some(progress) = progress {
            progress.finish().await;
        }

        Ok(hover_result)
    }

    async fn folding_range(
        &self,
        params: FoldingRangeParams,
    ) -> tower_lsp_server::jsonrpc::Result<Option<Vec<FoldingRange>>> {
        trace!("Folding Range {:?}", params);

        let work_done_token = params.work_done_progress_params.work_done_token.clone();
        let progress = if let Some(token) = work_done_token {
            let progress = self
                .client
                .progress(token, "Calculating folding ranges")
                .begin()
                .await;
            Some(progress)
        } else {
            None
        };

        let document_path = params
            .text_document
            .uri
            .to_file_path()
            .ok_or_else(|| Error::invalid_request());

        let document_path = match document_path {
            Ok(p) => p,
            Err(e) => {
                if let Some(progress) = progress {
                    progress.finish().await;
                }
                return Err(e);
            }
        };

        if !document_path.exists() {
            if let Some(progress) = progress {
                progress.finish().await;
            }
            return Err(Error::invalid_request());
        }
        let path = document_path.to_string_lossy().to_string();
        trace!("folding_range {:?} {:?}", path, document_path);

        let (parsed_file_type, comments, folding_ranges_lock) =
            if let Some(document) = self.parsed_files.get(&path) {
                (
                    document.parsed.clone(),
                    document.comments.clone(),
                    document.folding_ranges.clone(),
                )
            } else {
                if let Some(progress) = progress {
                    progress.finish().await;
                }
                return Ok(None);
            };

        let ranges = folding_ranges_lock.get_or_init(|| {
            let mut ranges = match &parsed_file_type {
                ParsedFileType::GameScript(program) => gs_folding_range(program),
                ParsedFileType::Soup(soup) => soup_folding_range(soup),
            };
            ranges.extend(gs_folding::comments::comments_folding_range(&comments));
            ranges
        });

        let result = Some(ranges.clone());

        if let Some(progress) = progress {
            progress.finish().await;
        }

        Ok(result)
    }
}

impl GameScriptLanguageServer {
    pub fn new(
        client: Client,
        validation_path: Option<PathBuf>,
        search_paths: Vec<PathBuf>,
    ) -> Self {
        trace!("Create GameScriptLanguageServer");

        trace!("Search paths {:?}", search_paths);

        Self {
            client,
            search_paths,
            validation_path,
            validators: Arc::new(OnceLock::new()),
            parsed_files: DashMap::new(),
            currently_processing: DashSet::new(),
            ast_cache: AstCache::new(),
        }
    }
}
