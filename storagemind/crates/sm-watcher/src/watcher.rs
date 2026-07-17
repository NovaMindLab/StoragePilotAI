use std::path::PathBuf;
use notify::{Watcher, RecursiveMode, RecommendedWatcher, EventKind};
use flume::{Receiver};
use serde::{Serialize, Deserialize};
use tracing::{info, warn};

use crate::error::{WatchError, WatchResult};

/// A file-system event emitted by the watcher.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchEvent {
    /// Absolute path that changed.
    pub path: String,
    /// One of: "created" | "modified" | "deleted" | "other"
    pub kind: String,
}

/// Wraps a `notify` watcher and exposes a channel of [`WatchEvent`]s.
pub struct FileWatcher {
    /// Keep the watcher alive — dropping it stops the OS watch.
    _watcher: RecommendedWatcher,
    event_rx: Receiver<WatchEvent>,
}

impl FileWatcher {
    /// Create a new watcher for the given paths (recursive).
    pub fn new(paths: Vec<PathBuf>) -> WatchResult<Self> {
        let (tx, rx) = flume::unbounded::<WatchEvent>();

        let mut watcher =
            notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                match res {
                    Ok(event) => {
                        let kind_str = match event.kind {
                            EventKind::Create(_) => "created",
                            EventKind::Modify(_) => "modified",
                            EventKind::Remove(_) => "deleted",
                            // Ignore pure access events — too noisy.
                            EventKind::Access(_) => return,
                            _ => "other",
                        };
                        for path in event.paths {
                            let _ = tx.send(WatchEvent {
                                path: path.to_string_lossy().to_string(),
                                kind: kind_str.to_string(),
                            });
                        }
                    }
                    Err(e) => warn!("Watch error: {}", e),
                }
            })?;

        for path in paths {
            info!("Watching {:?}", path);
            watcher.watch(&path, RecursiveMode::Recursive)?;
        }

        Ok(Self {
            _watcher: watcher,
            event_rx: rx,
        })
    }

    /// Borrow the raw receiver for use in `select!` or async loops.
    pub fn receiver(&self) -> &Receiver<WatchEvent> {
        &self.event_rx
    }

    /// Non-blocking poll — returns `None` if no event is pending.
    pub fn try_recv(&self) -> Option<WatchEvent> {
        self.event_rx.try_recv().ok()
    }
}
