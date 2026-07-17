//! Scan-related domain types.
//!
//! Covers the three-stage scan pipeline state machine ([`ScanStatus`],
//! [`ScanStage`]), real-time progress reporting ([`ScanProgress`]), and drive
//! enumeration ([`DriveInfo`]).

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// ScanStatus
// ---------------------------------------------------------------------------

/// Lifecycle state of a scan job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum ScanStatus {
    /// No scan is running.
    #[default]
    Idle,
    /// Actively traversing the filesystem.
    Scanning,
    /// Temporarily suspended; can be resumed.
    Paused,
    /// User-initiated cancellation; cannot be resumed.
    Cancelled,
    /// All requested stages finished successfully.
    Complete,
}

// ---------------------------------------------------------------------------
// ScanStage
// ---------------------------------------------------------------------------

/// Which of the three pipeline stages is currently active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum ScanStage {
    /// Stage 1 — fast metadata walk (`std::fs::read_dir` / `walkdir`).
    #[default]
    Stage1,
    /// Stage 2 — media parsing (EXIF, duration, dimensions, hashing).
    Stage2,
    /// Stage 3 — AI inference (embeddings, classification, OCR).
    Stage3,
}

// ---------------------------------------------------------------------------
// ScanProgress
// ---------------------------------------------------------------------------

/// Real-time progress snapshot emitted by the scanner at regular intervals.
///
/// This struct is serialised and sent to the Vue frontend via the
/// [`crate::event::ScanProgressEvent`] IPC event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    /// Drive root being scanned (e.g. `"/"` or `"C:\"`).
    pub drive: String,

    /// Current lifecycle state.
    pub status: ScanStatus,

    /// Current pipeline stage.
    pub stage: ScanStage,

    /// Number of regular files discovered so far.
    pub files_found: u64,

    /// Number of directories discovered so far.
    pub dirs_found: u64,

    /// Cumulative bytes of file metadata processed.
    pub bytes_scanned: u64,

    /// Absolute path of the entry being processed right now.
    pub current_path: String,

    /// Wall-clock seconds since the scan started.
    pub elapsed_secs: f64,

    /// Throughput: files processed per second (rolling average).
    pub files_per_sec: f64,
}

// ---------------------------------------------------------------------------
// DriveInfo
// ---------------------------------------------------------------------------

/// Information about a mounted drive / volume.
///
/// Populated by the platform layer (`sm-platform`) and stored in the index
/// so the frontend can display storage-usage summaries without hitting the OS
/// each time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DriveInfo {
    /// Mount point path (e.g. `"/"`, `"/Volumes/MyDrive"`, `"D:\"`).
    pub path: String,

    /// Human-readable volume label, if the OS provides one.
    pub label: Option<String>,

    /// Total capacity of the volume in bytes.
    pub total_bytes: u64,

    /// Bytes currently used.
    pub used_bytes: u64,

    /// Bytes currently free.
    pub free_bytes: u64,

    /// Filesystem type string (e.g. `"apfs"`, `"ntfs"`, `"ext4"`).
    pub fs_type: String,

    /// Whether this is a removable / external drive.
    pub is_removable: bool,
}

impl DriveInfo {
    /// Returns the used-space ratio in the range `[0.0, 1.0]`.
    pub fn usage_ratio(&self) -> f64 {
        if self.total_bytes == 0 {
            0.0
        } else {
            self.used_bytes as f64 / self.total_bytes as f64
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_progress_default() {
        let p = ScanProgress::default();
        assert_eq!(p.status, ScanStatus::Idle);
        assert_eq!(p.stage, ScanStage::Stage1);
        assert_eq!(p.files_found, 0);
    }

    #[test]
    fn scan_status_serializes_camel_case() {
        let s = ScanStatus::Scanning;
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(json, "\"scanning\"");
    }

    #[test]
    fn drive_info_usage_ratio() {
        let d = DriveInfo {
            total_bytes: 1000,
            used_bytes: 250,
            ..Default::default()
        };
        assert!((d.usage_ratio() - 0.25).abs() < f64::EPSILON);
    }

    #[test]
    fn drive_info_usage_ratio_zero_total() {
        let d = DriveInfo::default();
        assert_eq!(d.usage_ratio(), 0.0);
    }
}
