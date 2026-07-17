//! Settings commands.

use serde::{Deserialize, Serialize};
use tauri::State;
use crate::state::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsResponse {
    pub theme: String,
    pub show_hidden: bool,
    pub default_view: String,
    pub page_size: usize,
    pub include_hidden_scan: bool,
    pub use_trash: bool,
    pub ai_enabled: bool,
    pub language: String,
}

/// Get current settings for the frontend.
#[tauri::command]
pub async fn cmd_get_settings(state: State<'_, AppState>) -> Result<SettingsResponse, String> {
    let config = state.config.read();
    Ok(SettingsResponse {
        theme: config.ui.theme.clone(),
        show_hidden: config.ui.show_hidden,
        default_view: config.ui.default_view.clone(),
        page_size: config.ui.page_size,
        include_hidden_scan: config.scanner.include_hidden,
        use_trash: config.cleaner.use_trash,
        ai_enabled: config.ai.enabled,
        language: config.general.language.clone(),
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveSettingsParams {
    pub theme: Option<String>,
    pub show_hidden: Option<bool>,
    pub default_view: Option<String>,
    pub page_size: Option<usize>,
    pub include_hidden_scan: Option<bool>,
    pub use_trash: Option<bool>,
    pub ai_enabled: Option<bool>,
    pub language: Option<String>,
}

/// Save settings changes.
#[tauri::command]
pub async fn cmd_save_settings(
    params: SaveSettingsParams,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut config = state.config.write();

    if let Some(theme) = params.theme {
        config.ui.theme = theme;
    }
    if let Some(show_hidden) = params.show_hidden {
        config.ui.show_hidden = show_hidden;
    }
    if let Some(view) = params.default_view {
        config.ui.default_view = view;
    }
    if let Some(ps) = params.page_size {
        config.ui.page_size = ps;
    }
    if let Some(hidden) = params.include_hidden_scan {
        config.scanner.include_hidden = hidden;
    }
    if let Some(trash) = params.use_trash {
        config.cleaner.use_trash = trash;
    }
    if let Some(ai) = params.ai_enabled {
        config.ai.enabled = ai;
    }
    if let Some(lang) = params.language {
        config.general.language = lang;
    }

    config.save().map_err(|e| e.to_string())
}
