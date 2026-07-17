use std::sync::Arc;
use std::time::Duration;
use std::path::PathBuf;

use sm_database::DbPool;
use sm_scheduler::{Scheduler, Priority};
use sm_types::file::FileCategory;
use tracing::{info, debug, error};

use crate::processor::MetadataProcessor;

/// Spawns a background worker that continuously scans the database for files
/// that have stage=1, and upgrades them to stage=2 by extracting metadata and hashes.
pub fn spawn_stage2_worker(scheduler: Arc<Scheduler>, pool: DbPool) {
    let processor = MetadataProcessor::new(pool.clone());
    
    // We spawn this on the scheduler with Priority::Metadata (P3)
    let _ = scheduler.spawn(Priority::Metadata, move |cancel_token| async move {
        info!("Stage 2 background worker started.");
        
        loop {
            if cancel_token.is_cancelled() {
                info!("Stage 2 worker cancelled.");
                break;
            }

            // Sleep a little to prevent busy looping if the database is idle
            tokio::time::sleep(Duration::from_secs(5)).await;

            // Fetch a batch of files that need stage 2 processing
            let files_to_process = match fetch_pending_files(&pool, 100) {
                Ok(files) => files,
                Err(e) => {
                    error!("Error fetching pending stage 2 files: {}", e);
                    continue;
                }
            };

            if files_to_process.is_empty() {
                continue;
            }

            debug!("Processing Stage 2 for {} files", files_to_process.len());

            for (file_id, path_str, category_str) in files_to_process {
                if cancel_token.is_cancelled() {
                    break;
                }

                let path = PathBuf::from(path_str);
                
                // Parse category safely
                let category = parse_category(&category_str);

                // 1. Process Hashes
                if let Err(e) = processor.process_hash(file_id, path.clone()) {
                    error!("Hash processing failed for file {}: {}", file_id, e);
                }

                // 2. Process Metadata (EXIF, audio, etc)
                if let Err(e) = processor.process_metadata(file_id, path, &category) {
                    error!("Metadata processing failed for file {}: {}", file_id, e);
                }

                // Mark file as stage 2 complete
                if let Err(e) = mark_stage_complete(&pool, file_id, 2) {
                    error!("Failed to mark file {} as stage 2 complete: {}", file_id, e);
                }
            }
        }
    });
}

fn fetch_pending_files(pool: &DbPool, limit: usize) -> Result<Vec<(i64, String, String)>, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT id, path, category FROM files WHERE stage = 1 AND kind = 'regular' LIMIT ?1")
        .map_err(|e| e.to_string())?;
        
    let rows = stmt.query_map(rusqlite::params![limit as i64], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    }).map_err(|e| e.to_string())?;

    let mut files = Vec::new();
    for row in rows {
        if let Ok(r) = row {
            files.push(r);
        }
    }
    
    Ok(files)
}

fn mark_stage_complete(pool: &DbPool, file_id: i64, stage: u8) -> Result<(), String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    conn.execute("UPDATE files SET stage = ?1 WHERE id = ?2", rusqlite::params![stage, file_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn parse_category(cat: &str) -> FileCategory {
    match cat {
        "image" => FileCategory::Image,
        "video" => FileCategory::Video,
        "audio" => FileCategory::Audio,
        "document" => FileCategory::Document,
        "archive" => FileCategory::Archive,
        "code" => FileCategory::Code,
        "system" => FileCategory::System,
        _ => FileCategory::Other,
    }
}
