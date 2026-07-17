//! File repository — CRUD and bulk operations for the files table.

use rusqlite::{params, OptionalExtension};
use tracing::debug;

use crate::error::{DbError, DbResult};
use crate::DbPool;

/// A lightweight file record read from the database.
#[derive(Debug, Clone)]
pub struct FileRecord {
    pub id: i64,
    pub path: String,
    pub name: String,
    pub extension: Option<String>,
    pub size: i64,
    pub kind: String,
    pub category: String,
    pub parent_id: Option<i64>,
    pub depth: i64,
    pub inode: Option<i64>,
    pub is_hidden: bool,
    pub created_at: Option<i64>,
    pub modified_at: Option<i64>,
    pub accessed_at: Option<i64>,
    pub stage: i64,
}

/// Input for inserting a new file record.
#[derive(Debug, Clone)]
pub struct InsertFile<'a> {
    pub path: &'a str,
    pub name: &'a str,
    pub extension: Option<&'a str>,
    pub size: i64,
    pub kind: &'a str,
    pub category: &'a str,
    pub parent_id: Option<i64>,
    pub depth: i64,
    pub inode: Option<i64>,
    pub is_hidden: bool,
    pub created_at: Option<i64>,
    pub modified_at: Option<i64>,
    pub accessed_at: Option<i64>,
    pub scan_id: Option<&'a str>,
}

/// File repository backed by a connection pool.
pub struct FileRepo {
    pool: DbPool,
}

impl FileRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Insert or replace a file record (upsert on path).
    pub fn upsert(&self, file: &InsertFile<'_>) -> DbResult<i64> {
        let conn = self.pool.get()?;
        conn.execute(
            r#"INSERT INTO files
               (path, name, extension, size, kind, category, parent_id, depth,
                inode, is_hidden, created_at, modified_at, accessed_at, scan_id)
               VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)
               ON CONFLICT(path) DO UPDATE SET
                   name=excluded.name,
                   extension=excluded.extension,
                   size=excluded.size,
                   kind=excluded.kind,
                   category=excluded.category,
                   parent_id=excluded.parent_id,
                   depth=excluded.depth,
                   inode=excluded.inode,
                   is_hidden=excluded.is_hidden,
                   created_at=excluded.created_at,
                   modified_at=excluded.modified_at,
                   accessed_at=excluded.accessed_at,
                   scan_id=excluded.scan_id,
                   indexed_at=(strftime('%s','now')*1000)"#,
            params![
                file.path,
                file.name,
                file.extension,
                file.size,
                file.kind,
                file.category,
                file.parent_id,
                file.depth,
                file.inode,
                file.is_hidden as i64,
                file.created_at,
                file.modified_at,
                file.accessed_at,
                file.scan_id,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Bulk insert files using a transaction for maximum throughput.
    pub fn bulk_upsert(&self, files: &[InsertFile<'_>]) -> DbResult<u64> {
        if files.is_empty() {
            return Ok(0);
        }
        let mut conn = self.pool.get()?;
        let tx = conn.transaction()?;
        let mut count = 0u64;
        {
            let mut stmt = tx.prepare_cached(
                r#"INSERT INTO files
                   (path, name, extension, size, kind, category, parent_id, depth,
                    inode, is_hidden, created_at, modified_at, accessed_at, scan_id)
                   VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)
                   ON CONFLICT(path) DO UPDATE SET
                       name=excluded.name,
                       extension=excluded.extension,
                       size=excluded.size,
                       kind=excluded.kind,
                       category=excluded.category,
                       parent_id=excluded.parent_id,
                       depth=excluded.depth,
                       inode=excluded.inode,
                       is_hidden=excluded.is_hidden,
                       created_at=excluded.created_at,
                       modified_at=excluded.modified_at,
                       accessed_at=excluded.accessed_at,
                       scan_id=excluded.scan_id,
                       indexed_at=(strftime('%s','now')*1000)"#,
            )?;
            for file in files {
                stmt.execute(params![
                    file.path,
                    file.name,
                    file.extension,
                    file.size,
                    file.kind,
                    file.category,
                    file.parent_id,
                    file.depth,
                    file.inode,
                    file.is_hidden as i64,
                    file.created_at,
                    file.modified_at,
                    file.accessed_at,
                    file.scan_id,
                ])?;
                count += 1;
            }
        }
        tx.commit()?;
        debug!("Bulk upserted {} files", count);
        Ok(count)
    }

    /// Get file by ID.
    pub fn get_by_id(&self, id: i64) -> DbResult<Option<FileRecord>> {
        let conn = self.pool.get()?;
        let result = conn
            .query_row(
                "SELECT id, path, name, extension, size, kind, category, parent_id,
                        depth, inode, is_hidden, created_at, modified_at, accessed_at, stage
                 FROM files WHERE id = ?1",
                params![id],
                map_file_record,
            )
            .optional()?;
        Ok(result)
    }

    /// Get file by path.
    pub fn get_by_path(&self, path: &str) -> DbResult<Option<FileRecord>> {
        let conn = self.pool.get()?;
        let result = conn
            .query_row(
                "SELECT id, path, name, extension, size, kind, category, parent_id,
                        depth, inode, is_hidden, created_at, modified_at, accessed_at, stage
                 FROM files WHERE path = ?1",
                params![path],
                map_file_record,
            )
            .optional()?;
        Ok(result)
    }

    /// List children of a directory.
    pub fn list_children(&self, parent_id: i64, limit: i64, offset: i64) -> DbResult<Vec<FileRecord>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached(
            "SELECT id, path, name, extension, size, kind, category, parent_id,
                    depth, inode, is_hidden, created_at, modified_at, accessed_at, stage
             FROM files WHERE parent_id = ?1
             ORDER BY kind DESC, name ASC
             LIMIT ?2 OFFSET ?3",
        )?;
        let rows = stmt.query_map(params![parent_id, limit, offset], map_file_record)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::Sqlite)
    }

    /// Get top N largest files.
    pub fn largest_files(&self, limit: i64) -> DbResult<Vec<FileRecord>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached(
            "SELECT id, path, name, extension, size, kind, category, parent_id,
                    depth, inode, is_hidden, created_at, modified_at, accessed_at, stage
             FROM files WHERE kind = 'regular'
             ORDER BY size DESC
             LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], map_file_record)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::Sqlite)
    }

    /// Get total file count and size.
    pub fn total_stats(&self) -> DbResult<(i64, i64)> {
        let conn = self.pool.get()?;
        let (count, total_size) = conn.query_row(
            "SELECT COUNT(*), COALESCE(SUM(size), 0) FROM files WHERE kind = 'regular'",
            [],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)),
        )?;
        Ok((count, total_size))
    }

    /// Count files by category.
    pub fn count_by_category(&self) -> DbResult<Vec<(String, i64, i64)>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached(
            "SELECT category, COUNT(*) as cnt, COALESCE(SUM(size), 0) as total_size
             FROM files WHERE kind = 'regular'
             GROUP BY category ORDER BY total_size DESC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?))
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::Sqlite)
    }

    /// Delete a file by ID.
    pub fn delete(&self, id: i64) -> DbResult<()> {
        let conn = self.pool.get()?;
        conn.execute("DELETE FROM files WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Delete all files under a scan session.
    pub fn delete_by_scan(&self, scan_id: &str) -> DbResult<u64> {
        let conn = self.pool.get()?;
        let n = conn.execute(
            "DELETE FROM files WHERE scan_id != ?1 AND scan_id IS NOT NULL",
            params![scan_id],
        )?;
        Ok(n as u64)
    }
}

fn map_file_record(r: &rusqlite::Row<'_>) -> rusqlite::Result<FileRecord> {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::create_pool;
    use tempfile::tempdir;

    fn test_pool() -> DbPool {
        let dir = tempdir().unwrap();
        create_pool(&dir.path().join("test.db")).unwrap()
    }

    #[test]
    fn upsert_and_get_by_path() {
        let pool = test_pool();
        let repo = FileRepo::new(pool);
        let file = InsertFile {
            path: "/tmp/test.txt",
            name: "test.txt",
            extension: Some("txt"),
            size: 1024,
            kind: "regular",
            category: "document",
            parent_id: None,
            depth: 1,
            inode: None,
            is_hidden: false,
            created_at: None,
            modified_at: None,
            accessed_at: None,
            scan_id: None,
        };
        repo.upsert(&file).unwrap();
        let record = repo.get_by_path("/tmp/test.txt").unwrap().unwrap();
        assert_eq!(record.name, "test.txt");
        assert_eq!(record.size, 1024);
    }

    #[test]
    fn bulk_upsert_performance() {
        let pool = test_pool();
        let repo = FileRepo::new(pool);
        let files: Vec<InsertFile<'_>> = (0..1000)
            .map(|i| InsertFile {
                path: Box::leak(format!("/tmp/file_{i}.txt").into_boxed_str()),
                name: Box::leak(format!("file_{i}.txt").into_boxed_str()),
                extension: Some("txt"),
                size: i * 1024,
                kind: "regular",
                category: "document",
                parent_id: None,
                depth: 1,
                inode: None,
                is_hidden: false,
                created_at: None,
                modified_at: None,
                accessed_at: None,
                scan_id: None,
            })
            .collect();
        let count = repo.bulk_upsert(&files).unwrap();
        assert_eq!(count, 1000);
        let (total, _) = repo.total_stats().unwrap();
        assert_eq!(total, 1000);
    }
}
