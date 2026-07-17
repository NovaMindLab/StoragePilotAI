//! Core scanner implementation for Stage 1 file system traversal.
//!
//! [`Scanner`] walks a directory tree using [`walkdir`], classifies every
//! entry, and writes records to the database in configurable batches.
//! Progress and lifecycle events are emitted via a [`flume`] channel so that
//! callers (e.g. a Tauri command) can stream status updates to the front-end.
//!
//! ## Design notes
//!
//! * **No `Box::leak`** — strings are owned by the batch `Vec` and reborrrowed
//!   for each `InsertFile<'_>`.  This keeps memory bounded.
//! * **Cancellation** — an `AtomicBool` flag lets any thread call
//!   [`Scanner::cancel`] mid-walk.
//! * **Batch writes** — files are grouped into configurable batches before
//!   being flushed to SQLite in a single transaction, maximising throughput.
//! * **Progress throttle** — events are emitted at most once every 500 ms to
//!   avoid flooding the front-end channel.

use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use std::time::Instant;

use flume::Sender;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use uuid::Uuid;
use walkdir::WalkDir;

use sm_database::repo::file_repo::InsertFile;
use sm_database::DbPool;

use crate::error::{ScanError, ScanResult};
use crate::file_classifier::{classify_extension, is_hidden};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for a single scan session.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanConfig {
    /// Root path to scan.
    pub root_path: PathBuf,
    /// Paths that will be skipped entirely (and not descended into).
    pub exclude_paths: Vec<PathBuf>,
    /// Whether to follow symbolic links during traversal.
    pub follow_symlinks: bool,
    /// Whether to include hidden files and directories (names starting with `.`).
    pub include_hidden: bool,
    /// Number of parallel scanner threads. `0` means "use rayon default".
    /// Currently the walker is sequential; this field is reserved for future
    /// parallel-directory-dispatch work.
    pub threads: usize,
    /// How many file entries to accumulate before flushing to the database.
    pub batch_size: usize,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            root_path: PathBuf::from("/"),
            exclude_paths: vec![],
            follow_symlinks: false,
            include_hidden: false,
            threads: 0,
            batch_size: 500,
        }
    }
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/// Events emitted by [`Scanner`] during a scan session.
///
/// Callers should listen on the [`flume::Receiver`] end of the channel
/// passed to [`Scanner::new`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum ScanEvent {
    /// The scan has been initialised and is about to begin walking.
    Started {
        scan_id: String,
        root: String,
    },
    /// Periodic progress snapshot.
    Progress {
        scan_id: String,
        files_found: u64,
        dirs_found: u64,
        bytes_scanned: u64,
        /// The path being processed when this snapshot was taken.
        current_path: String,
        elapsed_secs: f64,
        files_per_sec: f64,
    },
    /// The scan finished normally.
    Completed {
        scan_id: String,
        total_files: u64,
        total_dirs: u64,
        total_bytes: u64,
        elapsed_secs: f64,
    },
    /// The scan was interrupted by [`Scanner::cancel`].
    Cancelled {
        scan_id: String,
    },
    /// A non-fatal error occurred for a specific path; the scan continues.
    Error {
        scan_id: String,
        path: String,
        message: String,
    },
}

// ---------------------------------------------------------------------------
// Owned batch entry (avoids Box::leak)
// ---------------------------------------------------------------------------

/// Fully-owned strings for a single file entry collected during a scan pass.
///
/// We keep these alive for the duration of the batch so that we can form
/// `InsertFile<'_>` borrows without leaking heap memory.
struct BatchEntry {
    path: String,
    name: String,
    extension: Option<String>,
    size: i64,
    kind: &'static str,
    category: &'static str,
    parent_id: Option<i64>,
    depth: i64,
    inode: Option<i64>,
    is_hidden: bool,
    created_at: Option<i64>,
    modified_at: Option<i64>,
    accessed_at: Option<i64>,
    scan_id: String,
}

impl BatchEntry {
    /// Borrow this entry as an [`InsertFile`] whose lifetime is tied to `self`.
    fn as_insert_file(&self) -> InsertFile<'_> {
        InsertFile {
            path: &self.path,
            name: &self.name,
            extension: self.extension.as_deref(),
            size: self.size,
            kind: self.kind,
            category: self.category,
            parent_id: self.parent_id,
            depth: self.depth,
            inode: self.inode,
            is_hidden: self.is_hidden,
            created_at: self.created_at,
            modified_at: self.modified_at,
            accessed_at: self.accessed_at,
            scan_id: Some(&self.scan_id),
        }
    }
}

// ---------------------------------------------------------------------------
// Scanner
// ---------------------------------------------------------------------------

/// Stage 1 file-system scanner.
///
/// Create one instance per scan session with [`Scanner::new`], then call
/// [`Scanner::run`] from a `tokio::task::spawn_blocking` closure (it is
/// synchronous / CPU-bound).
///
/// # Example
/// ```no_run
/// use sm_scanner::{Scanner, ScanConfig};
/// use sm_database::connection::create_pool;
/// use std::path::PathBuf;
///
/// let pool = create_pool(&PathBuf::from("/tmp/test.db")).unwrap();
/// let (tx, rx) = flume::unbounded();
/// let config = ScanConfig {
///     root_path: PathBuf::from("/Users"),
///     ..Default::default()
/// };
/// let scanner = Scanner::new(config, pool, tx);
/// // In a real app: tokio::task::spawn_blocking(move || scanner.run()).await
/// ```
pub struct Scanner {
    config: ScanConfig,
    scan_id: String,
    cancelled: Arc<AtomicBool>,
    files_found: Arc<AtomicU64>,
    dirs_found: Arc<AtomicU64>,
    bytes_scanned: Arc<AtomicU64>,
    event_tx: Sender<ScanEvent>,
    db_pool: DbPool,
}

impl Scanner {
    /// Create a new scanner for the given configuration.
    ///
    /// A fresh UUID is assigned as the `scan_id` for this session.
    pub fn new(config: ScanConfig, db_pool: DbPool, event_tx: Sender<ScanEvent>) -> Self {
        let scan_id = Uuid::new_v4().to_string();
        Self {
            config,
            scan_id,
            cancelled: Arc::new(AtomicBool::new(false)),
            files_found: Arc::new(AtomicU64::new(0)),
            dirs_found: Arc::new(AtomicU64::new(0)),
            bytes_scanned: Arc::new(AtomicU64::new(0)),
            event_tx,
            db_pool,
        }
    }

    /// Signal the scan to stop at the next entry boundary.
    ///
    /// This is safe to call from any thread.  The scan will return
    /// [`ScanError::Cancelled`] after the current entry is processed.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    /// The unique identifier for this scan session.
    pub fn scan_id(&self) -> &str {
        &self.scan_id
    }

    /// Run the scan synchronously.
    ///
    /// This method blocks the calling thread for the duration of the walk.
    /// Always call it from `tokio::task::spawn_blocking` to avoid starving
    /// the async runtime.
    ///
    /// Returns `Ok(())` on successful completion, or a [`ScanError`] if the
    /// scan was cancelled or encountered a fatal error.
    pub fn run(&self) -> ScanResult<()> {
        let start = Instant::now();
        let root = &self.config.root_path;

        info!("Starting scan [{}] at {:?}", self.scan_id, root);
        self.send_event(ScanEvent::Started {
            scan_id: self.scan_id.clone(),
            root: root.to_string_lossy().to_string(),
        });

        let file_repo = sm_database::repo::FileRepo::new(self.db_pool.clone());
        let mut batch: Vec<BatchEntry> = Vec::with_capacity(self.config.batch_size);
        let mut last_progress = Instant::now();

        let walker = WalkDir::new(root)
            .follow_links(self.config.follow_symlinks)
            .into_iter()
            .filter_entry(|e| {
                let path = e.path();
                // Skip excluded paths.
                for excl in &self.config.exclude_paths {
                    if path.starts_with(excl) {
                        return false;
                    }
                }
                // Skip hidden entries when requested.
                if !self.config.include_hidden {
                    if let Some(name) = path.file_name() {
                        if is_hidden(&name.to_string_lossy()) {
                            return false;
                        }
                    }
                }
                true
            });

        for result in walker {
            // Check cancellation before processing each entry.
            if self.cancelled.load(Ordering::Relaxed) {
                self.send_event(ScanEvent::Cancelled {
                    scan_id: self.scan_id.clone(),
                });
                return Err(ScanError::Cancelled);
            }

            let entry = match result {
                Ok(e) => e,
                Err(e) => {
                    let path = e
                        .path()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_default();
                    warn!("Walk error at {}: {}", path, e);
                    self.send_event(ScanEvent::Error {
                        scan_id: self.scan_id.clone(),
                        path,
                        message: e.to_string(),
                    });
                    continue;
                }
            };

            let path = entry.path();
            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(e) => {
                    warn!("Cannot read metadata for {:?}: {}", path, e);
                    continue;
                }
            };

            let kind: &'static str = if metadata.is_dir() {
                "directory"
            } else if metadata.is_symlink() {
                "symlink"
            } else {
                "regular"
            };

            let name = entry.file_name().to_string_lossy().to_string();
            let extension = if metadata.is_file() {
                path.extension()
                    .map(|e| e.to_string_lossy().into_owned())
            } else {
                None
            };
            let category: &'static str = match extension.as_deref() {
                Some(ext) => classify_extension(ext),
                None => "other",
            };
            let size: i64 = if metadata.is_file() {
                metadata.len() as i64
            } else {
                0
            };
            let depth = entry.depth() as i64;

            let modified_at = metadata.modified().ok().map(|t| {
                t.duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as i64
            });
            let created_at = metadata.created().ok().map(|t| {
                t.duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as i64
            });

            let entry_is_hidden = is_hidden(&name);

            batch.push(BatchEntry {
                path: path.to_string_lossy().into_owned(),
                name,
                extension,
                size,
                kind,
                category,
                parent_id: None, // TODO: resolve parent ID via path lookup
                depth,
                inode: None,
                is_hidden: entry_is_hidden,
                created_at,
                modified_at,
                accessed_at: None,
                scan_id: self.scan_id.clone(),
            });

            // Update counters.
            if kind == "directory" {
                self.dirs_found.fetch_add(1, Ordering::Relaxed);
            } else {
                self.files_found.fetch_add(1, Ordering::Relaxed);
                self.bytes_scanned
                    .fetch_add(size as u64, Ordering::Relaxed);
            }

            // Flush batch to DB when full.
            if batch.len() >= self.config.batch_size {
                self.flush_batch(&file_repo, &mut batch);
            }

            // Emit a progress event at most once every 500 ms.
            if last_progress.elapsed().as_millis() >= 500 {
                let elapsed = start.elapsed().as_secs_f64();
                let files = self.files_found.load(Ordering::Relaxed);
                let fps = if elapsed > 0.0 {
                    files as f64 / elapsed
                } else {
                    0.0
                };
                self.send_event(ScanEvent::Progress {
                    scan_id: self.scan_id.clone(),
                    files_found: files,
                    dirs_found: self.dirs_found.load(Ordering::Relaxed),
                    bytes_scanned: self.bytes_scanned.load(Ordering::Relaxed),
                    current_path: path.to_string_lossy().to_string(),
                    elapsed_secs: elapsed,
                    files_per_sec: fps,
                });
                last_progress = Instant::now();
            }
        }

        // Flush whatever remains in the final partial batch.
        if !batch.is_empty() {
            self.flush_batch(&file_repo, &mut batch);
        }

        let elapsed = start.elapsed().as_secs_f64();
        let total_files = self.files_found.load(Ordering::Relaxed);
        let total_dirs = self.dirs_found.load(Ordering::Relaxed);
        let total_bytes = self.bytes_scanned.load(Ordering::Relaxed);

        info!(
            "Scan [{}] complete: {} files, {} dirs, {:.2} MB in {:.1}s ({:.0} files/s)",
            self.scan_id,
            total_files,
            total_dirs,
            total_bytes as f64 / 1_048_576.0,
            elapsed,
            if elapsed > 0.0 {
                total_files as f64 / elapsed
            } else {
                0.0
            },
        );

        self.send_event(ScanEvent::Completed {
            scan_id: self.scan_id.clone(),
            total_files,
            total_dirs,
            total_bytes,
            elapsed_secs: elapsed,
        });

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Flush `batch` to the database and clear it.
    fn flush_batch(
        &self,
        file_repo: &sm_database::repo::FileRepo,
        batch: &mut Vec<BatchEntry>,
    ) {
        let inserts: Vec<InsertFile<'_>> = batch.iter().map(BatchEntry::as_insert_file).collect();
        if let Err(e) = file_repo.bulk_upsert(&inserts) {
            warn!("Batch write error ({} entries): {}", inserts.len(), e);
        }
        batch.clear();
    }

    /// Send an event on the channel, logging a warning on failure.
    fn send_event(&self, event: ScanEvent) {
        if self.event_tx.send(event).is_err() {
            warn!("scan [{}]: event channel closed", self.scan_id);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use sm_database::connection::create_pool;
    use tempfile::tempdir;

    fn make_pool() -> DbPool {
        let dir = tempdir().unwrap();
        create_pool(&dir.path().join("test.db")).unwrap()
    }

    #[test]
    fn default_config_is_sane() {
        let cfg = ScanConfig::default();
        assert_eq!(cfg.batch_size, 500);
        assert!(!cfg.follow_symlinks);
        assert!(!cfg.include_hidden);
    }

    #[test]
    fn scan_id_is_unique() {
        let pool = make_pool();
        let (tx, _rx) = flume::unbounded();
        let cfg = ScanConfig {
            root_path: PathBuf::from("/tmp"),
            ..Default::default()
        };
        let s1 = Scanner::new(cfg.clone(), pool.clone(), tx.clone());
        let s2 = Scanner::new(cfg, pool, tx);
        assert_ne!(s1.scan_id(), s2.scan_id());
    }

    #[test]
    fn cancel_stops_scan() {
        let pool = make_pool();
        let (tx, rx) = flume::unbounded();
        let cfg = ScanConfig {
            root_path: PathBuf::from("/usr"),
            batch_size: 50,
            ..Default::default()
        };
        let scanner = Scanner::new(cfg, pool, tx);
        scanner.cancel(); // cancel before run
        let result = scanner.run();
        assert!(matches!(result, Err(ScanError::Cancelled)));
        // Should have emitted a Cancelled event.
        let events: Vec<ScanEvent> = rx.drain().collect();
        assert!(events.iter().any(|e| matches!(e, ScanEvent::Cancelled { .. })));
    }

    #[test]
    fn scan_tmp_emits_completed() {
        let pool = make_pool();
        let (tx, rx) = flume::unbounded();
        let cfg = ScanConfig {
            root_path: std::env::temp_dir(),
            batch_size: 50,
            include_hidden: false,
            ..Default::default()
        };
        let scanner = Scanner::new(cfg, pool, tx);
        let result = scanner.run();
        assert!(result.is_ok(), "scan failed: {:?}", result);

        let events: Vec<ScanEvent> = rx.drain().collect();
        assert!(events.iter().any(|e| matches!(e, ScanEvent::Started { .. })));
        assert!(events
            .iter()
            .any(|e| matches!(e, ScanEvent::Completed { .. })));
    }
}
