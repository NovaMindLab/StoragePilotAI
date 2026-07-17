use thiserror::Error;

/// Errors that can occur in platform-specific operations.
#[derive(Debug, Error)]
pub enum PlatformError {
    /// Wraps standard I/O errors from filesystem or process calls.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// An error returned from a platform API (e.g., shell command failure).
    #[error("Platform API error: {0}")]
    Api(String),

    /// The requested operation is not supported on this platform.
    #[error("Unsupported platform")]
    Unsupported,

    /// The caller lacks the required permissions for the operation.
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
}

/// Convenience alias used throughout `sm-platform`.
pub type PlatformResult<T> = Result<T, PlatformError>;
