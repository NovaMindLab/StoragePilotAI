//! StorageMind Tauri Application
//!
//! This is the main entry point for the StorageMind desktop application.
//! Architecture follows the StorageMind design principles:
//! - Index First: All data access through IndexEngine
//! - Scheduler First: Single global scheduler
//! - Rust First: All computation in Rust, Vue only renders

#![warn(clippy::all)]
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

pub mod commands;
pub mod state;

use std::sync::Arc;

use tauri::Manager;
use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};

use sm_config::AppConfig;
use sm_database::connection::create_pool;
use sm_index::IndexEngine;
use sm_scheduler::{Scheduler, scheduler::SchedulerConfig};

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize tracing
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,storagemind=debug")),
        )
        .with_target(true)
        .init();

    info!("StorageMind starting up...");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Load configuration
            let config = AppConfig::load().unwrap_or_default();
            info!("Configuration loaded: data_dir={:?}", config.general.data_dir);

            // Initialize database
            let db_path = config.db_full_path();
            let pool = create_pool(&db_path)
                .expect("Failed to initialize database");
            info!("Database initialized at {:?}", db_path);

            // Initialize Index Engine
            let index = Arc::new(IndexEngine::new(pool.clone()));

            // Initialize Scheduler
            let sched_config = SchedulerConfig {
                max_concurrent: config.scheduler.max_concurrent_tasks,
                paused: false,
            };
            let scheduler = Scheduler::new(sched_config);
            
            // Start Stage 2 (Metadata) Worker
            tauri::async_runtime::spawn({
                let scheduler = scheduler.clone();
                let pool = pool.clone();
                async move {
                    sm_metadata::worker::spawn_stage2_worker(scheduler, pool);
                }
            });

            // Start Stage 3 (AI) Worker
            // Determine model directory
            let model_dir = config.ai.model_dir.clone();
            
            let ai_engine_opt = if config.ai.enabled {
                let engine_res = tauri::async_runtime::block_on(async {
                    sm_ai::engine::MobileClipEngine::new(&model_dir).await
                });
                match engine_res {
                    Ok(engine) => {
                        let engine = std::sync::Arc::new(engine);
                        tauri::async_runtime::spawn({
                            let scheduler = scheduler.clone();
                            let pool = pool.clone();
                            let engine = engine.clone();
                            async move {
                                sm_ai::worker::spawn_stage3_worker(scheduler, pool, engine);
                            }
                        });
                        Some(engine)
                    }
                    Err(e) => {
                        tracing::error!("Failed to initialize AI Engine: {}", e);
                        None
                    }
                }
            } else {
                None
            };

            // Store app state
            app.manage(AppState {
                config: Arc::new(parking_lot::RwLock::new(config)),
                pool,
                index,
                scheduler,
                ai_engine: ai_engine_opt,
            });

            info!("StorageMind initialized successfully");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Scan commands
            commands::scan::cmd_start_scan,
            commands::scan::cmd_cancel_scan,
            commands::scan::cmd_get_scan_status,
            // File commands
            commands::files::cmd_list_children,
            commands::files::cmd_get_largest_files,
            commands::files::cmd_get_category_stats,
            commands::files::cmd_get_total_stats,
            // Search commands
            commands::search::cmd_search_files,
            // Drives commands
            commands::drives::cmd_list_drives,
            // Settings commands
            commands::settings::cmd_get_settings,
            commands::settings::cmd_save_settings,
            // Cleaner commands
            commands::cleaner::cmd_preview_clean,
            commands::cleaner::cmd_execute_clean,
            commands::metadata::cmd_get_duplicates,
            commands::metadata::cmd_get_file_metadata,
            commands::ai::cmd_search_semantic
        ])
        .run(tauri::generate_context!())
        .expect("Error while running StorageMind application");
}
