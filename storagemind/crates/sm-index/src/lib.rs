#![warn(clippy::all)]
//! Index Engine — the central hub that all other modules access.
//! No module is allowed to scan disk directly; they must go through IndexEngine.

pub mod error;
pub mod engine;

pub use engine::IndexEngine;
pub use error::{IndexError, IndexResult};
