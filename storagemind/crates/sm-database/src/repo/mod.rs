//! Typed repository objects for database access.
//!
//! Each repository corresponds to a logical domain (files, tasks, etc.)
//! and provides a clean API over raw SQL.

pub mod file_repo;
pub mod task_repo;
pub mod search_repo;
pub mod settings_repo;

pub use file_repo::FileRepo;
pub use task_repo::TaskRepo;
pub use search_repo::SearchRepo;
pub use settings_repo::SettingsRepo;
