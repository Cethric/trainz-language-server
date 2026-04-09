use crate::process::guard::ProcessingGuard;
use crate::state::{GameScriptLanguageServer, ParsedFile, ParsedFileType};
use async_recursion::async_recursion;
use log::{error, trace};
use rayon::iter::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use tower_lsp_server::{Bounded, NotCancellable, OngoingProgress};
use trainz_ast::cache::ProgramCache;
use trainz_ast::gs::process::process_trainz_ast;
use trainz_ast::gs::Include;
use trainz_parser::gs::parse;

pub trait ProcessGS {
    fn process_gs_file(
        &self,
        path: &Path,
        content: &str,
        workspace_folders: &Vec<PathBuf>,
        changed: bool,
        progress: &OngoingProgress<Bounded, NotCancellable>,
    ) -> impl Future<Output = ()> + Send;
}

impl ProcessGS for GameScriptLanguageServer {
    async fn process_gs_file(
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
}

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
                        if let Ok(pairs) = trainz_parser::comments::parse_gs_comments(&source) {
                            Arc::new(trainz_ast::comments::process::process_comments(
                                pairs.into_iter().next().unwrap(),
                                &source,
                            ))
                        } else {
                            Arc::new(trainz_ast::comments::CommentProgram {
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

                let mut parsed = process_trainz_ast(pairs, content);

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

                let comments_arc = match trainz_parser::comments::parse_gs_comments(content) {
                    Ok(mut pairs) => {
                        if let Some(pair) = pairs.next() {
                            Arc::new(trainz_ast::comments::process::process_comments(
                                pair, content,
                            ))
                        } else {
                            Arc::new(trainz_ast::comments::CommentProgram {
                                comments: vec![],
                                range: Default::default(),
                            })
                        }
                    }
                    Err(_) => Arc::new(trainz_ast::comments::CommentProgram {
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
}
