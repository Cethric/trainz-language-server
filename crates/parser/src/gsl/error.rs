use thiserror::Error;

#[derive(Debug, Error)]
pub enum GslError {
    #[error("Unexpected end of file at offset {0}")]
    UnexpectedEof(usize),

    #[error("Invalid magic number: expected 0xF762575D, found 0x{0:08X}")]
    InvalidMagic(u32),

    #[error("Invalid string at offset {0}")]
    InvalidString(usize),

    #[error("Internal error: {0}")]
    Internal(String),
}
