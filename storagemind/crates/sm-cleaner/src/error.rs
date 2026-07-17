use thiserror::Error;

#[derive(Debug, Error)]
pub enum CleanError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Database error: {0}")]
    Database(String),
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
}

pub type CleanResult<T> = Result<T, CleanError>;
