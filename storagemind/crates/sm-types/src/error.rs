//! Unified error type for the StorageMind platform.

use thiserror::Error;

/// Top-level error enum covering every subsystem that can fail.
///
/// All internal crates should convert their local errors into a `StorageError`
/// variant at the boundary so that callers always receive a single error type.
#[derive(Debug, Error)]
pub enum StorageError {
    /// Wraps [`std::io::Error`] transparently.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Errors from the embedded or remote database layer.
    #[error("Database error: {0}")]
    Database(String),

    /// Platform-specific (macOS / Windows / Linux) errors.
    #[error("Platform error: {0}")]
    Platform(String),

    /// Errors produced by the three-stage file scanner.
    #[error("Scanner error: {0}")]
    Scanner(String),

    /// Errors from the unified Storage Index.
    #[error("Index error: {0}")]
    Index(String),

    /// Errors from the search subsystem.
    #[error("Search error: {0}")]
    Search(String),

    /// Errors from AI inference or embedding pipeline.
    #[error("AI error: {0}")]
    Ai(String),

    /// Configuration loading / validation errors.
    #[error("Config error: {0}")]
    Config(String),

    /// Plugin loading or ABI errors.
    #[error("Plugin error: {0}")]
    Plugin(String),

    /// Catch-all for errors that do not fit any category above.
    #[error("{0}")]
    Other(String),
}

/// Convenience alias used throughout the workspace.
pub type StorageResult<T> = Result<T, StorageError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_variants() {
        let e = StorageError::Database("conn refused".into());
        assert_eq!(e.to_string(), "Database error: conn refused");

        let e = StorageError::Other("something went wrong".into());
        assert_eq!(e.to_string(), "something went wrong");
    }

    #[test]
    fn from_io_error() {
        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let e: StorageError = io.into();
        assert!(e.to_string().starts_with("IO error:"));
    }
}
