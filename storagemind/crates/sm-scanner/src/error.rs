//! Error types for the sm-scanner crate.

use thiserror::Error;

/// All errors that can occur during a scan operation.
#[derive(Debug, Error)]
pub enum ScanError {
    /// An IO error occurred while reading a path.
    #[error("IO error on {path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
    /// A database write failed.
    #[error("Database error: {0}")]
    Database(String),
    /// The scan was externally cancelled.
    #[error("Scan was cancelled")]
    Cancelled,
    /// The event channel is closed/disconnected.
    #[error("Channel send error")]
    Channel,
}

/// Convenience alias for `Result<T, ScanError>`.
pub type ScanResult<T> = Result<T, ScanError>;
