use std::path::Path;
use std::io::Cursor;
use image::{DynamicImage, ImageFormat};
use tracing::debug;

use crate::error::{ThumbnailError, ThumbnailResult};
use sm_database::DbPool;

/// Default thumbnail dimension (longest edge).
pub const THUMB_SIZE: u32 = 256;

/// Generates JPEG thumbnails from image files and persists them in the database.
pub struct ThumbnailGenerator {
    pool: DbPool,
    thumb_size: u32,
}

impl ThumbnailGenerator {
    pub fn new(pool: DbPool) -> Self {
        Self {
            pool,
            thumb_size: THUMB_SIZE,
        }
    }

    /// Override the default thumbnail size.
    pub fn with_size(mut self, size: u32) -> Self {
        self.thumb_size = size;
        self
    }

    /// Generate a JPEG thumbnail for `path`, store it in the DB, and return the bytes.
    pub fn generate_and_store(&self, file_id: i64, path: &Path) -> ThumbnailResult<Vec<u8>> {
        let img = image::open(path)?;
        let thumb = img.thumbnail(self.thumb_size, self.thumb_size);
        let jpeg_bytes = self.encode_jpeg(&thumb)?;
        self.store(file_id, &jpeg_bytes, thumb.width(), thumb.height())?;
        debug!(
            "Generated thumbnail for file_id={} ({} bytes)",
            file_id,
            jpeg_bytes.len()
        );
        Ok(jpeg_bytes)
    }

    fn encode_jpeg(&self, img: &DynamicImage) -> ThumbnailResult<Vec<u8>> {
        let mut buf = Cursor::new(Vec::new());
        img.write_to(&mut buf, ImageFormat::Jpeg)
            .map_err(ThumbnailError::Image)?;
        Ok(buf.into_inner())
    }

    fn store(&self, file_id: i64, data: &[u8], w: u32, h: u32) -> ThumbnailResult<()> {
        let conn = self
            .pool
            .get()
            .map_err(|e| ThumbnailError::Database(e.to_string()))?;
        conn.execute(
            "INSERT INTO thumbnails (file_id, data, width, height, size_bytes)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(file_id) DO UPDATE SET
                 data        = excluded.data,
                 width       = excluded.width,
                 height      = excluded.height,
                 size_bytes  = excluded.size_bytes,
                 generated_at = (strftime('%s','now') * 1000)",
            rusqlite::params![file_id, data, w, h, data.len() as i64],
        )
        .map_err(|e| ThumbnailError::Database(e.to_string()))?;
        Ok(())
    }

    /// Retrieve a previously stored thumbnail, if any.
    pub fn get(&self, file_id: i64) -> ThumbnailResult<Option<Vec<u8>>> {
        let conn = self
            .pool
            .get()
            .map_err(|e| ThumbnailError::Database(e.to_string()))?;
        let result: Option<Vec<u8>> = conn
            .query_row(
                "SELECT data FROM thumbnails WHERE file_id = ?1",
                rusqlite::params![file_id],
                |r| r.get(0),
            )
            .ok();
        Ok(result)
    }
}
