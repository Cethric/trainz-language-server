use crate::process::acs_binary::ProcessAcsBinary;
use crate::process::acs_text::ProcessAcsText;
use crate::process::gs::ProcessGS;
use crate::state::{ParsedFileType, Project, RecursiveIncludeResolver, TrainzLanguageServer};
use dashmap::DashSet;
use futures::StreamExt;
use rayon::prelude::*;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tower_lsp_server::ls_types::{MessageType, ProgressToken};
use tracing::{info, trace};
use walkdir::WalkDir;

impl TrainzLanguageServer {
    pub async fn discover_projects(&self) {
        let workspace_folders = self.workspace_folders();

        self.client
            .log_message(
                MessageType::INFO,
                format!(
                    "Discovering projects in workspace folders: {:?}",
                    workspace_folders
                ),
            )
            .await;

        info!(
            "Discovering projects in workspace folders: {:?}",
            workspace_folders
        );

        let mut all_project_roots = std::collections::HashSet::new();

        for folder in workspace_folders {
            for entry in WalkDir::new(&folder)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_name() == "config.txt" {
                    let config_path = entry.path().to_path_buf();
                    let project_root = config_path.parent().unwrap().to_path_buf();
                    all_project_roots.insert(project_root);
                }
            }
        }

        futures::stream::iter(all_project_roots)
            .for_each_concurrent(None, |project_root| async move {
                if !self.projects.contains_key(&project_root) {
                    info!("Found new project at {:?}", project_root);
                    let config_txt = project_root.join("config.txt");
                    let project = Arc::new(Project {
                        root: project_root.clone(),
                        config_txt,
                        script_files: DashSet::new(),
                        chump_files: DashSet::new(),
                        assets: DashSet::new(),
                    });

                    // Scan for files in the project root, but stop at other projects
                    for entry in WalkDir::new(&project_root)
                        .follow_links(true)
                        .into_iter()
                        .filter_entry(|e| {
                            // Don't recurse into other projects
                            if e.file_name() == "config.txt"
                                && e.path().parent().unwrap() != project_root
                            {
                                return false;
                            }
                            true
                        })
                        .filter_map(|e| e.ok())
                    {
                        let path = entry.path().to_path_buf();
                        if path.is_file() {
                            if path.file_name().is_some_and(|n| n == "config.txt") {
                                continue;
                            }

                            if path.extension().is_some_and(|ext| ext == "gs") {
                                trace!(
                                    "Found script file {:?} for project {:?}",
                                    path, project_root
                                );
                                project.script_files.insert(path);
                            } else if path.extension().is_some_and(|ext| ext == "chp") {
                                trace!(
                                    "Found chump file {:?} for project {:?}",
                                    path, project_root
                                );
                                project.chump_files.insert(path);
                            } else {
                                trace!(
                                    "Found asset file {:?} for project {:?}",
                                    path, project_root
                                );
                                project.assets.insert(path);
                            }
                        }
                    }

                    self.projects.insert(project_root.clone(), project.clone());
                    self.index_project(project).await;
                }
            })
            .await;
    }

    pub async fn index_project(&self, project: Arc<Project>) {
        info!("Indexing project at {:?}", project.root);

        self.client
            .log_message(
                MessageType::INFO,
                format!("Indexing project at {:?}", project.root),
            )
            .await;

        let progress = self
            .client
            .progress(
                ProgressToken::String(project.root.to_string_lossy().to_string()),
                format!("Indexing project at {:?}", project.root),
            )
            .with_percentage(0)
            .with_message(format!("Indexing project: {:?}", project.root.file_name()))
            .begin()
            .await;

        if let Ok(content) = fs::read_to_string(&project.config_txt) {
            self.process_acs_text_file(&project.config_txt, &content, false, &progress)
                .await;
        }

        let workspace_folders = self.workspace_folders();
        let script_files: Vec<_> = project
            .script_files
            .iter()
            .map(|s| s.key().clone())
            .collect();
        futures::stream::iter(script_files)
            .for_each_concurrent(None, |script_path| {
                let workspace_folders = &workspace_folders;
                let progress = &progress;
                async move {
                    if let Ok(content) = fs::read_to_string(&script_path) {
                        self.process_gs_file(
                            &script_path,
                            &content,
                            workspace_folders,
                            false,
                            progress,
                        )
                        .await;
                    }
                }
            })
            .await;

        let chump_files: Vec<_> = project
            .chump_files
            .iter()
            .map(|s| s.key().clone())
            .collect();
        futures::stream::iter(chump_files)
            .for_each_concurrent(None, |chump_path| {
                let progress = &progress;
                async move {
                    if let Ok(content) = fs::read(&chump_path) {
                        self.process_acs_binary_file(&chump_path, &content, false, progress)
                            .await;
                    }
                }
            })
            .await;

        // Compute symbols for all project files to enable workspace search
        self.parsed_files
            .iter()
            .par_bridge()
            .for_each(|file_entry| {
                let path_str = file_entry.key();
                let path = Path::new(path_str);
                if path.starts_with(&project.root) {
                    let parsed_file = file_entry.value();
                    let parsed_file_type = parsed_file.parsed.clone();
                    let document_symbols_lock = parsed_file.document_symbols.clone();

                    document_symbols_lock.get_or_init(|| match &parsed_file_type {
                        ParsedFileType::GameScript(program) => {
                            let resolver = RecursiveIncludeResolver {
                                current_program: program,
                                parsed_files: &self.parsed_files,
                            };
                            trainz_symboliser::gs::trainz_symboliser(program, &resolver)
                        }
                        ParsedFileType::AcsText(acs_text) => {
                            trainz_symboliser::acs_text::acs_text_symboliser(acs_text)
                        }
                        ParsedFileType::AcsBinary(_) => vec![],
                    });
                }
            });

        progress.finish().await;
    }

    pub fn find_project_for_file(&self, path: &Path) -> Option<Arc<Project>> {
        let mut best_match: Option<(usize, Arc<Project>)> = None;
        for project in self.projects.iter() {
            let root = project.key();
            if path.starts_with(root) {
                let len = root.as_os_str().len();
                if best_match
                    .as_ref()
                    .is_none_or(|(best_len, _)| len > *best_len)
                {
                    best_match = Some((len, project.value().clone()));
                }
            }
        }
        best_match.map(|(_, p)| p)
    }

    pub fn add_file_to_project(&self, path: &Path) {
        if let Some(project) = self.find_project_for_file(path) {
            if path.extension().is_some_and(|ext| ext == "gs") {
                project.script_files.insert(path.to_path_buf());
            } else if path.extension().is_some_and(|ext| ext == "chp") {
                project.chump_files.insert(path.to_path_buf());
            } else if path.file_name().is_some_and(|n| n != "config.txt") {
                project.assets.insert(path.to_path_buf());
            }
        } else {
            // No project found, check if it's a config.txt itself?
            if path.file_name().is_some_and(|n| n == "config.txt") {
                let project_root = path.parent().unwrap().to_path_buf();
                if !self.projects.contains_key(&project_root) {
                    info!("Found new project via file open at {:?}", project_root);
                    let project = Arc::new(Project {
                        root: project_root.clone(),
                        config_txt: path.to_path_buf(),
                        script_files: DashSet::new(),
                        chump_files: DashSet::new(),
                        assets: DashSet::new(),
                    });
                    self.projects.insert(project_root, project);
                }
            } else {
                // Should we search upwards for config.txt?
                let mut current = path.parent();
                while let Some(dir) = current {
                    let config_txt = dir.join("config.txt");
                    if config_txt.exists() {
                        let project_root = dir.to_path_buf();
                        if !self.projects.contains_key(&project_root) {
                            info!("Found new project via upward search at {:?}", project_root);
                            let project = Arc::new(Project {
                                root: project_root.clone(),
                                config_txt: config_txt.clone(),
                                script_files: DashSet::new(),
                                chump_files: DashSet::new(),
                                assets: DashSet::new(),
                            });
                            if path.extension().is_some_and(|ext| ext == "gs") {
                                project.script_files.insert(path.to_path_buf());
                            } else if path.extension().is_some_and(|ext| ext == "chp") {
                                project.chump_files.insert(path.to_path_buf());
                            } else {
                                project.assets.insert(path.to_path_buf());
                            }
                            self.projects.insert(project_root, project);
                        } else {
                            let project = self.projects.get(&project_root).unwrap();
                            if path.extension().is_some_and(|ext| ext == "gs") {
                                project.script_files.insert(path.to_path_buf());
                            } else if path.extension().is_some_and(|ext| ext == "chp") {
                                project.chump_files.insert(path.to_path_buf());
                            } else {
                                project.assets.insert(path.to_path_buf());
                            }
                        }
                        break;
                    }
                    current = dir.parent();
                }
            }
        }
    }

    pub fn remove_file_from_project(&self, path: &Path) {
        if let Some(project) = self.find_project_for_file(path) {
            project.script_files.remove(path);
            project.chump_files.remove(path);
            project.assets.remove(path);

            // If the deleted file is config.txt, the project is gone
            if path.file_name().is_some_and(|n| n == "config.txt") {
                let project_root = path.parent().unwrap();
                self.projects.remove(project_root);
                info!(
                    "Project at {:?} removed because config.txt was deleted",
                    project_root
                );
            }
        }
    }
}
