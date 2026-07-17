//! Cleaner commands.

use serde::{Deserialize, Serialize};
use tauri::State;

use sm_cleaner::{Cleaner, rules::{CleanRule, RuleType}};
use crate::state::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanPreviewResponse {
    pub files: Vec<String>,
    pub total_size_bytes: u64,
    pub rule_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewCleanParams {
    pub rule_type: String, // "temp_files" | "large_files" | "duplicates" | "old_downloads"
    pub min_size_mb: Option<u64>,
    pub days_old: Option<u32>,
}

/// Preview what would be cleaned (dry run).
#[tauri::command]
pub async fn cmd_preview_clean(
    params: PreviewCleanParams,
    state: State<'_, AppState>,
) -> Result<CleanPreviewResponse, String> {
    let rule_type = match params.rule_type.as_str() {
        "temp_files" => RuleType::TempFiles,
        "large_files" => RuleType::LargeFiles {
            min_size_bytes: params.min_size_mb.unwrap_or(100) * 1_048_576,
        },
        "duplicates" => RuleType::Duplicates,
        "old_downloads" => RuleType::OldDownloads {
            days: params.days_old.unwrap_or(90),
        },
        "empty_folders" => RuleType::EmptyFolders,
        _ => return Err(format!("Unknown rule type: {}", params.rule_type)),
    };

    let rule = CleanRule {
        id: 0,
        name: params.rule_type.clone(),
        description: String::new(),
        rule_type,
        enabled: true,
    };

    let cleaner = Cleaner::new(state.pool.clone(), state.config.read().cleaner.use_trash);
    let preview = cleaner.preview(&rule).map_err(|e| e.to_string())?;

    Ok(CleanPreviewResponse {
        files: preview.files,
        total_size_bytes: preview.total_size_bytes,
        rule_name: rule.name,
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanExecuteResponse {
    pub files_deleted: u64,
    pub bytes_freed: u64,
    pub errors: Vec<String>,
}

/// Execute cleaning (with confirmation expected from frontend).
#[tauri::command]
pub async fn cmd_execute_clean(
    params: PreviewCleanParams,
    state: State<'_, AppState>,
) -> Result<CleanExecuteResponse, String> {
    let rule_type = match params.rule_type.as_str() {
        "temp_files" => RuleType::TempFiles,
        "large_files" => RuleType::LargeFiles {
            min_size_bytes: params.min_size_mb.unwrap_or(100) * 1_048_576,
        },
        "duplicates" => RuleType::Duplicates,
        "old_downloads" => RuleType::OldDownloads {
            days: params.days_old.unwrap_or(90),
        },
        "empty_folders" => RuleType::EmptyFolders,
        _ => return Err(format!("Unknown rule type: {}", params.rule_type)),
    };

    let rule = CleanRule {
        id: 0,
        name: params.rule_type.clone(),
        description: String::new(),
        rule_type,
        enabled: true,
    };

    let cleaner = Cleaner::new(state.pool.clone(), state.config.read().cleaner.use_trash);
    let result = cleaner.execute(&rule).map_err(|e| e.to_string())?;

    Ok(CleanExecuteResponse {
        files_deleted: result.files_deleted,
        bytes_freed: result.bytes_freed,
        errors: result.errors,
    })
}
