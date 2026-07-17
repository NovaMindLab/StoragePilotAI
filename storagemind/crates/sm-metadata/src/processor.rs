use std::path::PathBuf;
use tracing::debug;

use sm_database::DbPool;
use sm_types::file::FileCategory;

use crate::audio::extract_audio_metadata;
use crate::error::MetadataResult;
use crate::hash::calculate_hashes;
use crate::image::extract_image_metadata;

pub struct MetadataProcessor {
    pool: DbPool,
}

impl MetadataProcessor {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Process hash extraction for a single file.
    pub fn process_hash(&self, file_id: i64, path: PathBuf) -> MetadataResult<()> {
        debug!("Processing hash for file {}", file_id);
        if !path.exists() {
            return Ok(());
        }
        
        let hashes = calculate_hashes(&path)?;
        
        let conn = self.pool.get().map_err(|e| crate::error::MetadataError::Database(e.to_string()))?;
        conn.execute(
            "INSERT OR REPLACE INTO file_hashes (file_id, hash_blake3, hash_xxh3, computed_at) 
             VALUES (?1, ?2, ?3, (strftime('%s', 'now') * 1000))",
            rusqlite::params![file_id, hashes.blake3, hashes.xxh3],
        ).map_err(|e| crate::error::MetadataError::Database(e.to_string()))?;
        
        Ok(())
    }

    /// Process deep metadata (EXIF, Audio duration) based on file category.
    pub fn process_metadata(&self, file_id: i64, path: PathBuf, category: &FileCategory) -> MetadataResult<()> {
        debug!("Processing metadata for file {}", file_id);
        if !path.exists() {
            return Ok(());
        }

        let conn = self.pool.get().map_err(|e| crate::error::MetadataError::Database(e.to_string()))?;

        // Initialize record if not exists
        conn.execute(
            "INSERT OR IGNORE INTO file_metadata (file_id) VALUES (?1)",
            rusqlite::params![file_id],
        ).map_err(|e| crate::error::MetadataError::Database(e.to_string()))?;

        match category {
            FileCategory::Image => {
                let meta = extract_image_metadata(&path)?;
                conn.execute(
                    "UPDATE file_metadata SET width = ?1, height = ?2, exif_data = ?3 WHERE file_id = ?4",
                    rusqlite::params![meta.width, meta.height, meta.exif_json, file_id],
                ).map_err(|e| crate::error::MetadataError::Database(e.to_string()))?;
            }
            FileCategory::Audio | FileCategory::Video => {
                let meta = extract_audio_metadata(&path)?;
                conn.execute(
                    "UPDATE file_metadata SET duration_secs = ?1, bitrate = ?2 WHERE file_id = ?3",
                    rusqlite::params![meta.duration_secs, meta.bitrate, file_id],
                ).map_err(|e| crate::error::MetadataError::Database(e.to_string()))?;
            }
            _ => {}
        }

        Ok(())
    }
}
