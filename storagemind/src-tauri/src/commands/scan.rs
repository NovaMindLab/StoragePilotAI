//! File system scan commands.

use flume::unbounded;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, State, Window};
use tracing::{error, info};

use sm_scanner::scanner::{Scanner, ScanConfig, ScanEvent};
use sm_scheduler::priority::Priority;

use crate::state::AppState;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartScanParams {
    pub path: String,
    pub include_hidden: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanStarted {
    pub scan_id: String,
}

/// Start a new scan. Emits "scan:progress" and "scan:completed" events to the window.
#[tauri::command]
pub async fn cmd_start_scan(
    params: StartScanParams,
    state: State<'_, AppState>,
    window: Window,
) -> Result<ScanStarted, String> {
    let path = std::path::PathBuf::from(&params.path);
    if !path.exists() {
        return Err(format!("Path does not exist: {}", params.path));
    }

    let config = ScanConfig {
        root_path: path,
        include_hidden: params.include_hidden,
        batch_size: 500,
        ..Default::default()
    };

    let pool = state.pool.clone();
    let (event_tx, event_rx) = unbounded::<ScanEvent>();

    let scanner = Scanner::new(config, pool, event_tx);
    let scan_id = scanner.scan_id().to_string();

    // Emit scan events to frontend via Tauri events
    let window_clone = window.clone();
    let scan_id_clone = scan_id.clone();
    tokio::spawn(async move {
        while let Ok(event) = event_rx.recv_async().await {
            let event_name = match &event {
                ScanEvent::Started { .. } => "scan:started",
                ScanEvent::Progress { .. } => "scan:progress",
                ScanEvent::Completed { .. } => "scan:completed",
                ScanEvent::Cancelled { .. } => "scan:cancelled",
                ScanEvent::Error { .. } => "scan:error",
            };
            if let Err(e) = window_clone.emit(event_name, &event) {
                error!("Failed to emit event {}: {}", event_name, e);
            }
        }
    });

    // Run scanner in a blocking thread (it uses walkdir which is sync)
    let scheduler = state.scheduler.clone();
    scheduler
        .spawn(Priority::Scanner, move |cancel_token| async move {
            let result = tokio::task::spawn_blocking(move || scanner.run()).await;
            match result {
                Ok(Ok(())) => info!("Scan completed successfully"),
                Ok(Err(e)) => error!("Scan error: {}", e),
                Err(e) => error!("Scan task panicked: {}", e),
            }
        })
        .map_err(|e| e.to_string())?;

    Ok(ScanStarted { scan_id })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelScanParams {
    pub scan_id: String,
}

/// Cancel a running scan.
#[tauri::command]
pub async fn cmd_cancel_scan(
    params: CancelScanParams,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // In a full implementation, we'd track active scanners by scan_id
    info!("Cancel scan requested for: {}", params.scan_id);
    Ok(())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanStatus {
    pub is_scanning: bool,
    pub active_tasks: usize,
    pub is_paused: bool,
}

/// Get current scan/scheduler status.
#[tauri::command]
pub async fn cmd_get_scan_status(state: State<'_, AppState>) -> Result<ScanStatus, String> {
    Ok(ScanStatus {
        is_scanning: state.scheduler.active_count() > 0,
        active_tasks: state.scheduler.active_count(),
        is_paused: state.scheduler.is_paused(),
    })
}
