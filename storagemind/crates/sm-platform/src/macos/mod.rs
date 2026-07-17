//! macOS-specific platform implementation.
//!
//! Exposes [`MacOsPlatform`] which implements the [`StoragePlatform`] trait
//! using native macOS tooling (`df`, `dirs`).

use std::path::PathBuf;

use crate::drives::DriveInfo;
use crate::error::{PlatformError, PlatformResult};
use crate::traits::StoragePlatform;

/// Concrete [`StoragePlatform`] implementation for macOS.
pub struct MacOsPlatform;

impl StoragePlatform for MacOsPlatform {
    fn platform_name(&self) -> &'static str {
        "macOS"
    }

    fn list_drives(&self) -> PlatformResult<Vec<DriveInfo>> {
        crate::drives::list_drives()
    }

    /// Queries disk usage for the filesystem containing `path`.
    ///
    /// Shells out to `df -Pk <path>` and parses the 1-KiB-block output.
    /// Returns `(total, used, free)` in bytes.
    fn disk_usage(&self, path: &std::path::Path) -> PlatformResult<(u64, u64, u64)> {
        use std::process::Command;

        let output = Command::new("df")
            .args(["-Pk", path.to_str().unwrap_or("/")])
            .output()
            .map_err(|e| PlatformError::Api(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let total = parts[1].parse::<u64>().unwrap_or(0) * 1024;
                let used = parts[2].parse::<u64>().unwrap_or(0) * 1024;
                let free = parts[3].parse::<u64>().unwrap_or(0) * 1024;
                return Ok((total, used, free));
            }
        }

        // Path exists but df returned no data — return zeroes rather than error.
        Ok((0, 0, 0))
    }

    /// macOS (APFS) supports fast volume enumeration.
    fn supports_fast_scan(&self) -> bool {
        true
    }

    fn home_dir(&self) -> Option<PathBuf> {
        dirs::home_dir()
    }

    fn downloads_dir(&self) -> Option<PathBuf> {
        dirs::download_dir()
    }
}

/// Construct the default macOS platform handle.
pub fn current_platform() -> MacOsPlatform {
    MacOsPlatform
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_name() {
        assert_eq!(MacOsPlatform.platform_name(), "macOS");
    }

    #[test]
    fn supports_fast_scan() {
        assert!(MacOsPlatform.supports_fast_scan());
    }

    #[test]
    fn disk_usage_root() {
        let (total, used, free) = MacOsPlatform
            .disk_usage(std::path::Path::new("/"))
            .expect("disk_usage('/') should succeed on macOS");
        // Sanity: total must be > 0 and used + free ≈ total
        assert!(total > 0);
        assert!(used + free <= total + 1024); // allow 1-block rounding
    }

    #[test]
    fn home_dir_is_some() {
        assert!(MacOsPlatform.home_dir().is_some());
    }

    #[test]
    fn recommended_threads_range() {
        let t = MacOsPlatform.recommended_threads();
        assert!((2..=8).contains(&t));
    }
}
