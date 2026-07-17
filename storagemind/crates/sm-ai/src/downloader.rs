use std::path::{Path, PathBuf};
use tracing::info;
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;
use futures_util::StreamExt;

use crate::error::{AiError, AiResult};

// URLs for the ONNX models and tokenizer
// Using a placeholder URL, in a real app these would be actual CDN links
// For MobileCLIP, these are usually exported from the PyTorch model
const TEXT_ENCODER_URL: &str = "https://huggingface.co/Xenova/clip-vit-base-patch32/resolve/main/onnx/text_model.onnx";
const IMAGE_ENCODER_URL: &str = "https://huggingface.co/Xenova/clip-vit-base-patch32/resolve/main/onnx/vision_model.onnx";
const TOKENIZER_URL: &str = "https://huggingface.co/Xenova/clip-vit-base-patch32/resolve/main/tokenizer.json";

pub async fn ensure_models_exist(model_dir: &Path) -> AiResult<()> {
    if !model_dir.exists() {
        fs::create_dir_all(model_dir).await?;
    }

    let files = vec![
        ("text_encoder.onnx", TEXT_ENCODER_URL),
        ("image_encoder.onnx", IMAGE_ENCODER_URL),
        ("tokenizer.json", TOKENIZER_URL),
    ];

    for (filename, url) in files {
        let file_path = model_dir.join(filename);
        if !file_path.exists() {
            download_file(url, &file_path).await?;
        }
    }

    Ok(())
}

async fn download_file(url: &str, dest: &Path) -> AiResult<()> {
    info!("Downloading {} to {:?}", url, dest);
    
    // In a real app, this should report progress, but we keep it simple for now
    let response = reqwest::get(url)
        .await
        .map_err(|e| AiError::Download(e.to_string()))?;

    if !response.status().is_success() {
        return Err(AiError::Download(format!("Failed to download {}: HTTP {}", url, response.status())));
    }

    let mut file = File::create(dest).await?;
    let mut stream = response.bytes_stream();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| AiError::Download(e.to_string()))?;
        file.write_all(&chunk).await?;
    }

    info!("Downloaded {:?}", dest);
    Ok(())
}
