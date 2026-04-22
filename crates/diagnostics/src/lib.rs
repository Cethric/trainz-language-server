//! # Trainz Diagnostics
//!
//! This crate provides diagnostic checks for the Trainz language, including:
//! - Game Script (GS) validation
//! - AcsText validation
//!
//! The crate exposes functionality for the Language Server to report errors and warnings
//! based on the AST and project context.

pub mod acs_text;
pub mod gs;
mod sources;

use shadow_rs::shadow;

shadow!(build);
