#![warn(clippy::all)]
//! StorageMind Stage 1 Scanner
//!
//! Performs ultra-fast file system traversal to populate the index.
//! Uses rayon for parallel directory scanning.
//! Output is streamed via a flume channel to the database writer.

pub mod error;
pub mod file_classifier;
pub mod scanner;

pub use error::{ScanError, ScanResult};
pub use scanner::{ScanConfig, ScanEvent, Scanner};
