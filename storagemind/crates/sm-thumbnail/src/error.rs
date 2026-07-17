use thiserror::Error;

#[derive(Debug, Error)]
pub enum ThumbnailError {
    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Database error: {0}")]
    Database(String),
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
}

pub type ThumbnailResult<T> = Result<T, ThumbnailError>;
