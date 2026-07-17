use std::path::Path;
use ort::session::{Session, builder::GraphOptimizationLevel};
use tracing::{info, debug};

use crate::error::{AiError, AiResult};
use crate::downloader;

use std::sync::Mutex;

/// The MobileCLIP Engine managing ONNX sessions for text and image encoders.
pub struct MobileClipEngine {
    text_session: Mutex<Session>,
    image_session: Mutex<Session>,
    tokenizer: tokenizers::Tokenizer,
}

impl MobileClipEngine {
    /// Initialize the AI Engine, downloading models if they don't exist.
    pub async fn new(model_dir: &Path) -> AiResult<Self> {
        info!("Initializing MobileCLIP Engine at {:?}", model_dir);
        
        // Ensure models exist
        downloader::ensure_models_exist(model_dir).await?;

        // Setup ONNX Runtime (ort 2.x handles initialization automatically mostly,
        // but we can just create sessions directly).
        
        let text_model_path = model_dir.join("text_encoder.onnx");
        let image_model_path = model_dir.join("image_encoder.onnx");
        let tokenizer_path = model_dir.join("tokenizer.json");

        debug!("Loading Text Encoder from {:?}", text_model_path);
        let text_session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| AiError::Ort(e.into()))?
            .with_intra_threads(2)
            .map_err(|e| AiError::Ort(e.into()))?
            .commit_from_file(&text_model_path)?;

        debug!("Loading Image Encoder from {:?}", image_model_path);
        let image_session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| AiError::Ort(e.into()))?
            .with_intra_threads(2)
            .map_err(|e| AiError::Ort(e.into()))?
            .commit_from_file(&image_model_path)?;

        debug!("Loading Tokenizer from {:?}", tokenizer_path);
        let tokenizer = tokenizers::Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| AiError::Tokenizer(e.to_string()))?;

        info!("MobileCLIP Engine initialized successfully.");

        Ok(Self {
            text_session: Mutex::new(text_session),
            image_session: Mutex::new(image_session),
            tokenizer,
        })
    }

    pub fn text_session(&self) -> &Mutex<Session> {
        &self.text_session
    }

    pub fn image_session(&self) -> &Mutex<Session> {
        &self.image_session
    }

    pub fn tokenizer(&self) -> &tokenizers::Tokenizer {
        &self.tokenizer
    }
}
