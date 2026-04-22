#[derive(Debug, Clone)]
pub enum Validator {
    Unknown(String, Vec<(String, String)>),
}
