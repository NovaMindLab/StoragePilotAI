pub mod audio;
pub mod error;
pub mod hash;
pub mod image;
pub mod processor;
pub mod worker;

pub use error::{MetadataError, MetadataResult};
pub use processor::MetadataProcessor;
