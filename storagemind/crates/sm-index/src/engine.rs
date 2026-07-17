use std::sync::Arc;
use tracing::info;
use sm_database::DbPool;
use sm_database::repo::{FileRepo, SearchRepo};
use crate::error::{IndexError, IndexResult};

/// The central Index Engine.
/// All queries to the file system go through this — never directly to disk.
pub struct IndexEngine {
    pool: DbPool,
    file_repo: Arc<FileRepo>,
    search_repo: Arc<SearchRepo>,
}

impl IndexEngine {
    pub fn new(pool: DbPool) -> Self {
        let file_repo = Arc::new(FileRepo::new(pool.clone()));
        let search_repo = Arc::new(SearchRepo::new(pool.clone()));
        Self { pool, file_repo, search_repo }
    }

    pub fn file_repo(&self) -> &FileRepo {
        &self.file_repo
    }

    pub fn search_repo(&self) -> &SearchRepo {
        &self.search_repo
    }

    /// Get total file count and total size.
    pub fn total_stats(&self) -> IndexResult<(i64, i64)> {
        self.file_repo
            .total_stats()
            .map_err(|e| IndexError::Database(e.to_string()))
    }

    /// Get top N largest files.
    pub fn largest_files(
        &self,
        n: i64,
    ) -> IndexResult<Vec<sm_database::repo::file_repo::FileRecord>> {
        self.file_repo
            .largest_files(n)
            .map_err(|e| IndexError::Database(e.to_string()))
    }

    /// Get file count grouped by category.
    pub fn category_stats(&self) -> IndexResult<Vec<(String, i64, i64)>> {
        self.file_repo
            .count_by_category()
            .map_err(|e| IndexError::Database(e.to_string()))
    }
}
