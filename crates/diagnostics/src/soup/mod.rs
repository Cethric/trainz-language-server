pub mod validator;

#[cfg(test)]
mod tests;
mod validate_container;
mod validate_simple_value;
mod validate_value;

pub use validator::soup_diagnostics;
