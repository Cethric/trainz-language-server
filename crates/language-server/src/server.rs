use crate::process::acs_binary::ProcessAcsBinary;
use crate::process::acs_text::ProcessAcsText;
use crate::process::gs::ProcessGS;
use crate::state::{ParsedFile, ParsedFileType, RecursiveIncludeResolver, TrainzLanguageServer};
use dashmap::DashMap;
use ls_types::CodeActionProviderCapability;
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tower_lsp_server::jsonrpc::Error;
use tower_lsp_server::ls_types::{
    CodeAction, CodeActionKind, CodeActionOptions, CodeActionOrCommand, CodeActionParams,
    CodeActionResponse, CompletionOptions, CompletionOptionsCompletionItem, CompletionParams,
    CompletionResponse, CreateFilesParams, DefinitionOptions, DeleteFilesParams, Diagnostic,
    DiagnosticOptions, DiagnosticServerCapabilities, DidChangeConfigurationParams,
    DidChangeTextDocumentParams, DidChangeWatchedFilesParams, DidChangeWorkspaceFoldersParams,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, DocumentDiagnosticParams,
    DocumentDiagnosticReport, DocumentDiagnosticReportResult, DocumentFilter, DocumentLink,
    DocumentLinkOptions, DocumentLinkParams, DocumentSymbolOptions, DocumentSymbolParams,
    DocumentSymbolResponse, FoldingRange, FoldingRangeParams, FoldingRangeProviderCapability,
    FullDocumentDiagnosticReport, GotoDefinitionParams, GotoDefinitionResponse, Hover,
    HoverOptions, HoverParams, HoverProviderCapability, InitializeParams, InitializeResult,
    InitializedParams, InlineCompletionOptions, Location, MessageType, OneOf, PositionEncodingKind,
    ProgressToken, ReferenceOptions, ReferenceParams, RelatedFullDocumentDiagnosticReport,
    RelatedUnchangedDocumentDiagnosticReport, RenameFilesParams, SemanticTokens,
    SemanticTokensFullOptions, SemanticTokensLegend, SemanticTokensOptions, SemanticTokensParams,
    SemanticTokensResult, SemanticTokensServerCapabilities, ServerCapabilities, ServerInfo,
    SignatureHelp, SignatureHelpOptions, SignatureHelpParams,
    StaticTextDocumentColorProviderOptions, TextDocumentSyncCapability, TextDocumentSyncKind,
    TextDocumentSyncOptions, TextEdit, UnchangedDocumentDiagnosticReport, Uri,
    WorkDoneProgressOptions, WorkspaceDiagnosticParams, WorkspaceDiagnosticReport,
    WorkspaceDiagnosticReportResult, WorkspaceEdit, WorkspaceFileOperationsServerCapabilities,
    WorkspaceFoldersServerCapabilities, WorkspaceServerCapabilities, WorkspaceSymbol,
    WorkspaceSymbolOptions, WorkspaceSymbolParams, WorkspaceSymbolResponse,
};
use tower_lsp_server::{LanguageServer, ls_types};
use tracing::{debug, error, trace};
use trainz_common::language_id::{ACS_TEXT_LANGUAGE_ID, GAME_SCRIPT_LANGUAGE_ID};
use trainz_completions::acs_text::acs_text_completions;
use trainz_definition::acs_text::definitions::{ScriptResolver, acs_text_goto_definition};
use trainz_definition::gs::definitions::gs_goto_definition;
use trainz_definition::gs::references::gs_find_references;
use trainz_diagnostics::acs_text::acs_text_diagnostics;
use trainz_diagnostics::gs;
use trainz_folding::acs_text::acs_text_folding_range;
use trainz_folding::gs::trainz_folding_range;
use trainz_hover::acs_text::acs_text_hover;
use trainz_semantic_tokens::acs_text::acs_text_semantic_tokens;
use trainz_semantic_tokens::gs::semantic_tokens;
use trainz_symboliser::acs_text::acs_text_symboliser;

impl LanguageServer for TrainzLanguageServer {
    #[tracing::instrument(skip(self, params))]
    async fn initialize(
        &self,
        params: InitializeParams,
    ) -> tower_lsp_server::jsonrpc::Result<InitializeResult> {
        self.client
            .log_message(
                MessageType::INFO,
                format!(
                    "Trainz Language Server is initialised with version {}",
                    self.version
                ),
            )
            .await;

        let work_done_token = params.work_done_progress_params.work_done_token.clone();
        let progress = if let Some(token) = work_done_token {
            let progress = self
                .client
                .progress(token, "Initializing")
                .with_percentage(0)
                .begin()
                .await;
            Some(progress)
        } else {
            None
        };

        trace!("Initialising Trainz Language Server {:?}", params);

        let encoding = if let Some(encs) = params
            .capabilities
            .general
            .as_ref()
            .and_then(|g| g.position_encodings.as_ref())
        {
            if encs.contains(&PositionEncodingKind::UTF16) {
                PositionEncodingKind::UTF16
            } else {
                PositionEncodingKind::UTF8
            }
        } else {
            PositionEncodingKind::UTF16
        };

        #[allow(deprecated)]
        if let Some(folders) = params.workspace_folders {
            for folder in folders {
                if let Some(path) = folder.uri.to_file_path() {
                    self.workspace_folders.insert(path.to_path_buf());
                }
            }
        } else if let Some(root_uri) = params.root_uri {
            if let Some(path) = root_uri.to_file_path() {
                self.workspace_folders.insert(path.to_path_buf());
            }
        } else if let Some(root_path) = params.root_path {
            self.workspace_folders.insert(PathBuf::from(root_path));
        }

        let capabilities = ServerCapabilities {
            position_encoding: Some(encoding),
            text_document_sync: Some(TextDocumentSyncCapability::Options(
                TextDocumentSyncOptions {
                    open_close: Some(true),
                    change: Some(TextDocumentSyncKind::FULL),
                    will_save: None,
                    will_save_wait_until: None,
                    save: None,
                },
            )),
            workspace: Some(WorkspaceServerCapabilities {
                workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                    supported: Some(true),
                    change_notifications: Some(OneOf::Right(String::from(
                        "trainz-language-server",
                    ))),
                }),
                file_operations: Some(WorkspaceFileOperationsServerCapabilities {
                    did_create: None,
                    will_create: None,
                    did_delete: None,
                    will_delete: None,
                    did_rename: None,
                    will_rename: None,
                }),
            }),
            diagnostic_provider: Some(DiagnosticServerCapabilities::Options(DiagnosticOptions {
                identifier: Some("trainz-language-server".to_string()),
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
                            trainz_semantic_tokens::legend::get_legend();
                        SemanticTokensLegend {
                            token_modifiers,
                            token_types,
                        }
                    },
                }),
            ),
            document_symbol_provider: Some(OneOf::Right(DocumentSymbolOptions {
                label: Some(String::from("Trainz")),
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
            inline_completion_provider: Some(OneOf::Right(InlineCompletionOptions {
                work_done_progress_options: WorkDoneProgressOptions {
                    work_done_progress: Some(true),
                },
            })),
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
            code_action_provider: Some(CodeActionProviderCapability::Options(CodeActionOptions {
                code_action_kinds: Some(vec![CodeActionKind::QUICKFIX]),
                work_done_progress_options: WorkDoneProgressOptions {
                    work_done_progress: Some(true),
                },
                resolve_provider: Some(false),
            })),
            references_provider: Some(OneOf::Right(ReferenceOptions {
                work_done_progress_options: WorkDoneProgressOptions {
                    work_done_progress: Some(true),
                },
            })),
            folding_range_provider: Some(FoldingRangeProviderCapability::Options(
                StaticTextDocumentColorProviderOptions {
                    document_selector: Some(vec![
                        DocumentFilter {
                            language: Some(GAME_SCRIPT_LANGUAGE_ID.to_string()),
                            scheme: Some(String::from("file")),
                            pattern: Some(String::from("*.gs")),
                        },
                        DocumentFilter {
                            language: Some(ACS_TEXT_LANGUAGE_ID.to_string()),
                            scheme: Some(String::from("file")),
                            pattern: Some(String::from("config.txt")),
                        },
                    ]),
                    id: Some(String::from("trainz-language-server")),
                },
            )),
            workspace_symbol_provider: Some(OneOf::Right(WorkspaceSymbolOptions {
                resolve_provider: Some(true),
                work_done_progress_options: WorkDoneProgressOptions {
                    work_done_progress: Some(true),
                },
            })),
            ..Default::default()
        };

        if let Some(progress) = progress {
            progress.finish().await;
        }

        Ok(InitializeResult {
            capabilities,
            offset_encoding: None,
            server_info: Some(ServerInfo {
                name: String::from("Trainz LSP"),
                version: Some(self.version.clone()),
            }),
        })
    }

    #[tracing::instrument(skip(self))]
    async fn initialized(&self, _: InitializedParams) {
        self.acs_state.load_graph().await;

        self.discover_projects().await;

        self.client
            .log_message(
                MessageType::INFO,
                "Trainz Language Server has been initialised",
            )
            .await;
    }

    #[tracing::instrument(skip(self))]
    async fn shutdown(&self) -> tower_lsp_server::jsonrpc::Result<()> {
        trace!("Shutting down");
        self.parsed_files.clear();
        Ok(())
    }

    #[tracing::instrument(skip(self, params))]
    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let document_path = params.text_document.uri.to_file_path();
        if let Some(document_path) = document_path {
            if !document_path.exists() {
                return;
            }

            let _permit = self.processing_semaphore.acquire().await.ok();

            // Bust cache
            self.ast_cache.bust(&document_path);

            self.add_file_to_project(&document_path);

            let path = document_path.to_string_lossy().to_string();

            self.increment_count(&path);

            let workspace_folders = self.workspace_folders();

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
            } else if params.text_document.language_id == ACS_TEXT_LANGUAGE_ID {
                self.process_acs_text_file(
                    &document_path,
                    &params.text_document.text,
                    true,
                    &progress,
                )
                .await;
            }
            trace!("did_open: {:?}", document_path);
            progress.finish().await;
        }
    }

    #[tracing::instrument(skip(self, params))]
    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let document_path = params.text_document.uri.to_file_path();
        if let Some(document_path) = document_path {
            if !document_path.exists() {
                return;
            }
            let path = document_path.to_string_lossy().to_string();

            trace!("did_close {:?} {:?}", path, document_path);

            self.decrement_count(&path);
        }
    }

    #[tracing::instrument(skip(self, params))]
    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(document_path) = params.text_document.uri.to_file_path() {
            if !document_path.exists() {
                return;
            }

            let _permit = self.processing_semaphore.acquire().await.ok();

            // Bust cache
            self.ast_cache.bust(&document_path);

            self.add_file_to_project(&document_path);

            let path = document_path.to_string_lossy().to_string();
            let text = params.content_changes.first().unwrap().text.as_str();

            let workspace_folders = self.workspace_folders();

            let progress = self
                .client
                .progress(ProgressToken::String(path), "Updating file")
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
                    if let ParsedFileType::AcsText(_acs_text) = file_type {
                        self.process_acs_text_file(&document_path, text, true, &progress)
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
                    } else if let ParsedFileType::AcsBinary(_results) = file_type
                        && let Ok(content) = fs::read(&document_path)
                    {
                        self.process_acs_binary_file(&document_path, &content, true, &progress)
                            .await;
                    }
                }
            }
            trace!("did_change {:?}", document_path);
            progress.finish().await;
        }
    }

    #[tracing::instrument(skip(self, params))]
    async fn did_create_files(&self, params: CreateFilesParams) {
        debug!("did_create_files {:?}", params);
    }

    #[tracing::instrument(skip(self, params))]
    async fn did_rename_files(&self, params: RenameFilesParams) {
        debug!("did_rename_files {:?}", params);

        for event in params.files {
            trace!("did_rename_files {:?}", event);

            if let Some(exists) = self.parsed_files.remove(&event.old_uri) {
                self.parsed_files.insert(event.new_uri.clone(), exists.1);
            }

            if let Some(count) = self.counts.remove(&event.old_uri) {
                self.counts.insert(event.new_uri, count.1);
            }
        }
    }

    #[tracing::instrument(skip(self, params))]
    async fn did_delete_files(&self, params: DeleteFilesParams) {
        debug!("did_delete_files {:?}", params);

        for event in params.files {
            if let Ok(uri) = event.uri.parse::<Uri>() {
                if let Some(path) = uri.to_file_path() {
                    self.remove_file_from_project(&path);
                    self.decrement_count(&path.to_string_lossy());
                } else {
                    self.decrement_count(&event.uri);
                }
            } else {
                self.decrement_count(&event.uri);
            }

            trace!("did_delete_files {:?}", event.uri);
        }
    }

    #[tracing::instrument(skip(self, params))]
    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        for event in params.changes {
            if let Some(document_path) = event.uri.to_file_path() {
                if !document_path.exists() {
                    self.remove_file_from_project(&document_path);
                    continue;
                }

                let _permit = self.processing_semaphore.acquire().await.ok();

                // Bust cache
                self.ast_cache.bust(&document_path);

                self.add_file_to_project(&document_path);

                let path = document_path.to_string_lossy().to_string();

                if let Ok(document) = fs::read(path.clone()) {
                    let source = String::from_utf8_lossy(&document);

                    let workspace_folders = self.workspace_folders();

                    let progress = self
                        .client
                        .progress(ProgressToken::String(path), "Updating file")
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
                            if let ParsedFileType::AcsText(_acs_text) = file_type {
                                self.process_acs_text_file(
                                    &document_path,
                                    &source,
                                    true,
                                    &progress,
                                )
                                .await;
                            } else if let ParsedFileType::GameScript(_program) = file_type {
                                self.process_gs_file(
                                    &document_path,
                                    &source,
                                    &workspace_folders,
                                    true,
                                    &progress,
                                )
                                .await;
                            } else if let ParsedFileType::AcsBinary(_results) = file_type {
                                self.process_acs_binary_file(
                                    &document_path,
                                    &document,
                                    true,
                                    &progress,
                                )
                                .await;
                            }
                        }
                    }
                    trace!("did_change_watched_files {:?}", document_path);
                    progress.finish().await;
                }
            }
        }
    }

    #[tracing::instrument(skip(self, params))]
    async fn did_change_workspace_folders(&self, params: DidChangeWorkspaceFoldersParams) {
        trace!("did_change_workspace_folders {:?}", params.event);
        for folder in params.event.removed {
            if let Some(path) = folder.uri.to_file_path() {
                self.workspace_folders.remove(&*path);
                // Also remove projects in this folder
                self.projects.retain(|root, _| !root.starts_with(&path));
            }
        }
        for folder in params.event.added {
            if let Some(path) = folder.uri.to_file_path() {
                self.workspace_folders.insert(path.to_path_buf());
            }
        }

        self.discover_projects().await;
    }

    #[tracing::instrument(skip(self, params))]
    async fn did_change_configuration(&self, params: DidChangeConfigurationParams) {
        debug!("did_change_configuration {:?}", params.settings);
    }

    #[tracing::instrument(skip(self, params))]
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
                ParsedFileType::GameScript(program) => {
                    let resolver = RecursiveIncludeResolver {
                        current_program: program,
                        parsed_files: &self.parsed_files,
                    };
                    gs::trainz_diagnostics(&path, program, &resolver, &resolver)
                }
                ParsedFileType::AcsText(acs_text) => tokio::task::block_in_place(move || {
                    tokio::runtime::Handle::current().block_on(async move {
                        if let guard = self.acs_state.graph.read().await
                            && let Some(graph) = guard.as_ref()
                        {
                            acs_text_diagnostics(acs_text, graph, Some(&document_path))
                        } else {
                            vec![]
                        }
                    })
                }),
                ParsedFileType::AcsBinary(_) => vec![],
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

    #[tracing::instrument(skip(self))]
    async fn workspace_diagnostic(
        &self,
        _params: WorkspaceDiagnosticParams,
    ) -> tower_lsp_server::jsonrpc::Result<WorkspaceDiagnosticReportResult> {
        Ok(WorkspaceDiagnosticReportResult::Report(
            WorkspaceDiagnosticReport { items: vec![] },
        ))
    }

    #[tracing::instrument(skip(self, params))]
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
            let tokens = tokio::task::block_in_place(move || {
                let (mut raw_tokens, src) = match &parsed_file_type {
                    ParsedFileType::GameScript(program) => {
                        (semantic_tokens(program), Some(program.src.as_str()))
                    }
                    ParsedFileType::AcsText(acs_text) => (
                        tokio::runtime::Handle::current().block_on(async move {
                            if let guard = self.acs_state.graph.read().await
                                && let Some(graph) = guard.as_ref()
                            {
                                acs_text_semantic_tokens(acs_text, graph)
                            } else {
                                vec![]
                            }
                        }),
                        Some(acs_text.src.as_str()),
                    ),
                    ParsedFileType::AcsBinary(_) => (vec![], None),
                };
                let mut comment_tokens =
                    trainz_semantic_tokens::comments::comments_semantic_tokens(&comments);
                raw_tokens.append(&mut comment_tokens);
                trainz_semantic_tokens::process_raw_tokens(raw_tokens, src)
            });
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

    #[tracing::instrument(skip(self, params))]
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
            .ok_or_else(Error::invalid_request);

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
            let parsed_files_clone = self.parsed_files.clone();
            let symbols = tokio::task::spawn_blocking(move || match &parsed_file_type {
                ParsedFileType::GameScript(program) => {
                    let resolver = RecursiveIncludeResolver {
                        current_program: program,
                        parsed_files: &parsed_files_clone,
                    };
                    trainz_symboliser::gs::trainz_symboliser(program, &resolver)
                }
                ParsedFileType::AcsText(acs_text) => acs_text_symboliser(acs_text),
                ParsedFileType::AcsBinary(_) => vec![],
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

    #[tracing::instrument(skip(self, params))]
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
            .ok_or_else(Error::invalid_request);

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
        if let Some(document) = self.parsed_files.get(&path)
            && let ParsedFileType::GameScript(program) = &document.parsed
        {
            let links = program
                .includes
                .par_iter()
                .map(|include| DocumentLink {
                    range: include.range,
                    target: include.path.clone().and_then(Uri::from_file_path),
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

        if let Some(progress) = progress {
            progress.finish().await;
        }

        Ok(result)
    }

    #[tracing::instrument(skip(self, _params))]
    async fn document_link_resolve(
        &self,
        _params: DocumentLink,
    ) -> tower_lsp_server::jsonrpc::Result<DocumentLink> {
        todo!()
    }

    #[tracing::instrument(skip(self, params))]
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
            .ok_or_else(Error::invalid_request);

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
            if let ParsedFileType::AcsText(_acs_text) = &file_info.parsed {
                result = Some(CompletionResponse::Array(acs_text_completions(
                    _acs_text,
                    params,
                    self.asset_cache_path.as_deref(),
                )));
            } else if let ParsedFileType::GameScript(_program) = &file_info.parsed {
                result = Some(CompletionResponse::Array(
                    trainz_completions::gs::trainz_completions(_program, params),
                ));
            }
        }

        if let Some(progress) = progress {
            progress.finish().await;
        }

        Ok(result)
    }

    #[tracing::instrument(skip(self, params))]
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
            .ok_or_else(Error::invalid_request);

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
                ParsedFileType::AcsText(_) => {}
                ParsedFileType::AcsBinary(_) => {}
            }
        }

        if let Some(progress) = progress {
            progress.finish().await;
        }

        Ok(result)
    }

    #[tracing::instrument(skip(self, params))]
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

    #[tracing::instrument(skip(self, params))]
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
            .ok_or_else(Error::invalid_request);

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
                ParsedFileType::AcsText(acs_text) => {
                    let base_path = document_path.parent();

                    let resolver = AcsTextScriptResolver {
                        parsed_files: &self.parsed_files,
                    };

                    if let Some(locations) = acs_text_goto_definition(
                        acs_text,
                        position,
                        params
                            .text_document_position_params
                            .text_document
                            .uri
                            .clone(),
                        base_path,
                        Some(&resolver),
                    ) {
                        result = Some(GotoDefinitionResponse::Array(locations));
                    }
                }
                ParsedFileType::AcsBinary(_) => {}
            }
        }

        if let Some(progress) = progress {
            progress.finish().await;
        }

        Ok(result)
    }

    #[tracing::instrument(skip(self, params))]
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

        if let Some(path) = &document_path
            && let Some(document) = self.parsed_files.get(&path.to_string_lossy().to_string())
        {
            match &document.parsed {
                ParsedFileType::AcsText(acs_text) => {
                    if let Some(progress) = &progress {
                        progress
                            .report_with_message("Searching AcsText file", 25)
                            .await;
                    }

                    hover_result = acs_text_hover(
                        acs_text,
                        params.clone(),
                        path.parent(),
                        Some(&AcsTextScriptResolver {
                            parsed_files: &self.parsed_files,
                        }),
                    );

                    if hover_result.is_some() {
                        if let Some(progress) = progress {
                            progress.finish().await;
                        }
                        return Ok(hover_result);
                    }
                }
                ParsedFileType::GameScript(program) => {
                    if let Some(progress) = &progress {
                        progress
                            .report_with_message("Searching GameScript file", 25)
                            .await;
                    }
                    let resolver = RecursiveIncludeResolver {
                        current_program: program,
                        parsed_files: &self.parsed_files,
                    };
                    hover_result = trainz_hover::gs::trainz_hover(
                        program,
                        &resolver,
                        &resolver,
                        params.text_document_position_params.position,
                    );
                }
                ParsedFileType::AcsBinary(_) => {}
            }
        }

        let definition_params = GotoDefinitionParams {
            text_document_position_params: params.text_document_position_params.clone(),
            work_done_progress_params: params.work_done_progress_params.clone(),
            partial_result_params: ls_types::PartialResultParams::default(),
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
                ls_types::GotoDefinitionResponse::Scalar(loc) => (Some((loc.uri, loc.range)), None),
                ls_types::GotoDefinitionResponse::Array(locs) => {
                    let loc = locs.into_iter().next();
                    (loc.clone().map(|l| (l.uri, l.range)), None)
                }
                ls_types::GotoDefinitionResponse::Link(links) => {
                    let link = links.into_iter().next();
                    (
                        link.clone()
                            .map(|l| (l.target_uri, l.target_selection_range)),
                        link.and_then(|l| l.origin_selection_range),
                    )
                }
            };

            if let Some((target_uri, target_range)) = target
                && let Some(target_path) = target_uri.to_file_path()
            {
                let path_str = target_path.to_string_lossy().to_string();
                if let Some(document) = self.parsed_files.get(&path_str) {
                    if hover_result.is_none()
                        && let ParsedFileType::GameScript(program) = &document.parsed
                    {
                        let resolver = RecursiveIncludeResolver {
                            current_program: program,
                            parsed_files: &self.parsed_files,
                        };
                        hover_result = trainz_hover::gs::trainz_hover(
                            program,
                            &resolver,
                            &resolver,
                            target_range.start,
                        );
                        if let Some(hr) = &mut hover_result {
                            hr.range = origin_range;
                        }
                    }

                    use trainz_ast::find::HasRange;
                    use trainz_ast::gs::find::{find_field_by_id_range, find_param_by_id_range};
                    let comments = &document.comments;
                    let definition_line = target_range.start.line;

                    let mut preceding_comment = None;
                    let mut following_comment = None;
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
                            } else if range.start.line == definition_line {
                                if range.start.character >= target_range.end.character {
                                    following_comment = Some(comment);
                                    break;
                                }
                            } else if range.start.line > definition_line {
                                break;
                            }
                        }
                    }

                    let mut is_param = false;
                    let mut is_field = false;
                    if let ParsedFileType::GameScript(program) = &document.parsed {
                        is_param = find_param_by_id_range(program, target_range).is_some();
                        is_field = find_field_by_id_range(program, target_range).is_some();
                    }

                    let mut comments_to_process = Vec::new();
                    if !is_param && let Some(comment) = preceding_comment {
                        // If it's a field and we have a following comment, ignore the preceding one.
                        if !(is_field && following_comment.is_some()) {
                            let range = comment.range();
                            if definition_line == 0
                                || definition_line.saturating_sub(range.end.line) <= 2
                            {
                                comments_to_process.push(comment);
                            }
                        }
                    }
                    if let Some(comment) = following_comment {
                        comments_to_process.push(comment);
                    }

                    if !comments_to_process.is_empty() {
                        let mut combined_text = Vec::new();
                        for comment in comments_to_process {
                            let raw_text = match comment {
                                trainz_ast::comments::Comment::LineComment(c) => c.text.clone(),
                                trainz_ast::comments::Comment::BlockComment(c) => c.text.clone(),
                                trainz_ast::comments::Comment::GroupComment(c) => c
                                    .comments
                                    .par_iter()
                                    .map(|lc| lc.text.as_str())
                                    .collect::<Vec<_>>()
                                    .join("\n"),
                            };

                            let tags = [
                                "Parm:",
                                "Desc:",
                                "Name:",
                                "Returns:",
                                "Retn:",
                                "File:",
                                "See Also:",
                            ];
                            let mut lines: Vec<String> = Vec::new();
                            let mut in_triple_slash_block = false;

                            for line in raw_text.lines() {
                                let mut t = line.trim();
                                let mut is_triple_slash = false;

                                if t.starts_with("/*") {
                                    t = t[2..].trim_start();
                                }
                                if t.ends_with("*/") {
                                    t = t[..t.len() - 2].trim_end();
                                }
                                if t.starts_with("//!") {
                                    t = t[3..].trim_start();
                                    is_triple_slash = true;
                                } else if t.starts_with("//") {
                                    t = t[2..].trim_start();
                                } else if t.starts_with('*') {
                                    t = t[1..].trim_start();
                                }

                                let t = t.trim();
                                if t.is_empty() {
                                    lines.push(String::new());
                                    in_triple_slash_block = false;
                                    continue;
                                }
                                if t.chars().all(|c| c == '=') {
                                    in_triple_slash_block = false;
                                    continue;
                                }

                                if is_triple_slash {
                                    let has_tag = tags.iter().any(|&tag| t.starts_with(tag));
                                    if !has_tag {
                                        if !in_triple_slash_block {
                                            lines.push(format!("Desc: {}", t));
                                        } else {
                                            lines.push(t.to_string());
                                        }
                                    } else {
                                        lines.push(t.to_string());
                                    }
                                    in_triple_slash_block = true;
                                } else {
                                    lines.push(t.to_string());
                                    in_triple_slash_block = false;
                                }
                            }

                            let mut processed_lines: Vec<String> = Vec::new();
                            for line in lines {
                                if line.is_empty() {
                                    processed_lines.push(line);
                                    continue;
                                }

                                let is_tag = tags.iter().any(|&tag| line.starts_with(tag));

                                if !is_tag
                                    && let Some(last) = processed_lines.last_mut()
                                    && !last.is_empty()
                                    && tags.iter().any(|&tag| last.starts_with(tag))
                                {
                                    last.push(' ');
                                    last.push_str(&line);
                                    continue;
                                }

                                processed_lines.push(line);
                            }

                            let text = processed_lines
                                .into_iter()
                                .filter(|s| !s.is_empty())
                                .collect::<Vec<_>>()
                                .join("\n\n");
                            if !text.is_empty() {
                                combined_text.push(text);
                            }
                        }

                        if !combined_text.is_empty() {
                            let text = combined_text.join("\n\n");
                            let mut value = text;
                            if let Some(hr) = &hover_result
                                && let ls_types::HoverContents::Markup(markup) = &hr.contents
                            {
                                value.push_str("\n\n---\n\n");
                                value.push_str(&markup.value);
                            }

                            hover_result = Some(Hover {
                                contents: ls_types::HoverContents::Markup(
                                    ls_types::MarkupContent {
                                        kind: ls_types::MarkupKind::Markdown,
                                        value,
                                    },
                                ),
                                range: origin_range
                                    .or_else(|| hover_result.as_ref().and_then(|h| h.range)),
                            });
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

    #[tracing::instrument(skip(self, params))]
    async fn code_action(
        &self,
        params: CodeActionParams,
    ) -> tower_lsp_server::jsonrpc::Result<Option<CodeActionResponse>> {
        let mut actions = vec![];

        let path = params
            .text_document
            .uri
            .to_file_path()
            .map(|p| p.to_string_lossy().to_string());

        if let Some(path) = path
            && let Some(file) = self.parsed_files.get(&path)
            && let ParsedFileType::AcsText(acs_text) = &file.parsed
            && let Some(kuid) = acs_text.find_kuid_at(params.range.start)
        {
            // Increment
            let mut inc_kuid = kuid.clone();
            inc_kuid.increment();
            let inc_title = format!(
                "Increment KUID version to {}",
                inc_kuid.version.unwrap_or(2)
            );
            actions.push(self.create_kuid_version_action(
                inc_title,
                acs_text,
                &kuid,
                inc_kuid,
                &params.text_document.uri,
            ));

            // Decrement
            if kuid.can_decrement() {
                let mut dec_kuid = kuid.clone();
                dec_kuid.decrement();
                let dec_title = format!(
                    "Decrement KUID version to {}",
                    dec_kuid.version.unwrap_or(0)
                );
                actions.push(self.create_kuid_version_action(
                    dec_title,
                    acs_text,
                    &kuid,
                    dec_kuid,
                    &params.text_document.uri,
                ));
            }
        }

        for diagnostic in &params.context.diagnostics {
            if let Some(ls_types::NumberOrString::String(code)) = &diagnostic.code
                && code == "invalid-kind-lib"
            {
                let mut changes = std::collections::HashMap::new();
                changes.insert(
                    params.text_document.uri.clone(),
                    vec![TextEdit {
                        range: diagnostic.range,
                        new_text: "\"library\"".to_string(),
                    }],
                );

                actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                    title: "Change \"lib\" to \"library\"".to_string(),
                    kind: Some(CodeActionKind::QUICKFIX),
                    diagnostics: Some(vec![diagnostic.clone()]),
                    edit: Some(WorkspaceEdit {
                        changes: Some(changes),
                        ..Default::default()
                    }),
                    is_preferred: Some(true),
                    ..Default::default()
                }));
            }
        }

        if actions.is_empty() {
            Ok(None)
        } else {
            Ok(Some(actions))
        }
    }

    #[tracing::instrument(skip(self, params))]
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
            .ok_or_else(Error::invalid_request);

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
                ParsedFileType::GameScript(program) => trainz_folding_range(program),
                ParsedFileType::AcsText(acs_text) => acs_text_folding_range(acs_text),
                ParsedFileType::AcsBinary(_) => vec![],
            };
            ranges.extend(trainz_folding::comments::comments_folding_range(&comments));
            ranges
        });

        let result = Some(ranges.clone());

        if let Some(progress) = progress {
            progress.finish().await;
        }

        Ok(result)
    }

    #[tracing::instrument(skip(self, params))]
    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> tower_lsp_server::jsonrpc::Result<Option<WorkspaceSymbolResponse>> {
        trace!("Workspace Symbol {:?}", params);

        let work_done_token = params.work_done_progress_params.work_done_token.clone();
        let progress = if let Some(token) = work_done_token {
            let progress = self
                .client
                .progress(token, "Finding workspace symbols")
                .begin()
                .await;
            Some(progress)
        } else {
            None
        };

        let query = params.query.to_lowercase();

        // Collect symbols from all parsed files
        let symbols: Vec<WorkspaceSymbol> = self
            .parsed_files
            .par_iter()
            .flat_map(|entry| {
                let path = entry.key();
                let document = entry.value();
                let mut local_symbols = Vec::new();

                if let Some(document_symbols) = document.document_symbols.get() {
                    for symbol in document_symbols {
                        // Recursively collect symbols that match the query
                        collect_matching_symbols(&mut local_symbols, symbol, &query, path);
                    }
                }
                local_symbols
            })
            .collect();

        if let Some(progress) = progress {
            progress.finish().await;
        }

        Ok(Some(WorkspaceSymbolResponse::Nested(symbols)))
    }
}

#[tracing::instrument(skip(symbols, symbol, query, file_path))]
fn collect_matching_symbols(
    symbols: &mut Vec<WorkspaceSymbol>,
    symbol: &ls_types::DocumentSymbol,
    query: &str,
    file_path: &str,
) {
    // Check if this symbol matches the query
    if query.is_empty() || symbol.name.to_lowercase().contains(query) {
        let uri = Uri::from_file_path(file_path).unwrap_or_else(|| file_path.parse().unwrap());

        symbols.push(WorkspaceSymbol {
            name: symbol.name.clone(),
            kind: symbol.kind,
            tags: symbol.tags.clone(),
            container_name: symbol.detail.clone(),
            location: OneOf::Left(Location {
                uri: uri.clone(),
                range: symbol.range,
            }),
            data: None,
        });
    }

    // Recursively check children
    if let Some(children) = &symbol.children {
        for child in children {
            collect_matching_symbols(symbols, child, query, file_path);
        }
    }
}

struct AcsTextScriptResolver<'a> {
    parsed_files: &'a DashMap<String, ParsedFile>,
}

impl<'a> ScriptResolver for AcsTextScriptResolver<'a> {
    #[tracing::instrument(skip(self, base_path, script_name))]
    fn resolve_script(
        &self,
        base_path: &Path,
        script_name: &str,
    ) -> Option<(Uri, Arc<trainz_ast::gs::Program>)> {
        let parent = base_path.parent().unwrap_or(base_path);
        let mut script_path = parent.join(script_name);
        if !script_path.exists() && !script_name.to_lowercase().ends_with(".gs") {
            script_path.set_extension("gs");
        }

        let path_str = script_path.to_string_lossy().to_string();
        if let Some(file) = self.parsed_files.get(&path_str)
            && let ParsedFileType::GameScript(program) = &file.value().parsed
        {
            return Some((Uri::from_file_path(script_path).unwrap(), program.clone()));
        }
        None
    }
}
