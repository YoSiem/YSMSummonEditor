use ysm_core::evolution::ChainError;

#[derive(Debug, thiserror::Error)]
pub enum DataError {
    #[error("database is not configured")]
    NotConfigured,
    /// The connection could not be opened, or broke while in use.
    #[error("database connection problem: {0}")]
    Connection(String),
    /// The server did not answer in time; the connection was closed.
    #[error("the database did not answer in time: {0}")]
    Timeout(String),
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

impl DataError {
    /// Whether the failure means the connection is gone, as opposed to the
    /// server rejecting one request. After such a failure the outcome of a
    /// write in flight is unknown.
    pub fn is_connection_problem(&self) -> bool {
        matches!(self, DataError::Connection(_) | DataError::Timeout(_))
    }
}

pub type Result<T> = std::result::Result<T, DataError>;
