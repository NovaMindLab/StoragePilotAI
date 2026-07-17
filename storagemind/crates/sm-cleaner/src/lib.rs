#![warn(clippy::all)]
//! Cleaner engine — safely removes junk files, duplicates, temp files.
//! Always uses trash (recycle bin) by default to prevent data loss.

pub mod error;
pub mod cleaner;
pub mod rules;

pub use cleaner::Cleaner;
pub use error::{CleanError, CleanResult};
