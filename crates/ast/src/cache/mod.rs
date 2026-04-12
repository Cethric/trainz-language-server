use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, trace};

pub mod program;

pub use program::ProgramCache;

#[derive(Debug)]
pub struct AstCache {
    cache_dir: PathBuf,
}

impl Default for AstCache {
    fn default() -> Self {
        Self::new()
    }
}

impl AstCache {
    pub fn new() -> Self {
        let mut cache_dir = std::env::temp_dir();
        cache_dir.push("language-server-cache");
        if !cache_dir.exists() {
            let _ = fs::create_dir_all(&cache_dir);
        }
        info!("Cache Directory: {:?}", cache_dir);
        Self { cache_dir }
    }

    fn get_cache_path(&self, file_path: &Path) -> PathBuf {
        let mut hasher = Sha256::new();
        hasher.update(file_path.to_string_lossy().as_bytes());
        let hash = hex::encode(hasher.finalize());
        let mut path = self.cache_dir.clone();
        path.push(format!("{}.bson", hash));
        path
    }

    pub fn bust(&self, file_path: &Path) {
        let cache_path = self.get_cache_path(file_path);
        if cache_path.exists() {
            let _ = fs::remove_file(&cache_path);
            trace!("Busted AST cache for {:?}", file_path);
        }
    }
}
