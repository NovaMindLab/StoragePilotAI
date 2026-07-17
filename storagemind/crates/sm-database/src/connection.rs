//! SQLite connection pool using r2d2.

use std::path::Path;

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::OpenFlags;
use tracing::info;

use crate::error::{DbError, DbResult};
use crate::migrate;

/// Type alias for the connection pool.
pub type DbPool = Pool<SqliteConnectionManager>;

/// Create and initialize a connection pool.
///
/// - Enables WAL mode for concurrent reads
/// - Sets optimal SQLite PRAGMAs
/// - Runs all migrations
pub fn create_pool(db_path: &Path) -> DbResult<DbPool> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            DbError::Migration(format!("Failed to create DB directory: {e}"))
        })?;
    }

    // Auto-load sqlite-vec on all new connections
    static INIT_VEC: std::sync::Once = std::sync::Once::new();
    INIT_VEC.call_once(|| {
        unsafe {
            rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute(
                sqlite_vec::sqlite3_vec_init as *const ()
            )));
        }
    });

    info!("Opening database at {:?}", db_path);

    let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
        | OpenFlags::SQLITE_OPEN_CREATE
        | OpenFlags::SQLITE_OPEN_NO_MUTEX;

    let manager = SqliteConnectionManager::file(db_path)
        .with_flags(flags)
        .with_init(|conn| {
            // WAL mode for concurrent reads
            conn.execute_batch(
                "PRAGMA journal_mode = WAL;
                 PRAGMA wal_autocheckpoint = 1000;
                 PRAGMA synchronous = NORMAL;
                 PRAGMA foreign_keys = ON;
                 PRAGMA temp_store = MEMORY;
                 PRAGMA mmap_size = 268435456;  -- 256MB mmap
                 PRAGMA cache_size = -65536;    -- 64MB cache
                 PRAGMA page_size = 4096;
                 PRAGMA optimize;",
            )?;
            Ok(())
        });

    let pool = Pool::builder()
        .max_size(16)
        .min_idle(Some(2))
        .build(manager)?;

    // Run migrations on a single connection
    {
        let conn = pool.get()?;
        migrate::run_migrations(&conn)?;
    }

    info!("Database pool initialized successfully");
    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn pool_creates_and_migrates() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = create_pool(&db_path).unwrap();
        let conn = pool.get().unwrap();
        // Verify WAL mode
        let mode: String = conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap();
        assert_eq!(mode, "wal");
    }
}
