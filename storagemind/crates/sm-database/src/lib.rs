//! StorageMind Database Layer
//!
//! Provides a connection pool, schema migrations, and typed repository
//! objects for all database operations.
//!
//! Architecture:
//! - Single SQLite file in WAL mode for maximum read concurrency
//! - FTS5 virtual table for full-text search
//! - r2d2 connection pool for async-safe access
//! - All write operations go through typed repositories

#![warn(clippy::all)]

pub mod connection;
pub mod error;
pub mod migrate;
pub mod repo;
pub mod schema;

pub use connection::DbPool;
pub use error::{DbError, DbResult};
