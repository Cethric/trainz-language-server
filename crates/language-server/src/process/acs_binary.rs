use crate::process::guard::ProcessingGuard;
use crate::state::{ParsedFile, ParsedFileType, TrainzLanguageServer};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::sync::{Arc, OnceLock};
use tower_lsp_server::{Bounded, NotCancellable, OngoingProgress};
use tracing::{error, trace};
use trainz_tdx::{TdxReader, TdxValue};

pub trait ProcessAcsBinary {
    fn process_acs_binary_file(
        &self,
        path: &Path,
        content: &[u8],
        changed: bool,
        progress: &OngoingProgress<Bounded, NotCancellable>,
    ) -> impl Future<Output = ()> + Send;
}

impl ProcessAcsBinary for TrainzLanguageServer {
    #[tracing::instrument(skip(self, content, progress))]
    async fn process_acs_binary_file(
        &self,
        path: &Path,
        content: &[u8],
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

        self.process_acs_binary_file_inner(path, content, changed, progress)
            .await;
    }
}

impl TrainzLanguageServer {
    #[tracing::instrument(skip(self, content, progress))]
    async fn process_acs_binary_file_inner(
        &self,
        path: &Path,
        content: &[u8],
        changed: bool,
        progress: &OngoingProgress<Bounded, NotCancellable>,
    ) {
        if let Some(path_str) = path.to_str() {
            if self.parsed_files.contains_key(path_str) && !changed {
                trace!("File already processed {:?}", path);
                return;
            }
            trace!("Processing binary file {:?}", path);

            let mut reader = TdxReader::new(content);
            match reader.parse_all() {
                Ok(results) => {
                    trace!("File parsed {:?}", path);

                    // Capture asset dependencies
                    let kuids = TdxValue::find_kuids(&results);
                    if let Some(tdx_cache) = &self.tdx_cache_path {
                        let path_hash = {
                            let mut hasher = Sha256::new();
                            hasher.update(path_str.as_bytes());
                            hex::encode(hasher.finalize())
                        };

                        let cache_file = tdx_cache.join(format!("{}.json", path_hash));
                        if let Ok(json) = serde_json::to_string(&kuids) {
                            let _ = std::fs::create_dir_all(tdx_cache);
                            let _ = std::fs::write(cache_file, json);
                        }
                    }

                    self.parsed_files.insert(
                        path_str.to_string(),
                        ParsedFile {
                            parsed: ParsedFileType::AcsBinary(Arc::new(results)),
                            comments: Arc::new(trainz_ast::comments::CommentProgram {
                                comments: vec![],
                                range: Default::default(),
                            }),
                            semantic_tokens: Arc::new(OnceLock::new()),
                            document_symbols: Arc::new(OnceLock::new()),
                            diagnostics: Arc::new(OnceLock::new()),
                            folding_ranges: Arc::new(OnceLock::new()),
                        },
                    );
                }
                Err(e) => {
                    error!("Failed to parse binary file: {:?}", e);
                }
            }
        } else {
            unreachable!("path is not a string");
        }

        progress.report_with_message("Processed file", 100).await;
    }
}
