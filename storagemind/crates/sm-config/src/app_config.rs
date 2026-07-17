use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::info;

use crate::error::ConfigError;

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AppConfig {
    pub general: GeneralConfig,
    pub scanner: ScannerConfig,
    pub scheduler: SchedulerConfig,
    pub database: DatabaseConfig,
    pub ui: UiConfig,
    pub cleaner: CleanerConfig,
    pub ai: AiConfig,
}

/// General application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// App data directory path
    pub data_dir: PathBuf,
    /// Whether to start on system boot
    pub start_on_boot: bool,
    /// Whether to minimize to tray on close
    pub minimize_to_tray: bool,
    /// Language code (e.g. "en", "zh")
    pub language: String,
}

/// File scanner settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannerConfig {
    /// Drives/paths to scan on startup
    pub auto_scan_paths: Vec<PathBuf>,
    /// Paths to exclude from scanning
    pub exclude_paths: Vec<PathBuf>,
    /// Maximum file size to process (bytes), 0 = unlimited
    pub max_file_size: u64,
    /// Number of scanner worker threads
    pub worker_threads: usize,
    /// Whether to follow symlinks
    pub follow_symlinks: bool,
    /// Whether to include hidden files
    pub include_hidden: bool,
}

/// Task scheduler settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    /// Maximum number of concurrent tasks
    pub max_concurrent_tasks: usize,
    /// CPU usage limit for background tasks (0-100)
    pub cpu_limit_percent: u8,
    /// Whether to pause on battery
    pub pause_on_battery: bool,
    /// Whether to pause when CPU is busy
    pub pause_on_high_cpu: bool,
    /// CPU usage threshold to pause at (0-100)
    pub high_cpu_threshold: u8,
}

/// SQLite database settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database file path (relative to data_dir)
    pub db_path: PathBuf,
    /// WAL checkpoint interval (in seconds)
    pub wal_checkpoint_interval: u64,
    /// Maximum DB cache size (KB)
    pub cache_size_kb: u64,
}

/// UI/presentation settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    /// Theme: "dark" | "light" | "system"
    pub theme: String,
    /// Whether to show file extensions
    pub show_extensions: bool,
    /// Default view: "list" | "grid"
    pub default_view: String,
    /// Number of items per page
    pub page_size: usize,
    /// Whether to show hidden files in UI
    pub show_hidden: bool,
}

/// Storage cleaner settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanerConfig {
    /// Whether to move to trash instead of permanent delete
    pub use_trash: bool,
    /// Whether to require confirmation before cleaning
    pub confirm_before_clean: bool,
    /// Temp file directories to clean
    pub temp_dirs: Vec<PathBuf>,
}

/// AI model and inference settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    /// Whether AI features are enabled
    pub enabled: bool,
    /// ONNX model directory
    pub model_dir: PathBuf,
    /// Whether to run AI on battery power
    pub run_on_battery: bool,
    /// Maximum memory for AI models (MB)
    pub max_memory_mb: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("StorageMind");
        Self {
            general: GeneralConfig {
                data_dir: data_dir.clone(),
                start_on_boot: false,
                minimize_to_tray: true,
                language: "en".to_string(),
            },
            scanner: ScannerConfig {
                auto_scan_paths: vec![],
                exclude_paths: vec![],
                max_file_size: 0,
                worker_threads: num_cpus(),
                follow_symlinks: false,
                include_hidden: false,
            },
            scheduler: SchedulerConfig {
                max_concurrent_tasks: 4,
                cpu_limit_percent: 50,
                pause_on_battery: true,
                pause_on_high_cpu: true,
                high_cpu_threshold: 80,
            },
            database: DatabaseConfig {
                db_path: PathBuf::from("storagemind.db"),
                wal_checkpoint_interval: 60,
                cache_size_kb: 64 * 1024, // 64 MB
            },
            ui: UiConfig {
                theme: "dark".to_string(),
                show_extensions: true,
                default_view: "list".to_string(),
                page_size: 100,
                show_hidden: false,
            },
            cleaner: CleanerConfig {
                use_trash: true,
                confirm_before_clean: true,
                temp_dirs: vec![],
            },
            ai: AiConfig {
                enabled: true,
                model_dir: data_dir.join("models"),
                run_on_battery: false,
                max_memory_mb: 512,
            },
        }
    }
}

/// Returns a sensible default thread count capped at 8.
fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get().min(8))
        .unwrap_or(4)
}

impl AppConfig {
    /// Load config from the default location, creating it with defaults if absent.
    pub fn load() -> Result<Self, ConfigError> {
        let path = Self::config_path()?;
        if path.exists() {
            info!("Loading config from {:?}", path);
            let content = std::fs::read_to_string(&path)?;
            let config: AppConfig = toml::from_str(&content)?;
            Ok(config)
        } else {
            info!("Config not found, creating defaults at {:?}", path);
            let config = AppConfig::default();
            config.save()?;
            Ok(config)
        }
    }

    /// Persist the current config to disk.
    pub fn save(&self) -> Result<(), ConfigError> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        info!("Config saved to {:?}", path);
        Ok(())
    }

    /// Absolute path to the SQLite database file.
    pub fn db_full_path(&self) -> PathBuf {
        self.general.data_dir.join(&self.database.db_path)
    }

    /// Returns the platform-appropriate path for `config.toml`.
    ///
    /// | OS      | Path                                          |
    /// |---------|-----------------------------------------------|
    /// | macOS   | `~/Library/Application Support/StorageMind/`  |
    /// | Windows | `%APPDATA%\StorageMind\`                       |
    /// | Linux   | `~/.config/StorageMind/`                       |
    fn config_path() -> Result<PathBuf, ConfigError> {
        let dir = dirs::config_dir()
            .ok_or(ConfigError::DirNotFound)?
            .join("StorageMind");
        Ok(dir.join("config.toml"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        let config = AppConfig::default();
        assert_eq!(config.ui.theme, "dark");
        assert!(config.scheduler.max_concurrent_tasks > 0);
        assert!(config.scheduler.cpu_limit_percent <= 100);
        assert!(config.scheduler.high_cpu_threshold <= 100);
    }

    #[test]
    fn config_serializes_roundtrip() {
        let config = AppConfig::default();
        let toml_str = toml::to_string_pretty(&config).expect("serialize");
        let deserialized: AppConfig = toml::from_str(&toml_str).expect("deserialize");
        assert_eq!(deserialized.ui.theme, config.ui.theme);
        assert_eq!(deserialized.ui.page_size, config.ui.page_size);
        assert_eq!(
            deserialized.scheduler.cpu_limit_percent,
            config.scheduler.cpu_limit_percent
        );
    }

    #[test]
    fn db_full_path_joins_correctly() {
        let config = AppConfig::default();
        let db_path = config.db_full_path();
        assert!(db_path.ends_with("storagemind.db"));
    }

    #[test]
    fn num_cpus_returns_positive() {
        assert!(num_cpus() > 0);
        assert!(num_cpus() <= 8);
    }
}
