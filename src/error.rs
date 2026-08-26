use thiserror::Error;

#[derive(Debug, Error)]
pub enum SimError {
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    #[error("entity not found: {0}")]
    NotFound(String),
    #[error("dimension limit exceeded: {actual} > {max}")]
    DimensionLimit { actual: usize, max: usize },
    #[error("non-finite numeric value")]
    NonFinite,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, SimError>;
