use std::sync::Arc;
use std::time::Duration;
use std::path::PathBuf;
use rusqlite::params;
use tracing::{info, debug, error};

use sm_database::DbPool;
use sm_scheduler::{Scheduler, Priority};
use crate::engine::MobileClipEngine;
use crate::vision::encode_image;

/// Spawns a background worker that scans for images at stage=2
/// and extracts their MobileCLIP embeddings, upgrading them to stage=3.
pub fn spawn_stage3_worker(
    scheduler: Arc<Scheduler>, 
    pool: DbPool,
    engine: Arc<MobileClipEngine>,
) {
    let _ = scheduler.spawn(Priority::Metadata, move |cancel_token| async move {
        info!("Stage 3 (AI Embedding) background worker started.");
        
        loop {
            if cancel_token.is_cancelled() {
                info!("Stage 3 worker cancelled.");
                break;
            }

            // Sleep a little to prevent busy looping
            tokio::time::sleep(Duration::from_secs(5)).await;

            // Fetch a batch of image files at stage 2
            let files_to_process = match fetch_pending_files(&pool, 50) {
                Ok(files) => files,
                Err(e) => {
                    error!("Error fetching pending stage 3 files: {}", e);
                    continue;
                }
            };

            if files_to_process.is_empty() {
                continue;
            }

            debug!("Processing Stage 3 for {} files", files_to_process.len());

            for (file_id, path_str) in files_to_process {
                if cancel_token.is_cancelled() {
                    break;
                }

                let path = PathBuf::from(path_str);
                
                // 1. Process Vision Embedding
                match encode_image(engine.image_session(), &path) {
                    Ok(embedding) => {
                        // Store the embedding and upgrade stage
                        if let Err(e) = store_embedding_and_upgrade(&pool, file_id, &embedding) {
                            error!("Failed to store embedding for file {}: {}", file_id, e);
                        }
                    }
                    Err(e) => {
                        error!("Image embedding failed for file {}: {}", file_id, e);
                        // Mark as stage 3 anyway so we don't infinitely retry failed images
                        let _ = mark_stage_complete(&pool, file_id, 3);
                    }
                }
            }
        }
    });
}

fn fetch_pending_files(pool: &DbPool, limit: usize) -> Result<Vec<(i64, String)>, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    // Only process images for now
    let mut stmt = conn.prepare("SELECT id, path FROM files WHERE stage = 2 AND category = 'image' AND kind = 'regular' LIMIT ?1")
        .map_err(|e| e.to_string())?;
        
    let rows = stmt.query_map(params![limit as i64], |row| {
        Ok((row.get(0)?, row.get(1)?))
    }).map_err(|e| e.to_string())?;

    let mut files = Vec::new();
    for row in rows {
        if let Ok(r) = row {
            files.push(r);
        }
    }
    
    Ok(files)
}

fn store_embedding_and_upgrade(pool: &DbPool, file_id: i64, embedding: &[f32]) -> Result<(), String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    
    // Store in sqlite-vec table
    // Serialize embedding array to raw bytes (f32 array) for sqlite-vec
    // sqlite-vec expects a BLOB of f32s.
    let embedding_bytes = unsafe {
        std::slice::from_raw_parts(
            embedding.as_ptr() as *const u8,
            embedding.len() * std::mem::size_of::<f32>(),
        )
    };

    tx.execute(
        "INSERT OR REPLACE INTO file_embeddings (file_id, embedding) VALUES (?1, ?2)",
        params![file_id, embedding_bytes]
    ).map_err(|e| e.to_string())?;

    // Upgrade stage
    tx.execute("UPDATE files SET stage = 3 WHERE id = ?1", params![file_id])
        .map_err(|e| e.to_string())?;
        
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

fn mark_stage_complete(pool: &DbPool, file_id: i64, stage: u8) -> Result<(), String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    conn.execute("UPDATE files SET stage = ?1 WHERE id = ?2", params![stage, file_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}
