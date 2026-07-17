//! Database migrations system.
//!
//! Migrations are applied in version order and are idempotent —
//! they check the current schema version before applying.

use rusqlite::Connection;
use tracing::info;

use crate::error::{DbError, DbResult};
use crate::schema::{SCHEMA_V1, SCHEMA_VERSION};

/// Run all pending migrations on the given connection.
pub fn run_migrations(conn: &Connection) -> DbResult<()> {
    let current_version = get_schema_version(conn)?;
    info!(
        "Current schema version: {}, target: {}",
        current_version, SCHEMA_VERSION
    );

    if current_version < 1 {
        apply_v1(conn)?;
    }
    
    if current_version < 2 {
        apply_v2(conn)?;
    }

    if current_version == SCHEMA_VERSION {
        info!("Database schema is up to date (v{})", SCHEMA_VERSION);
    }

    Ok(())
}

fn get_schema_version(conn: &Connection) -> DbResult<u32> {
    // Check if schema_versions table exists
    let table_exists: bool = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='schema_versions'",
        [],
        |r| r.get::<_, i64>(0),
    ).map(|n| n > 0).unwrap_or(false);

    if !table_exists {
        return Ok(0);
    }

    let version: u32 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_versions",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    Ok(version)
}

fn apply_v1(conn: &Connection) -> DbResult<()> {
    info!("Applying schema migration v1...");
    conn.execute_batch(SCHEMA_V1)
        .map_err(|e| DbError::Migration(format!("v1 migration failed: {e}")))?;

    conn.execute(
        "INSERT OR IGNORE INTO schema_versions (version, description) VALUES (?1, ?2)",
        rusqlite::params![1u32, "Initial schema: files, metadata, hashes, FTS5, tasks"],
    )?;

    info!("Schema migration v1 applied successfully");
    Ok(())
}

fn apply_v2(conn: &Connection) -> DbResult<()> {
    info!("Applying schema migration v2...");
    conn.execute_batch(r#"
        CREATE VIRTUAL TABLE IF NOT EXISTS file_embeddings USING vec0(
            file_id INTEGER PRIMARY KEY,
            embedding float[512]
        );
    "#).map_err(|e| DbError::Migration(format!("v2 migration failed: {e}")))?;

    conn.execute(
        "INSERT OR IGNORE INTO schema_versions (version, description) VALUES (?1, ?2)",
        rusqlite::params![2u32, "Added file_embeddings virtual table"],
    )?;

    info!("Schema migration v2 applied successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn in_memory_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;",
        )
        .unwrap();
        conn
    }

    #[test]
    fn migrations_run_cleanly() {
        let conn = in_memory_conn();
        run_migrations(&conn).unwrap();
        // Running again should be idempotent
        run_migrations(&conn).unwrap();
    }

    #[test]
    fn schema_version_is_correct_after_migration() {
        let conn = in_memory_conn();
        run_migrations(&conn).unwrap();
        let version = get_schema_version(&conn).unwrap();
        assert_eq!(version, SCHEMA_VERSION);
    }

    #[test]
    fn fts5_table_exists() {
        let conn = in_memory_conn();
        run_migrations(&conn).unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='files_fts'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }
}
