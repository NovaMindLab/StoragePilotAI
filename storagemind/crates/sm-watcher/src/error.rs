use thiserror::Error;

#[derive(Debug, Error)]
pub enum WatchError {
    #[error("Notify error: {0}")]
    Notify(#[from] notify::Error),
    #[error("Channel error")]
    Channel,
}

pub type WatchResult<T> = Result<T, WatchError>;
