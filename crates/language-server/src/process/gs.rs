use crate::process::guard::ProcessingGuard;
use crate::state::{GameScriptLanguageServer, ParsedFile, ParsedFileType};
use async_recursion::async_recursion;
use rayon::iter::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use tower_lsp_server::{Bounded, NotCancellable, OngoingProgress};
use tracing::{debug, error, info, trace};
use trainz_ast::cache::ProgramCache;
use trainz_ast::gs::Include;
use trainz_ast::gs::process::process_trainz_ast;
use trainz_parser::gs::parse;

pub trait ProcessGS {
    fn process_gs_file(
        &self,
        path: &Path,
        content: &str,
        workspace_folders: &[PathBuf],
        changed: bool,
        progress: &OngoingProgress<Bounded, NotCancellable>,
    ) -> impl Future<Output = ()> + Send;
}

impl ProcessGS for GameScriptLanguageServer {
    #[tracing::instrument(skip(self, content, workspace_folders, progress))]
    async fn process_gs_file(
        &self,
        path: &Path,
        content: &str,
        workspace_folders: &[PathBuf],
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
    workspace_folders: &[PathBuf],
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
    #[tracing::instrument(skip(self, workspace_folders, progress))]
    async fn process_gs_include(
        &self,
        include: Include,
        workspace_folders: &[PathBuf],
        progress: &OngoingProgress<Bounded, NotCancellable>,
    ) {
        if let Some(path) = include.path {
            let path_str = path.to_string_lossy().to_string();
            self.increment_count(&path_str);
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

                    let program_arc = Arc::new(program);
                    self.parsed_files.insert(
                        path_str.clone(),
                        ParsedFile {
                            parsed: ParsedFileType::GameScript(program_arc.clone()),
                            comments,
                            semantic_tokens: Arc::new(OnceLock::new()),
                            document_symbols: Arc::new(OnceLock::new()),
                            diagnostics: Arc::new(OnceLock::new()),
                            folding_ranges: Arc::new(OnceLock::new()),
                        },
                    );

                    // Process includes recursively for cached file
                    for include in &program_arc.includes {
                        self.process_gs_include(include.clone(), workspace_folders, progress)
                            .await;
                    }

                    return;
                }
            }

            debug!("Processing include file {:?}", path);

            let document = fs::read(path.clone());
            if let Ok(document) = document {
                let source = String::from_utf8_lossy(&document);

                self.process_gs_file(path.as_path(), &source, workspace_folders, false, progress)
                    .await;
            }
        }
    }

    #[async_recursion]
    #[tracing::instrument(skip(self, content, workspace_folders, progress))]
    async fn process_gs_file_inner(
        &self,
        path: &Path,
        content: &str,
        workspace_folders: &[PathBuf],
        changed: bool,
        progress: &OngoingProgress<Bounded, NotCancellable>,
    ) {
        let mut includes: Vec<Include> = vec![];
        let mut old_include_paths = std::collections::HashSet::new();

        if let Some(path_str) = path.to_str() {
            let base_path = path.parent().unwrap();

            if self.parsed_files.contains_key(path_str) && !changed {
                trace!("File already processed {:?}", path);
                return;
            }
            trace!("Processing file {:?}", path);

            // Get old includes to decrement counts if they changed
            if let Some(file) = self.parsed_files.get(path_str)
                && let ParsedFileType::GameScript(program) = &file.parsed
            {
                for include in &program.includes {
                    if let Some(p) = &include.path {
                        old_include_paths.insert(p.to_string_lossy().to_string());
                    }
                }
            }

            let pairs = parse(content);
            if let Ok(pairs) = pairs {
                trace!("File parsed {:?}", path);

                let mut parsed = process_trainz_ast(pairs, content);

                // Resolve include paths
                for include in &mut parsed.includes {
                    include.path = find_include_path(
                        &include.name,
                        base_path,
                        workspace_folders,
                        &self.search_paths,
                    );
                }

                // Save to cache after include resolution
                let _ = self.ast_cache.save(path, &parsed);

                // Decrement count for removed includes
                let mut new_include_paths = std::collections::HashSet::new();
                for include in &parsed.includes {
                    if let Some(p) = &include.path {
                        new_include_paths.insert(p.to_string_lossy().to_string());
                    }
                }

                for old_path in &old_include_paths {
                    if !new_include_paths.contains(old_path) {
                        self.decrement_count(old_path);
                    }
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
                        parsed: ParsedFileType::GameScript(parsed_arc.clone()),
                        comments: comments_arc,
                        semantic_tokens: Arc::new(OnceLock::new()),
                        document_symbols: Arc::new(OnceLock::new()),
                        diagnostics: Arc::new(OnceLock::new()),
                        folding_ranges: Arc::new(OnceLock::new()),
                    },
                );

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

            if let Some(include_path) = &include.path {
                info!("File {:?} includes {:?}", path, include_path);

                // Only process includes that are new for this file to avoid double-counting
                // and unnecessary recursion
                if old_include_paths.contains(&include_path.to_string_lossy().to_string()) {
                    continue;
                }
            }

            self.process_gs_include(include, workspace_folders, progress)
                .await;
        }
    }
}
