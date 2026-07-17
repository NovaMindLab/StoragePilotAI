//! Core file-system domain types.
//!
//! Covers the Stage 1 [`FileEntry`] (fast metadata) and the Stage 2
//! [`FileMetadata`] (enriched / media-parsed data), plus [`FolderStats`] for
//! aggregated folder analytics.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// FileId
// ---------------------------------------------------------------------------

/// Opaque identifier for a file entry in the Storage Index.
///
/// Backed by a `u64` derived from the inode number on Unix, or a synthetic
/// hash on Windows / non-POSIX platforms.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(transparent)]
pub struct FileId(pub u64);

impl FileId {
    /// Creates a new `FileId` from a raw integer.
    #[inline]
    pub fn new(v: u64) -> Self {
        Self(v)
    }

    /// Returns the inner `u64` value.
    #[inline]
    pub fn inner(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for FileId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// FileKind
// ---------------------------------------------------------------------------

/// The filesystem kind of an entry — mirrors `std::fs::FileType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum FileKind {
    /// A regular file.
    #[default]
    Regular,
    /// A directory.
    Directory,
    /// A symbolic link (target may or may not exist).
    Symlink,
    /// Anything else: named pipe, device file, socket, …
    Other,
}

// ---------------------------------------------------------------------------
// FileCategory
// ---------------------------------------------------------------------------

/// High-level semantic category inferred from the file extension / MIME type.
///
/// Used by the search layer to provide fast category-filtered queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum FileCategory {
    /// Raster or vector image (jpg, png, svg, …).
    Image,
    /// Video file (mp4, mov, mkv, …).
    Video,
    /// Audio file (mp3, flac, wav, …).
    Audio,
    /// Document (pdf, docx, md, txt, …).
    Document,
    /// Archive / compressed bundle (zip, tar, gz, …).
    Archive,
    /// Source code or script.
    Code,
    /// OS / app system files (dylib, dll, pkg, …).
    System,
    /// Anything that does not map to the above categories.
    #[default]
    Other,
}

impl FileCategory {
    /// Infer the [`FileCategory`] from a lowercase file extension, if known.
    pub fn from_extension(ext: &str) -> Self {
        match ext {
            // Images
            "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "tiff" | "tif"
            | "heic" | "heif" | "avif" | "svg" | "ico" | "raw" | "cr2" | "nef"
            | "arw" | "dng" => Self::Image,

            // Video — use "mts" for MPEG Transport Stream; "ts" is TypeScript (Code)
            "mp4" | "mov" | "avi" | "mkv" | "wmv" | "flv" | "webm" | "m4v"
            | "mpg" | "mpeg" | "mts" | "3gp" => Self::Video,

            // Audio
            "mp3" | "flac" | "wav" | "aac" | "ogg" | "m4a" | "wma" | "opus"
            | "aiff" | "alac" => Self::Audio,

            // Documents
            "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "odt"
            | "ods" | "odp" | "txt" | "md" | "rtf" | "epub" | "pages"
            | "numbers" | "key" => Self::Document,

            // Archives
            "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" | "dmg"
            | "iso" | "pkg" | "deb" | "rpm" => Self::Archive,

            // Code
            "rs" | "py" | "js" | "ts" | "go" | "c" | "cpp" | "h" | "hpp"
            | "java" | "kt" | "swift" | "rb" | "php" | "sh" | "bash"
            | "zsh" | "fish" | "cs" | "dart" | "lua" | "toml" | "yaml"
            | "yml" | "json" | "xml" | "html" | "css" | "scss" | "sql" => Self::Code,

            // System
            "dylib" | "dll" | "so" | "exe" | "app" | "framework" | "sys"
            | "ini" | "plist" | "reg" => Self::System,

            _ => Self::Other,
        }
    }
}

// ---------------------------------------------------------------------------
// FileEntry  (Stage 1)
// ---------------------------------------------------------------------------

/// A single file-system entry produced by the Stage 1 fast-metadata scan.
///
/// All fields are populated purely from `std::fs::Metadata` and path
/// decomposition — no file content is read at this stage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    /// Unique identifier within the Storage Index.
    pub id: FileId,

    /// Absolute path to the file or directory.
    pub path: PathBuf,

    /// File name component of the path (no directory prefix).
    pub name: String,

    /// Lowercase extension, if present (e.g. `"rs"`, `"png"`).
    pub extension: Option<String>,

    /// Size in bytes (0 for directories).
    pub size: u64,

    /// Filesystem kind of the entry.
    pub kind: FileKind,

    /// Semantic category inferred from the extension.
    pub category: FileCategory,

    /// Creation timestamp (unavailable on some Linux filesystems).
    pub created_at: Option<DateTime<Utc>>,

    /// Last-modification timestamp.
    pub modified_at: Option<DateTime<Utc>>,

    /// Last-access timestamp (may be disabled on some mounts).
    pub accessed_at: Option<DateTime<Utc>>,

    /// Unix inode number or equivalent.
    pub inode: Option<u64>,

    /// Whether the file is considered hidden (starts with `.` on Unix, or has
    /// the hidden attribute set on Windows).
    pub is_hidden: bool,

    /// Depth of this entry relative to the scan root (root itself = 0).
    pub depth: u32,

    /// [`FileId`] of the containing directory entry, if known.
    pub parent_id: Option<FileId>,
}

// ---------------------------------------------------------------------------
// FileMetadata  (Stage 2)
// ---------------------------------------------------------------------------

/// Enriched metadata for a file, produced by the Stage 2 media-parsing scan.
///
/// Only populated for entries where richer introspection is worth the cost
/// (images, video, audio). Hash fields may be populated lazily by a background
/// task.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FileMetadata {
    /// References the owning [`FileEntry`].
    pub file_id: FileId,

    /// Image / video width in pixels.
    pub width: Option<u32>,

    /// Image / video height in pixels.
    pub height: Option<u32>,

    /// Audio / video duration in **seconds**.
    pub duration: Option<f64>,

    /// BLAKE3 content hash (hex-encoded, 64 chars).
    pub hash_blake3: Option<String>,

    /// xxHash3 content hash (hex-encoded, 16 chars).
    pub hash_xxh3: Option<String>,

    /// Raw EXIF / XMP / ID3 tags as a JSON object.
    ///
    /// Keys and value structure vary by file format.  Consumers should treat
    /// this as an opaque blob unless they need specific fields.
    pub exif_data: Option<serde_json::Value>,

    /// Whether a thumbnail has been generated and stored on disk.
    pub has_thumbnail: bool,
}

// ---------------------------------------------------------------------------
// FolderStats
// ---------------------------------------------------------------------------

/// Aggregated statistics for a directory subtree.
///
/// Computed by the index aggregation layer after Stage 1 completes for a
/// given root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FolderStats {
    /// Absolute path of the directory.
    pub path: PathBuf,

    /// Total number of regular files (recursive).
    pub total_files: u64,

    /// Total number of sub-directories (recursive).
    pub total_dirs: u64,

    /// Sum of all file sizes in bytes (recursive).
    pub total_size: u64,

    /// Size of the single largest file in bytes.
    pub largest_file_size: u64,

    /// Maximum depth of nesting below this directory.
    pub depth: u32,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_id_roundtrip() {
        let id = FileId::new(42);
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "42");
        let back: FileId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, id);
    }

    #[test]
    fn file_category_from_extension() {
        assert_eq!(FileCategory::from_extension("rs"), FileCategory::Code);
        assert_eq!(FileCategory::from_extension("mp4"), FileCategory::Video);
        assert_eq!(FileCategory::from_extension("png"), FileCategory::Image);
        assert_eq!(FileCategory::from_extension("xyz"), FileCategory::Other);
    }

    #[test]
    fn file_entry_serialization() {
        let entry = FileEntry {
            id: FileId::new(1),
            path: PathBuf::from("/tmp/test.rs"),
            name: "test.rs".into(),
            extension: Some("rs".into()),
            size: 1024,
            kind: FileKind::Regular,
            category: FileCategory::Code,
            created_at: None,
            modified_at: None,
            accessed_at: None,
            inode: Some(999),
            is_hidden: false,
            depth: 2,
            parent_id: Some(FileId::new(0)),
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"isHidden\":false"));
        assert!(json.contains("\"category\":\"code\""));
    }
}
