//! `sm-config` — Application configuration management for StorageMind.
//!
//! Handles loading, saving, and defaulting of TOML-based config stored
//! in the platform's standard config directory.

#![warn(clippy::all)]

pub mod app_config;
pub mod error;

pub use app_config::AppConfig;
pub use error::ConfigError;

pub type ConfigResult<T> = Result<T, ConfigError>;
