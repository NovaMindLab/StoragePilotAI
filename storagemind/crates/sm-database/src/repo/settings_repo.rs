//! Settings key-value store repository.

use rusqlite::{params, OptionalExtension};

use crate::error::{DbError, DbResult};
use crate::DbPool;

pub struct SettingsRepo {
    pool: DbPool,
}

impl SettingsRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub fn get(&self, key: &str) -> DbResult<Option<String>> {
        let conn = self.pool.get()?;
        conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |r| r.get(0),
        )
        .optional()
        .map_err(DbError::Sqlite)
    }

    pub fn set(&self, key: &str, value: &str) -> DbResult<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value,
             updated_at=(strftime('%s','now')*1000)",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn delete(&self, key: &str) -> DbResult<()> {
        let conn = self.pool.get()?;
        conn.execute("DELETE FROM settings WHERE key = ?1", params![key])?;
        Ok(())
    }

    pub fn get_all(&self) -> DbResult<Vec<(String, String)>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached("SELECT key, value FROM settings ORDER BY key")?;
        let result: Result<Vec<_>, _> = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .map_err(DbError::Sqlite)?
            .collect();
        result.map_err(DbError::Sqlite)
    }
}
