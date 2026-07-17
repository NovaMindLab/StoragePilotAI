use thiserror::Error;

/// Errors produced by the global scheduler.
#[derive(Debug, Error)]
pub enum SchedulerError {
    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Scheduler is shutdown")]
    Shutdown,

    #[error("Task cancelled")]
    Cancelled,

    #[error("Channel error: {0}")]
    Channel(String),
}

/// Convenience alias for scheduler results.
pub type SchedulerResult<T> = Result<T, SchedulerError>;
