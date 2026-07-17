use std::path::Path;
use ndarray::Array4;
use ort::{inputs, session::Session};
use image::{imageops::FilterType, GenericImageView, ImageReader};

use crate::error::AiResult;

use std::sync::Mutex;

const TARGET_SIZE: u32 = 224;

/// Generate an embedding from an image using the vision encoder session.
pub fn encode_image(
    session_mutex: &Mutex<Session>,
    image_path: &Path,
) -> AiResult<Vec<f32>> {
    // 1. Load and preprocess image
    let img = ImageReader::open(image_path)?.decode()?;
    
    // Resize and center crop
    let (width, height) = img.dimensions();
    let resize_ratio = (TARGET_SIZE as f32) / (width.min(height) as f32);
    let new_width = (width as f32 * resize_ratio).round() as u32;
    let new_height = (height as f32 * resize_ratio).round() as u32;

    let resized = img.resize_exact(new_width, new_height, FilterType::Triangle);
    let cropped = resized.crop_imm(
        (new_width - TARGET_SIZE) / 2,
        (new_height - TARGET_SIZE) / 2,
        TARGET_SIZE,
        TARGET_SIZE,
    ).to_rgb8();

    // 2. Normalize and convert to NCHW format
    // Standard CLIP normalization: mean=[0.48145466, 0.4578275, 0.40821073], std=[0.26862954, 0.26130258, 0.27577711]
    let mean = [0.48145466, 0.4578275, 0.40821073];
    let std = [0.26862954, 0.26130258, 0.27577711];

    let mut tensor = Array4::<f32>::zeros((1, 3, TARGET_SIZE as usize, TARGET_SIZE as usize));
    
    for (x, y, pixel) in cropped.enumerate_pixels() {
        for c in 0..3 {
            let val = (pixel[c] as f32 / 255.0 - mean[c]) / std[c];
            tensor[[0, c, y as usize, x as usize]] = val;
        }
    }
    use ort::value::Tensor;
    let input_tensor = Tensor::from_array(tensor)?;

    let mut session = session_mutex.lock().unwrap();
    // 3. Run inference
    let outputs = session.run(inputs![
        "pixel_values" => input_tensor
    ])?;

    // 4. Extract output
    let output_tensor = outputs[0].try_extract_tensor::<f32>()?;
    let embedding: Vec<f32> = output_tensor.1.iter().copied().collect();

    // Normalize the embedding (L2 normalization)
    let embedding = normalize(&embedding);

    Ok(embedding)
}

fn normalize(v: &[f32]) -> Vec<f32> {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        v.iter().map(|x| x / norm).collect()
    } else {
        v.to_vec()
    }
}
