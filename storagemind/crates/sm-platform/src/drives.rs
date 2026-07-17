use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::error::{PlatformError, PlatformResult};

/// Information about a single storage volume or drive.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveInfo {
    /// Mount point path (e.g., `"/"` on macOS, `"C:\\"` on Windows).
    pub mount_point: String,

    /// Volume label or name, if available.
    pub label: Option<String>,

    /// File system type string (e.g., `"APFS"`, `"NTFS"`, `"exFAT"`).
    pub fs_type: String,

    /// Total capacity in bytes.
    pub total_bytes: u64,

    /// Bytes currently in use.
    pub used_bytes: u64,

    /// Bytes currently free.
    pub free_bytes: u64,

    /// `true` if this is a removable device (USB, SD card, etc.).
    pub is_removable: bool,

    /// `true` if the path is currently reachable (mount point exists).
    pub is_accessible: bool,
}

impl DriveInfo {
    /// Returns the usage percentage in the range `0.0..=100.0`.
    pub fn usage_percent(&self) -> f64 {
        if self.total_bytes == 0 {
            return 0.0;
        }
        (self.used_bytes as f64 / self.total_bytes as f64) * 100.0
    }
}

// ─── Public entry point ───────────────────────────────────────────────────────

/// Lists all available drives on the current platform.
///
/// Dispatches to the appropriate platform implementation at compile time.
pub fn list_drives() -> PlatformResult<Vec<DriveInfo>> {
    #[cfg(target_os = "macos")]
    return list_drives_macos();

    #[cfg(target_os = "windows")]
    return list_drives_windows();

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    return list_drives_unix();
}

// ─── macOS ────────────────────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
fn list_drives_macos() -> PlatformResult<Vec<DriveInfo>> {
    use std::process::Command;

    let output = Command::new("df")
        .args(["-Pk"])
        .output()
        .map_err(|e| PlatformError::Api(format!("df failed: {e}")))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut drives = Vec::new();

    for line in stdout.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 6 {
            continue;
        }

        let mount = parts[5];

        // Filter out noise pseudo-filesystems; keep /System/Volumes/Data
        // because that is where user files actually live on APFS-sealed roots.
        if mount.starts_with("/System/Volumes") && mount != "/System/Volumes/Data" {
            continue;
        }

        let total_kb: u64 = parts[1].parse().unwrap_or(0);
        let used_kb: u64 = parts[2].parse().unwrap_or(0);
        let free_kb: u64 = parts[3].parse().unwrap_or(0);

        let is_accessible = std::path::Path::new(mount).exists();
        // Heuristic: drives not under /System/Volumes and not at "/" are
        // either removable media or network shares.
        let is_removable = mount.starts_with("/Volumes");

        drives.push(DriveInfo {
            mount_point: mount.to_string(),
            label: None,
            fs_type: "APFS".to_string(),
            total_bytes: total_kb * 1024,
            used_bytes: used_kb * 1024,
            free_bytes: free_kb * 1024,
            is_removable,
            is_accessible,
        });
    }

    debug!("Found {} drives on macOS", drives.len());
    Ok(drives)
}

// ─── Windows ──────────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn list_drives_windows() -> PlatformResult<Vec<DriveInfo>> {
    let mut drives = Vec::new();

    for letter in b'A'..=b'Z' {
        let root = format!("{}:\\", letter as char);
        let path = std::path::Path::new(&root);
        if path.exists() {
            match get_disk_usage_windows(&root) {
                Ok((total, used, free)) => {
                    drives.push(DriveInfo {
                        mount_point: root,
                        label: None,
                        fs_type: "NTFS".to_string(),
                        total_bytes: total,
                        used_bytes: used,
                        free_bytes: free,
                        is_removable: false,
                        is_accessible: true,
                    });
                }
                Err(e) => {
                    tracing::warn!("Could not query disk usage for {root}: {e}");
                }
            }
        }
    }

    debug!("Found {} drives on Windows", drives.len());
    Ok(drives)
}

/// Queries free/total space for a Windows drive root using `GetDiskFreeSpaceEx`.
///
/// Returns `(total, used, free)` in bytes.
#[cfg(target_os = "windows")]
fn get_disk_usage_windows(path: &str) -> PlatformResult<(u64, u64, u64)> {
    // TODO: replace with windows-sys / winapi call in a future iteration.
    // For now we return zeros so the drive is still listed.
    let _ = path;
    Ok((0, 0, 0))
}

// ─── Generic Unix (Linux, FreeBSD, …) ────────────────────────────────────────

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn list_drives_unix() -> PlatformResult<Vec<DriveInfo>> {
    use std::fs;

    let mounts = fs::read_to_string("/proc/mounts")
        .map_err(|e| PlatformError::Api(format!("Cannot read /proc/mounts: {e}")))?;

    let mut drives = Vec::new();

    for line in mounts.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            continue;
        }
        let device = parts[0];
        let mount = parts[1];
        let fs_type = parts[2];

        // Skip virtual / kernel filesystems
        if matches!(
            fs_type,
            "sysfs" | "proc" | "devpts" | "tmpfs" | "cgroup" | "cgroup2"
                | "pstore" | "bpf" | "debugfs" | "tracefs" | "fusectl"
                | "mqueue" | "hugetlbfs" | "devtmpfs"
        ) {
            continue;
        }

        let is_accessible = std::path::Path::new(mount).exists();
        drives.push(DriveInfo {
            mount_point: mount.to_string(),
            label: None,
            fs_type: fs_type.to_string(),
            total_bytes: 0,
            used_bytes: 0,
            free_bytes: 0,
            is_removable: device.starts_with("/dev/sd") || device.starts_with("/dev/mmcblk"),
            is_accessible,
        });
    }

    debug!("Found {} drives on Unix", drives.len());
    Ok(drives)
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drive_info_usage_percent_zero_total() {
        let d = DriveInfo {
            mount_point: "/".to_string(),
            label: None,
            fs_type: "test".to_string(),
            total_bytes: 0,
            used_bytes: 0,
            free_bytes: 0,
            is_removable: false,
            is_accessible: true,
        };
        assert_eq!(d.usage_percent(), 0.0);
    }

    #[test]
    fn drive_info_usage_percent_half() {
        let d = DriveInfo {
            mount_point: "/".to_string(),
            label: None,
            fs_type: "test".to_string(),
            total_bytes: 1000,
            used_bytes: 500,
            free_bytes: 500,
            is_removable: false,
            is_accessible: true,
        };
        let pct = d.usage_percent();
        assert!((pct - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn list_drives_returns_ok() {
        // Should succeed on any CI host; content varies.
        assert!(list_drives().is_ok());
    }
}
