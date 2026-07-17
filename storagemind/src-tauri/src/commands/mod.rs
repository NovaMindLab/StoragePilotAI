//! Tauri IPC command handlers.
//!
//! All commands follow the pattern:
//! - Receive parameters from Vue frontend (JSON-deserialized)
//! - Access AppState via tauri::State
//! - Return serde-serializable results or error strings

pub mod cleaner;
pub mod drives;
pub mod files;
pub mod scan;
pub mod search;
pub mod settings;
pub mod metadata;
pub mod ai;
