#![warn(clippy::all)]
//! Global task scheduler for StorageMind.
//!
//! Architecture:
//! - Single global scheduler (singleton per application)
//! - Priority-based task queue (P0=highest, P9=lowest)
//! - Configurable concurrency limits
//! - Pause/Resume/Cancel support
//! - CPU usage awareness (stub, real impl uses sysinfo)

pub mod error;
pub mod priority;
pub mod scheduler;
pub mod task_handle;

pub use error::{SchedulerError, SchedulerResult};
pub use priority::Priority;
pub use scheduler::{Scheduler, SchedulerConfig};
pub use task_handle::TaskHandle;
