//! File data commands.

use serde::{Deserialize, Serialize};
use tauri::State;

use sm_database::repo::file_repo::FileRecord;
use crate::state::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub id: i64,
    pub path: String,
    pub name: String,
    pub extension: Option<String>,
    pub size: i64,
    pub kind: String,
    pub category: String,
    pub depth: i64,
    pub is_hidden: bool,
    pub modified_at: Option<i64>,
}

impl From<FileRecord> for FileEntry {
    fn from(r: FileRecord) -> Self {
        Self {
            id: r.id,
            path: r.path,
            name: r.name,
            extension: r.extension,
            size: r.size,
            kind: r.kind,
            category: r.category,
            depth: r.depth,
            is_hidden: r.is_hidden,
            modified_at: r.modified_at,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListChildrenParams {
    pub parent_id: i64,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// List children of a directory.
#[tauri::command]
pub async fn cmd_list_children(
    params: ListChildrenParams,
    state: State<'_, AppState>,
) -> Result<Vec<FileEntry>, String> {
    let limit = params.limit.unwrap_or(200);
    let offset = params.offset.unwrap_or(0);
    state
        .index
        .file_repo()
        .list_children(params.parent_id, limit, offset)
        .map(|files| files.into_iter().map(FileEntry::from).collect())
        .map_err(|e| e.to_string())
}

/// Get top N largest files.
#[tauri::command]
pub async fn cmd_get_largest_files(
    n: Option<i64>,
    state: State<'_, AppState>,
) -> Result<Vec<FileEntry>, String> {
    state
        .index
        .largest_files(n.unwrap_or(100))
        .map(|files| files.into_iter().map(FileEntry::from).collect())
        .map_err(|e| e.to_string())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryStat {
    pub category: String,
    pub file_count: i64,
    pub total_size: i64,
}

/// Get file counts grouped by category.
#[tauri::command]
pub async fn cmd_get_category_stats(
    state: State<'_, AppState>,
) -> Result<Vec<CategoryStat>, String> {
    state
        .index
        .category_stats()
        .map(|stats| {
            stats
                .into_iter()
                .map(|(cat, count, size)| CategoryStat {
                    category: cat,
                    file_count: count,
                    total_size: size,
                })
                .collect()
        })
        .map_err(|e| e.to_string())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TotalStats {
    pub total_files: i64,
    pub total_size: i64,
}

/// Get total file count and size.
#[tauri::command]
pub async fn cmd_get_total_stats(state: State<'_, AppState>) -> Result<TotalStats, String> {
    state
        .index
        .total_stats()
        .map(|(count, size)| TotalStats {
            total_files: count,
            total_size: size,
        })
        .map_err(|e| e.to_string())
}
