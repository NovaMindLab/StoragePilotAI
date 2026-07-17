use std::path::PathBuf;

use crate::drives::DriveInfo;
use crate::error::PlatformResult;

/// Unified interface for platform-specific storage operations.
///
/// Each supported platform (macOS, Windows, Linux) provides a concrete
/// implementation of this trait. Consumers should program to this trait
/// rather than calling platform-specific code directly.
pub trait StoragePlatform: Send + Sync {
    /// Returns the human-readable name of this platform implementation.
    fn platform_name(&self) -> &'static str;

    /// Lists all available drives/volumes visible to the current user.
    fn list_drives(&self) -> PlatformResult<Vec<DriveInfo>>;

    /// Returns the total, used, and free byte counts for the filesystem
    /// containing `path` as a tuple `(total, used, free)`.
    fn disk_usage(&self, path: &std::path::Path) -> PlatformResult<(u64, u64, u64)>;

    /// Returns the recommended number of scanner worker threads.
    ///
    /// Defaults to `min(available_parallelism, 8).max(2)`, giving a
    /// sensible bound without saturating low-core machines or servers.
    fn recommended_threads(&self) -> usize {
        std::thread::available_parallelism()
            .map(|n| n.get().min(8).max(2))
            .unwrap_or(4)
    }

    /// Whether this platform supports accelerated volume enumeration
    /// (e.g., APFS snapshots on macOS, MFT walk on Windows NTFS).
    fn supports_fast_scan(&self) -> bool {
        false
    }

    /// Returns the OS-level temporary directory path.
    fn temp_dir(&self) -> PathBuf {
        std::env::temp_dir()
    }

    /// Returns the current user's home directory, if determinable.
    fn home_dir(&self) -> Option<PathBuf>;

    /// Returns the current user's Downloads directory, if determinable.
    fn downloads_dir(&self) -> Option<PathBuf>;
}
