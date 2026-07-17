//! Full-text search repository using SQLite FTS5.

use rusqlite::params;
use tracing::debug;

use crate::error::{DbError, DbResult};
use crate::repo::file_repo::FileRecord;
use crate::DbPool;

/// Search query parameters.
#[derive(Debug, Clone)]
pub struct SearchParams {
    pub query: String,
    pub category: Option<String>,
    pub kind: Option<String>,
    pub size_min: Option<i64>,
    pub size_max: Option<i64>,
    pub modified_after: Option<i64>,
    pub modified_before: Option<i64>,
    pub limit: i64,
    pub offset: i64,
}

impl Default for SearchParams {
    fn default() -> Self {
        Self {
            query: String::new(),
            category: None,
            kind: None,
            size_min: None,
            size_max: None,
            modified_after: None,
            modified_before: None,
            limit: 100,
            offset: 0,
        }
    }
}

/// Search result with total count and timing.
#[derive(Debug)]
pub struct SearchResults {
    pub files: Vec<FileRecord>,
    pub total: i64,
    pub elapsed_ms: u64,
}

pub struct SearchRepo {
    pool: DbPool,
}

impl SearchRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Full-text search using FTS5.
    pub fn search(&self, params: &SearchParams) -> DbResult<SearchResults> {
        if params.query.trim().is_empty() {
            return Ok(SearchResults { files: vec![], total: 0, elapsed_ms: 0 });
        }

        let start = std::time::Instant::now();
        let conn = self.pool.get()?;

        // Build FTS5 query with filters
        let fts_query = format!("{}*", params.query.trim().replace('"', "\"\""));

        // Build WHERE clauses for file table filters
        let mut filters = vec!["f.id = fts.rowid".to_string()];
        if let Some(ref cat) = params.category {
            filters.push(format!("f.category = '{}'", cat.replace('\'', "''")));
        }
        if let Some(ref kind) = params.kind {
            filters.push(format!("f.kind = '{}'", kind.replace('\'', "''")));
        }
        if let Some(min) = params.size_min {
            filters.push(format!("f.size >= {min}"));
        }
        if let Some(max) = params.size_max {
            filters.push(format!("f.size <= {max}"));
        }
        if let Some(after) = params.modified_after {
            filters.push(format!("f.modified_at >= {after}"));
        }
        if let Some(before) = params.modified_before {
            filters.push(format!("f.modified_at <= {before}"));
        }

        let where_clause = filters.join(" AND ");

        let count_sql = format!(
            "SELECT COUNT(*) FROM files_fts fts, files f
             WHERE files_fts MATCH ?1 AND {where_clause}"
        );
        let total: i64 = conn
            .query_row(&count_sql, params![fts_query], |r| r.get(0))
            .unwrap_or(0);

        let query_sql = format!(
            "SELECT f.id, f.path, f.name, f.extension, f.size, f.kind, f.category,
                    f.parent_id, f.depth, f.inode, f.is_hidden,
                    f.created_at, f.modified_at, f.accessed_at, f.stage
             FROM files_fts fts, files f
             WHERE files_fts MATCH ?1 AND {where_clause}
             ORDER BY rank
             LIMIT ?2 OFFSET ?3"
        );

        let mut stmt = conn.prepare(&query_sql)?;
        let rows = stmt.query_map(
            params![fts_query, params.limit, params.offset],
            |r| {
                Ok(FileRecord {
                    id: r.get(0)?,
                    path: r.get(1)?,
                    name: r.get(2)?,
                    extension: r.get(3)?,
                    size: r.get(4)?,
                    kind: r.get(5)?,
                    category: r.get(6)?,
                    parent_id: r.get(7)?,
                    depth: r.get(8)?,
                    inode: r.get(9)?,
                    is_hidden: r.get::<_, i64>(10)? != 0,
                    created_at: r.get(11)?,
                    modified_at: r.get(12)?,
                    accessed_at: r.get(13)?,
                    stage: r.get(14)?,
                })
            },
        )?;

        let files = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(DbError::Sqlite)?;

        let elapsed_ms = start.elapsed().as_millis() as u64;
        debug!(
            "Search '{}' → {} results in {}ms",
            params.query, total, elapsed_ms
        );

        Ok(SearchResults { files, total, elapsed_ms })
    }

    /// Rebuild the FTS5 index (use after bulk imports).
    pub fn rebuild_fts(&self) -> DbResult<()> {
        let conn = self.pool.get()?;
        conn.execute_batch("INSERT INTO files_fts(files_fts) VALUES('rebuild')")?;
        Ok(())
    }

    /// Optimize the FTS5 index.
    pub fn optimize_fts(&self) -> DbResult<()> {
        let conn = self.pool.get()?;
        conn.execute_batch("INSERT INTO files_fts(files_fts) VALUES('optimize')")?;
        Ok(())
    }
}
