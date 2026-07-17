//! Search commands.

use serde::{Deserialize, Serialize};
use tauri::State;

use sm_database::repo::search_repo::SearchParams;
use crate::commands::files::FileEntry;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchFilesParams {
    pub query: String,
    pub category: Option<String>,
    pub kind: Option<String>,
    pub size_min: Option<i64>,
    pub size_max: Option<i64>,
    pub modified_after: Option<i64>,
    pub modified_before: Option<i64>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    pub files: Vec<FileEntry>,
    pub total: i64,
    pub elapsed_ms: u64,
}

/// Search files using FTS5 full-text search.
#[tauri::command]
pub async fn cmd_search_files(
    params: SearchFilesParams,
    state: State<'_, AppState>,
) -> Result<SearchResponse, String> {
    let search_params = SearchParams {
        query: params.query,
        category: params.category,
        kind: params.kind,
        size_min: params.size_min,
        size_max: params.size_max,
        modified_after: params.modified_after,
        modified_before: params.modified_before,
        limit: params.limit.unwrap_or(100),
        offset: params.offset.unwrap_or(0),
    };

    state
        .index
        .search_repo()
        .search(&search_params)
        .map(|results| SearchResponse {
            files: results.files.into_iter().map(FileEntry::from).collect(),
            total: results.total,
            elapsed_ms: results.elapsed_ms,
        })
        .map_err(|e| e.to_string())
}
