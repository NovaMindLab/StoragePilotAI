//! Drive listing commands.

use tauri::State;

use sm_platform::drives::DriveInfo;
use crate::state::AppState;

/// List all available drives on this system.
#[tauri::command]
pub async fn cmd_list_drives(_state: State<'_, AppState>) -> Result<Vec<DriveInfo>, String> {
    sm_platform::list_drives().map_err(|e| e.to_string())
}
