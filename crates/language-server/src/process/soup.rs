use crate::process::guard::ProcessingGuard;
use crate::state::{GameScriptLanguageServer, ParsedFile, ParsedFileType};
use std::path::Path;
use std::sync::{Arc, OnceLock};
use tower_lsp_server::{Bounded, NotCancellable, OngoingProgress};
use tracing::{error, trace};
use trainz_ast::soup::process::process_soup_ast;
use trainz_parser::soup::parse_soup;

pub trait ProcessSoup {
    fn process_soup_file(
        &self,
        path: &Path,
        content: &str,
        changed: bool,
        progress: &OngoingProgress<Bounded, NotCancellable>,
    ) -> impl Future<Output = ()> + Send;
}

impl ProcessSoup for GameScriptLanguageServer {
    #[tracing::instrument(skip(self, content, progress))]
    async fn process_soup_file(
        &self,
        path: &Path,
        content: &str,
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

        self.process_soup_file_inner(path, content, changed, progress)
            .await;
    }
}

impl GameScriptLanguageServer {
    #[tracing::instrument(skip(self, content, progress))]
    async fn process_soup_file_inner(
        &self,
        path: &Path,
        content: &str,
        changed: bool,
        progress: &OngoingProgress<Bounded, NotCancellable>,
    ) {
        if let Some(path_str) = path.to_str() {
            if self.parsed_files.contains_key(path_str) && !changed {
                trace!("File already processed {:?}", path);
                return;
            }
            trace!("Processing file {:?}", path);

            let pairs = parse_soup(content);
            if let Ok(pairs) = pairs {
                trace!("File parsed {:?}", path);

                let parsed = process_soup_ast(pairs, content);

                let parsed_arc = Arc::new(parsed);

                let comments_arc = match trainz_parser::comments::parse_soup_comments(content) {
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
