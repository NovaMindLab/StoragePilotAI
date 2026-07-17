use std::path::Path;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::error::{CleanError, CleanResult};
use crate::rules::CleanRule;
use sm_database::DbPool;

/// A preview of what a rule would clean (dry-run output).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanPreview {
    pub rule: CleanRule,
    /// Absolute paths of files that would be removed.
    pub files: Vec<String>,
    pub total_size_bytes: u64,
}

/// Summary of a completed clean operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanReport {
    pub files_deleted: u64,
    pub bytes_freed: u64,
    /// Per-file errors that did not abort the overall run.
    pub errors: Vec<String>,
}

/// The main cleaner engine.
pub struct Cleaner {
    pool: DbPool,
    /// When true, files are moved to the OS trash rather than permanently deleted.
    use_trash: bool,
}

impl Cleaner {
    pub fn new(pool: DbPool, use_trash: bool) -> Self {
        Self { pool, use_trash }
    }

    /// Preview what would be cleaned by a rule without performing any deletions.
    pub fn preview(&self, rule: &CleanRule) -> CleanResult<CleanPreview> {
        // TODO: implement per-RuleType DB queries to collect candidate files.
        Ok(CleanPreview {
            rule: rule.clone(),
            files: vec![],
            total_size_bytes: 0,
        })
    }

    /// Execute the cleaning rule. Calls `preview` first, then deletes each file.
    pub fn execute(&self, rule: &CleanRule) -> CleanResult<CleanReport> {
        info!("Executing clean rule: {}", rule.name);
        let preview = self.preview(rule)?;

        let mut deleted: u64 = 0;
        let mut bytes_freed: u64 = 0;
        let mut errors: Vec<String> = Vec::new();

        for file_path in &preview.files {
            let path = Path::new(file_path);
            match self.delete_file(path) {
                Ok(size) => {
                    deleted += 1;
                    bytes_freed += size;
                }
                Err(e) => {
                    warn!("Failed to delete {}: {}", file_path, e);
                    errors.push(format!("{}: {}", file_path, e));
                }
            }
        }

        info!("Cleaned {} files, freed {} bytes", deleted, bytes_freed);
        Ok(CleanReport {
            files_deleted: deleted,
            bytes_freed,
            errors,
        })
    }

    /// Delete a single file, respecting the `use_trash` setting.
    /// Returns the size in bytes that was freed.
    fn delete_file(&self, path: &Path) -> CleanResult<u64> {
        let size = path.metadata().map(|m| m.len()).unwrap_or(0);
        if self.use_trash {
            // TODO: replace with `trash` crate for proper recycle-bin support.
            std::fs::remove_file(path)?;
        } else {
            std::fs::remove_file(path)?;
        }
        Ok(size)
    }
}
