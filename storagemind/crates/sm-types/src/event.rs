//! IPC event types sent from the Rust backend to the Vue 3 frontend.
//!
//! All events are serialised with a `type` discriminant tag so the frontend
//! can switch on the event kind without an extra wrapper.
//!
//! ```json
//! { "type": "scanProgress", "data": { ... } }
//! ```
//!
//! The [`AppEvent`] enum is the canonical type emitted via `tauri::Emitter`.

use serde::{Deserialize, Serialize};

use crate::scan::ScanProgress;
use crate::task::Task;

// ---------------------------------------------------------------------------
// Individual event payloads
// ---------------------------------------------------------------------------

/// Emitted periodically while a scan is running.
///
/// The frontend uses this to update its progress bar and statistics panel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgressEvent {
    /// Current scan state.
    pub progress: ScanProgress,
}

/// Emitted after a batch of file entries has been committed to the index.
///
/// Allows the frontend to refresh its file list without waiting for the scan
/// to fully complete.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexUpdatedEvent {
    /// Number of entries added or updated in this batch.
    pub count: u64,
}

/// Emitted whenever a background [`Task`] changes state.
///
/// The frontend can use this to update its task-manager panel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskStatusEvent {
    /// Full snapshot of the task at the moment the event was emitted.
    pub task: Task,
}

/// Emitted by the filesystem watcher when it detects a change.
///
/// The frontend can use this to invalidate cached directory listings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatcherEvent {
    /// Absolute path of the file or directory that changed.
    pub path: String,

    /// Change kind: one of `"created"`, `"modified"`, or `"deleted"`.
    pub kind: WatcherEventKind,
}

/// Discriminant for [`WatcherEvent::kind`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WatcherEventKind {
    /// A new file or directory was created.
    Created,
    /// An existing file or directory was modified.
    Modified,
    /// A file or directory was deleted.
    Deleted,
}

// ---------------------------------------------------------------------------
// AppEvent — top-level discriminated union
// ---------------------------------------------------------------------------

/// The top-level event envelope sent from Rust to Vue via Tauri's IPC layer.
///
/// Serialised as a tagged union with `type` as the discriminant field and
/// `data` as the payload, e.g.:
///
/// ```json
/// { "type": "scanProgress", "data": { "progress": { ... } } }
/// ```
///
/// On the Vue side, listen with:
/// ```ts
/// import { listen } from '@tauri-apps/api/event';
/// await listen<AppEvent>('app-event', e => { ... });
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
pub enum AppEvent {
    /// A scan-progress snapshot.
    ScanProgress(ScanProgressEvent),
    /// The Storage Index has been updated.
    IndexUpdated(IndexUpdatedEvent),
    /// A background task changed state.
    TaskStatus(TaskStatusEvent),
    /// The filesystem watcher detected a change.
    Watcher(WatcherEvent),
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::{ScanProgress, ScanStage, ScanStatus};

    #[test]
    fn app_event_tag_roundtrip() {
        let evt = AppEvent::IndexUpdated(IndexUpdatedEvent { count: 42 });
        let json = serde_json::to_string(&evt).unwrap();
        assert!(json.contains("\"type\":\"indexUpdated\""));
        assert!(json.contains("\"count\":42"));

        let back: AppEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(evt, back);
    }

    #[test]
    fn scan_progress_event_roundtrip() {
        let evt = AppEvent::ScanProgress(ScanProgressEvent {
            progress: ScanProgress {
                drive: "/".into(),
                status: ScanStatus::Scanning,
                stage: ScanStage::Stage1,
                files_found: 1000,
                dirs_found: 50,
                bytes_scanned: 1_000_000,
                current_path: "/home/user/docs".into(),
                elapsed_secs: 3.5,
                files_per_sec: 285.7,
            },
        });
        let json = serde_json::to_string(&evt).unwrap();
        assert!(json.contains("\"type\":\"scanProgress\""));
        let back: AppEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(evt, back);
    }

    #[test]
    fn watcher_event_kinds() {
        let kinds = [
            WatcherEventKind::Created,
            WatcherEventKind::Modified,
            WatcherEventKind::Deleted,
        ];
        for kind in kinds {
            let json = serde_json::to_string(&kind).unwrap();
            let back: WatcherEventKind = serde_json::from_str(&json).unwrap();
            assert_eq!(kind, back);
        }
    }
}
