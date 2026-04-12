use crate::cache::AstCache;
use crate::gs::Program;
use std::fs;
use std::io::Cursor;
use std::path::Path;
use tracing::trace;

pub trait ProgramCache {
    fn load(&self, file_path: &Path) -> Option<Program>;
    fn save(&self, file_path: &Path, program: &Program) -> Result<(), Box<dyn std::error::Error>>;
}

impl ProgramCache for AstCache {
    #[tracing::instrument]
    fn load(&self, file_path: &Path) -> Option<Program> {
        let cache_path = self.get_cache_path(file_path);
        if !cache_path.exists() {
            return None;
        }

        let data = fs::read(&cache_path).ok()?;
        let program: Program = bson::deserialize_from_reader(Cursor::new(data)).ok()?;

        trace!("Loaded AST from cache for {:?}", file_path);
        Some(program)
    }

    #[tracing::instrument]
    fn save(&self, file_path: &Path, program: &Program) -> Result<(), Box<dyn std::error::Error>> {
        let cache_path = self.get_cache_path(file_path);

        let buffer = bson::serialize_to_vec(program)?;

        match fs::write(&cache_path, buffer) {
            Ok(_) => Ok(()),
            Err(err) => Err(err.into()),
        }
    }
}
