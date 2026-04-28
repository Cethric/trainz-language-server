/// Represents a validator for a rule node value.
///
/// # Examples
///
/// ```
/// # use trainz_acs_text_validators::validation_graph::validation::Validator;
/// # let validator = Validator::Unknown("test".to_string(), vec![]);
/// ```
#[derive(Debug, Clone)]
pub enum Validator {
    /// An unknown validator type.
    Unknown(String, Vec<(String, String)>),
}
