use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A single cleaning rule that describes what to remove.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanRule {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub rule_type: RuleType,
    pub enabled: bool,
}

/// The kind of files a [`CleanRule`] targets.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleType {
    /// Common temp / cache files (`.tmp`, `.DS_Store`, `Thumbs.db`, …).
    TempFiles,
    /// Files larger than a given threshold.
    LargeFiles { min_size_bytes: u64 },
    /// Byte-identical duplicate files.
    Duplicates,
    /// Files in Downloads older than N days.
    OldDownloads { days: u32 },
    /// Directories with no children.
    EmptyFolders,
    /// User-defined: specific extensions and/or path prefixes.
    Custom {
        extensions: Vec<String>,
        paths: Vec<PathBuf>,
    },
}
