use ysm_core::evolution::ChainError;

#[derive(Debug, thiserror::Error)]
pub enum DataError {
    #[error("database is not configured")]
    NotConfigured,
    #[error("could not connect to the database: {0}")]
    Connection(String),
    #[error("database query failed: {0}")]
    Query(String),
    #[error("{0} not found")]
    NotFound(String),
    #[error("{0}")]
    Invalid(String),
    #[error(transparent)]
    Chain(#[from] ChainError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, DataError>;
