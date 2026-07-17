#![warn(clippy::all)]

//! `sm-platform` — platform abstraction layer for StorageMind.
//!
//! Provides the [`StoragePlatform`] trait and platform-specific implementations
//! for macOS, Windows, and generic Unix systems.

pub mod drives;
pub mod error;
pub mod traits;

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;

// Re-export commonly used items
pub use drives::{list_drives, DriveInfo};
pub use error::{PlatformError, PlatformResult};
pub use traits::StoragePlatform;
