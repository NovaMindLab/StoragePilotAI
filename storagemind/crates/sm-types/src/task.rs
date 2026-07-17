//! Task / scheduler domain types.
//!
//! The StorageMind scheduler is a priority-based work queue that drives all
//! background processing (scans, hashing, thumbnails, AI).  Every unit of work
//! is represented as a [`Task`].

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// TaskId
// ---------------------------------------------------------------------------

/// Unique identifier for a scheduled task.
pub type TaskId = Uuid;

// ---------------------------------------------------------------------------
// TaskKind
// ---------------------------------------------------------------------------

/// The type of work a [`Task`] performs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskKind {
    /// Stage 1 filesystem metadata scan.
    Scan,
    /// Filesystem watch / change-notification loop.
    Watch,
    /// Content hashing (BLAKE3 + xxHash3).
    Hash,
    /// Thumbnail generation for images and videos.
    Thumbnail,
    /// Optical Character Recognition for documents / images.
    Ocr,
    /// Vector embedding generation for AI search.
    Embedding,
    /// Housekeeping: remove stale index entries, orphaned thumbnails, etc.
    Clean,
}

// ---------------------------------------------------------------------------
// TaskPriority
// ---------------------------------------------------------------------------

/// Numeric priority for the scheduler.
///
/// P0 is the highest priority (runs first); P9 is the lowest.
/// Tasks with equal priority are processed in FIFO order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskPriority {
    /// Critical — blocks other work (e.g. user-initiated immediate scan).
    P0 = 0,
    P1 = 1,
    P2 = 2,
    P3 = 3,
    P4 = 4,
    P5 = 5,
    P6 = 6,
    P7 = 7,
    P8 = 8,
    /// Background-only — runs only when the scheduler is otherwise idle.
    P9 = 9,
}

impl TaskPriority {
    /// Returns the numeric value of the priority level.
    pub fn value(self) -> u8 {
        self as u8
    }
}

impl Default for TaskPriority {
    fn default() -> Self {
        Self::P5
    }
}

// ---------------------------------------------------------------------------
// TaskStatus
// ---------------------------------------------------------------------------

/// Lifecycle state of a [`Task`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskStatus {
    /// Waiting in the priority queue.
    Queued,
    /// Currently being executed by a worker.
    Running,
    /// Execution has been suspended; can be resumed.
    Paused,
    /// Execution has been abandoned; will not resume.
    Cancelled,
    /// Completed successfully.
    Done,
    /// Completed with an error.  The inner string is the error message.
    #[serde(rename = "failed")]
    Failed(String),
}

impl Default for TaskStatus {
    fn default() -> Self {
        Self::Queued
    }
}

// ---------------------------------------------------------------------------
// Task
// ---------------------------------------------------------------------------

/// A unit of work managed by the StorageMind scheduler.
///
/// Tasks are persisted to the database so they survive application restarts.
/// The `payload` field carries task-specific parameters (e.g. the root path
/// for a scan task) encoded as a JSON value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    /// Globally unique identifier.
    pub id: TaskId,

    /// What kind of work this task performs.
    pub kind: TaskKind,

    /// Scheduler priority (lower number = higher priority).
    pub priority: TaskPriority,

    /// Current lifecycle state.
    pub status: TaskStatus,

    /// When this task was submitted to the scheduler.
    pub created_at: DateTime<Utc>,

    /// When execution began, if it has started.
    pub started_at: Option<DateTime<Utc>>,

    /// When execution finished (successfully or with an error).
    pub completed_at: Option<DateTime<Utc>>,

    /// Arbitrary JSON payload carrying task-specific parameters.
    ///
    /// Consumers should deserialise this into a concrete type based on `kind`.
    pub payload: serde_json::Value,

    /// Fractional completion in the range `[0.0, 1.0]`.
    ///
    /// `0.0` = not started; `1.0` = complete.  The scheduler updates this
    /// field as workers report progress.
    pub progress: f32,
}

impl Task {
    /// Creates a new [`Task`] in the [`TaskStatus::Queued`] state.
    pub fn new(kind: TaskKind, priority: TaskPriority, payload: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            kind,
            priority,
            status: TaskStatus::Queued,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            payload,
            progress: 0.0,
        }
    }

    /// Returns `true` if the task is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            TaskStatus::Done | TaskStatus::Cancelled | TaskStatus::Failed(_)
        )
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn priority_ordering() {
        assert!(TaskPriority::P0 < TaskPriority::P9);
        assert!(TaskPriority::P3 < TaskPriority::P5);
    }

    #[test]
    fn priority_value() {
        assert_eq!(TaskPriority::P0.value(), 0);
        assert_eq!(TaskPriority::P9.value(), 9);
    }

    #[test]
    fn task_new_is_queued() {
        let t = Task::new(TaskKind::Scan, TaskPriority::P0, serde_json::json!({"root": "/"}));
        assert_eq!(t.status, TaskStatus::Queued);
        assert!(!t.is_terminal());
        assert_eq!(t.progress, 0.0);
    }

    #[test]
    fn task_terminal_states() {
        let mut t = Task::new(TaskKind::Hash, TaskPriority::P5, serde_json::Value::Null);
        t.status = TaskStatus::Done;
        assert!(t.is_terminal());

        t.status = TaskStatus::Failed("timeout".into());
        assert!(t.is_terminal());

        t.status = TaskStatus::Running;
        assert!(!t.is_terminal());
    }

    #[test]
    fn task_kind_serializes_camel_case() {
        let json = serde_json::to_string(&TaskKind::Thumbnail).unwrap();
        assert_eq!(json, "\"thumbnail\"");
    }

    #[test]
    fn task_status_failed_roundtrip() {
        let s = TaskStatus::Failed("oops".into());
        let json = serde_json::to_string(&s).unwrap();
        let back: TaskStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }
}
