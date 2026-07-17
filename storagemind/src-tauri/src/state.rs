//! Application state shared across all Tauri commands.

use std::sync::Arc;

use parking_lot::RwLock;
use sm_config::AppConfig;
use sm_database::DbPool;
use sm_index::IndexEngine;
use sm_scheduler::Scheduler;

/// Global application state, available to all Tauri commands via `State<AppState>`.
pub struct AppState {
    /// Application configuration
    pub config: Arc<RwLock<AppConfig>>,
    /// Database connection pool
    pub pool: DbPool,
    /// Central index engine (all data access goes through here)
    pub index: Arc<IndexEngine>,
    /// Global task scheduler (single instance)
    pub scheduler: Arc<Scheduler>,
    /// AI engine instance
    pub ai_engine: Option<Arc<sm_ai::engine::MobileClipEngine>>,
}
