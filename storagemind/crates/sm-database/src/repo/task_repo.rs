//! Task queue repository.

use rusqlite::{params, OptionalExtension};
use tracing::debug;

use crate::error::{DbError, DbResult};
use crate::DbPool;

#[derive(Debug, Clone)]
pub struct TaskRecord {
    pub id: String,
    pub kind: String,
    pub priority: i64,
    pub status: String,
    pub payload: String,
    pub progress: f64,
    pub error_msg: Option<String>,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
}

pub struct TaskRepo {
    pool: DbPool,
}

impl TaskRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub fn enqueue(&self, id: &str, kind: &str, priority: i64, payload: &str) -> DbResult<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO tasks (id, kind, priority, payload) VALUES (?1, ?2, ?3, ?4)",
            params![id, kind, priority, payload],
        )?;
        debug!("Enqueued task {} ({}) priority={}", id, kind, priority);
        Ok(())
    }

    pub fn update_status(&self, id: &str, status: &str, progress: f64, error_msg: Option<&str>) -> DbResult<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE tasks SET status=?1, progress=?2, error_msg=?3,
             started_at = CASE WHEN ?1='running' AND started_at IS NULL
                          THEN (strftime('%s','now')*1000) ELSE started_at END,
             completed_at = CASE WHEN ?1 IN ('done','failed','cancelled')
                            THEN (strftime('%s','now')*1000) ELSE NULL END
             WHERE id=?4",
            params![status, progress, error_msg, id],
        )?;
        Ok(())
    }

    pub fn get(&self, id: &str) -> DbResult<Option<TaskRecord>> {
        let conn = self.pool.get()?;
        conn.query_row(
            "SELECT id, kind, priority, status, payload, progress, error_msg,
                    created_at, started_at, completed_at
             FROM tasks WHERE id = ?1",
            params![id],
            map_task,
        )
        .optional()
        .map_err(DbError::Sqlite)
    }

    pub fn list_pending(&self, limit: i64) -> DbResult<Vec<TaskRecord>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached(
            "SELECT id, kind, priority, status, payload, progress, error_msg,
                    created_at, started_at, completed_at
             FROM tasks WHERE status IN ('queued','paused')
             ORDER BY priority ASC, created_at ASC LIMIT ?1",
        )?;
        let result: Result<Vec<_>, _> = stmt.query_map(params![limit], map_task)
            .map_err(DbError::Sqlite)?
            .collect();
        result.map_err(DbError::Sqlite)
    }

    pub fn delete(&self, id: &str) -> DbResult<()> {
        let conn = self.pool.get()?;
        conn.execute("DELETE FROM tasks WHERE id = ?1", params![id])?;
        Ok(())
    }
}

fn map_task(r: &rusqlite::Row<'_>) -> rusqlite::Result<TaskRecord> {
    Ok(TaskRecord {
        id: r.get(0)?,
        kind: r.get(1)?,
        priority: r.get(2)?,
        status: r.get(3)?,
        payload: r.get(4)?,
        progress: r.get(5)?,
        error_msg: r.get(6)?,
        created_at: r.get(7)?,
        started_at: r.get(8)?,
        completed_at: r.get(9)?,
    })
}
