pub mod error;
pub mod reader;

pub use error::GslError;
pub use reader::GslReader;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GslLibrary {
    pub name: String,
    pub symbols: Vec<String>,
}

impl GslLibrary {
    /// Creates a new GslLibrary.
    pub fn new(name: String, symbols: Vec<String>) -> Self {
        Self { name, symbols }
    }
}
