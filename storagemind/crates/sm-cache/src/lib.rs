#![warn(clippy::all)]
//! In-memory LRU cache for expensive computations.
//! Used to cache: folder statistics, treemap data, search results.

pub mod timed_cache;

pub use timed_cache::TimedCache;
