use ndarray::Array2;
use ort::{inputs, session::Session};

use crate::error::{AiError, AiResult};

use std::sync::Mutex;

/// Generate an embedding from text using the text encoder session and tokenizer.
pub fn encode_text(
    session_mutex: &Mutex<Session>,
    tokenizer: &tokenizers::Tokenizer,
    query: &str,
) -> AiResult<Vec<f32>> {
    // 1. Tokenize the text
    let encoding = tokenizer.encode(query, true)
        .map_err(|e| AiError::Tokenizer(e.to_string()))?;

    let ids = encoding.get_ids();
    let attention_mask = encoding.get_attention_mask();

    // 2. Prepare tensors
    // The shape is usually [batch_size, sequence_length], so [1, seq_len]
    let seq_len = ids.len();
    
    // Convert to i64 as required by typical ONNX models for input_ids
    let input_ids: Vec<i64> = ids.iter().map(|&x| x as i64).collect();
    let mask: Vec<i64> = attention_mask.iter().map(|&x| x as i64).collect();

    let input_ids_array = Array2::from_shape_vec((1, seq_len), input_ids)?;
    let attention_mask_array = Array2::from_shape_vec((1, seq_len), mask)?;

    use ort::value::Tensor;
    let input_ids_tensor = Tensor::from_array(input_ids_array)?;
    let attention_mask_tensor = Tensor::from_array(attention_mask_array)?;

    let mut session = session_mutex.lock().unwrap();
    // 3. Run inference
    let outputs = session.run(inputs![
        "input_ids" => input_ids_tensor,
        "attention_mask" => attention_mask_tensor
    ])?;

    // 4. Extract output
    // CLIP text models usually output the pooled output or last_hidden_state
    // For standard export, it's typically named "output" or "pooler_output"
    // We'll extract the first output
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
