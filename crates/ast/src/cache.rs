use crate::gs::Program;
use log::{info, trace};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct AstCache {
    cache_dir: PathBuf,
}

impl AstCache {
    pub fn new() -> Self {
        let mut cache_dir = std::env::temp_dir();
        cache_dir.push("gs-lsp-cache");
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

    pub fn load(&self, file_path: &Path) -> Option<Program> {
        let cache_path = self.get_cache_path(file_path);
        if !cache_path.exists() {
            return None;
        }

        let data = fs::read(&cache_path).ok()?;
        let program: Program = bson::deserialize_from_reader(Cursor::new(data)).ok()?;

        trace!("Loaded AST from cache for {:?}", file_path);
        Some(program)
    }

    pub fn save(
        &self,
        file_path: &Path,
        program: &Program,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let cache_path = self.get_cache_path(file_path);

        let buffer = bson::serialize_to_vec(program)?;

        match fs::write(&cache_path, buffer) {
            Ok(_) => Ok(()),
            Err(err) => Err(err.into()),
        }
    }

    pub fn bust(&self, file_path: &Path) {
        let cache_path = self.get_cache_path(file_path);
        if cache_path.exists() {
            let _ = fs::remove_file(&cache_path);
            trace!("Busted AST cache for {:?}", file_path);
        }
    }
}
