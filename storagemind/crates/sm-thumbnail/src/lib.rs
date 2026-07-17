#![warn(clippy::all)]
//! Thumbnail generator for images.
//! Generates JPEG thumbnails and stores them in the database.

pub mod error;
pub mod generator;

pub use error::{ThumbnailError, ThumbnailResult};
pub use generator::ThumbnailGenerator;
