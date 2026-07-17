//! # sm-types
//!
//! Shared data types for the entire StorageMind workspace.
//!
//! This crate is the single source of truth for all domain types and must NOT
//! depend on any other internal crates.

#![warn(clippy::all)]

pub mod error;
pub mod event;
pub mod file;
pub mod scan;
pub mod search;
pub mod task;
