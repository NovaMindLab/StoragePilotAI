use thiserror::Error;

#[derive(Debug, Error)]
pub enum IndexError {
    #[error("Database error: {0}")]
    Database(String),
    #[error("File not found: {0}")]
    NotFound(String),
}

pub type IndexResult<T> = Result<T, IndexError>;
