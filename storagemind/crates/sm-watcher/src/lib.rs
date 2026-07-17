#![warn(clippy::all)]
//! File system watcher — monitors for real-time file changes.
//! Uses the `notify` crate which provides FSEvents on macOS,
//! ReadDirectoryChangesW on Windows, inotify on Linux.

pub mod error;
pub mod watcher;

pub use error::{WatchError, WatchResult};
pub use watcher::{FileWatcher, WatchEvent};
