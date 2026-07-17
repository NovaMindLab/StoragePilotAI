use thiserror::Error;

#[derive(Debug, Error)]
pub enum AiError {
    #[error("ONNX Runtime error: {0}")]
    Ort(#[from] ort::Error),
    #[error("Tokenizer error: {0}")]
    Tokenizer(String),
    #[error("Image processing error: {0}")]
    Image(#[from] image::ImageError),
    #[error("Shape error: {0}")]
    Shape(#[from] ndarray::ShapeError),
    #[error("Database error: {0}")]
    Database(String),
    #[error("Download error: {0}")]
    Download(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Engine not initialized")]
    NotInitialized,
}

pub type AiResult<T> = Result<T, AiError>;
