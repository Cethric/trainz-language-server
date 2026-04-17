use thiserror::Error;

#[derive(Debug, Error)]
pub enum TdxError {
    #[error("Unexpected EOF at offset {0}")]
    UnexpectedEof(usize),
    #[error("Utf8 error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid header: expected ACS$, found {0:?}")]
    InvalidHeader([u8; 4]),
    #[error("Invalid tag: {0}")]
    InvalidTag(String),
}
