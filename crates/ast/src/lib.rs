//! AST definitions for GS and AcsText languages used in Trainz.

use shadow_rs::shadow;
pub use tower_lsp_server::ls_types::{Position, Range};

pub mod acs_text;
pub mod cache;
pub mod comments;
pub mod find;
pub mod gs;
pub mod gsl;

shadow!(build);
