//! File classification utilities for Stage 1 scanning.
//!
//! Classifies files into broad categories based on extension,
//! and provides helpers for detecting hidden entries.

/// Classify a file into a category based on its extension.
///
/// The comparison is case-insensitive. Returns a `'static` string
/// that can be stored without allocation.
///
/// # Examples
/// ```
/// use sm_scanner::file_classifier::classify_extension;
/// assert_eq!(classify_extension("jpg"), "image");
/// assert_eq!(classify_extension("MP4"), "video");
/// assert_eq!(classify_extension("rs"),  "code");
/// ```
pub fn classify_extension(ext: &str) -> &'static str {
    match ext.to_lowercase().as_str() {
        // Images
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "svg" | "ico"
        | "tif" | "tiff" | "heic" | "heif" | "avif" | "raw" | "cr2" | "nef" => "image",
        // Videos
        "mp4" | "mkv" | "avi" | "mov" | "wmv" | "flv" | "webm" | "m4v"
        | "3gp" | "ts" | "mts" | "m2ts" => "video",
        // Audio
        "mp3" | "flac" | "wav" | "aac" | "ogg" | "m4a" | "wma" | "opus"
        | "aiff" | "ape" | "dsf" => "audio",
        // Documents
        "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "odt"
        | "ods" | "odp" | "txt" | "rtf" | "md" | "epub" | "pages" | "numbers" => "document",
        // Archives
        "zip" | "tar" | "gz" | "bz2" | "7z" | "rar" | "xz" | "zst" | "dmg" | "iso" => "archive",
        // Code
        "rs" | "py" | "js" | "go" | "java" | "cpp" | "c" | "h" | "cs"
        | "rb" | "php" | "swift" | "kt" | "vue" | "jsx" | "tsx" | "html" | "css"
        | "json" | "yaml" | "yml" | "toml" | "xml" | "sh" | "bash" | "zsh" => "code",
        // System
        "dll" | "so" | "dylib" | "exe" | "app" | "sys" | "drv" => "system",
        _ => "other",
    }
}

/// Return `true` if a file or directory name is hidden (starts with `.`).
///
/// # Examples
/// ```
/// use sm_scanner::file_classifier::is_hidden;
/// assert!(is_hidden(".gitignore"));
/// assert!(!is_hidden("README.md"));
/// ```
pub fn is_hidden(name: &str) -> bool {
    name.starts_with('.')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_images() {
        assert_eq!(classify_extension("jpg"), "image");
        assert_eq!(classify_extension("PNG"), "image");
        assert_eq!(classify_extension("heic"), "image");
        assert_eq!(classify_extension("CR2"), "image");
    }

    #[test]
    fn classifies_videos() {
        assert_eq!(classify_extension("mp4"), "video");
        assert_eq!(classify_extension("MKV"), "video");
        assert_eq!(classify_extension("m2ts"), "video");
    }

    #[test]
    fn classifies_audio() {
        assert_eq!(classify_extension("mp3"), "audio");
        assert_eq!(classify_extension("FLAC"), "audio");
        assert_eq!(classify_extension("opus"), "audio");
    }

    #[test]
    fn classifies_documents() {
        assert_eq!(classify_extension("pdf"), "document");
        assert_eq!(classify_extension("DOCX"), "document");
        assert_eq!(classify_extension("md"), "document");
    }

    #[test]
    fn classifies_archives() {
        assert_eq!(classify_extension("zip"), "archive");
        assert_eq!(classify_extension("7z"), "archive");
        assert_eq!(classify_extension("zst"), "archive");
    }

    #[test]
    fn classifies_code() {
        assert_eq!(classify_extension("rs"), "code");
        assert_eq!(classify_extension("VUE"), "code");
        assert_eq!(classify_extension("tsx"), "code");
    }

    #[test]
    fn classifies_system() {
        assert_eq!(classify_extension("dylib"), "system");
        assert_eq!(classify_extension("DLL"), "system");
    }

    #[test]
    fn classifies_unknown_as_other() {
        assert_eq!(classify_extension("xyz123"), "other");
        assert_eq!(classify_extension(""), "other");
    }

    #[test]
    fn hidden_detection() {
        assert!(is_hidden(".gitignore"));
        assert!(is_hidden(".DS_Store"));
        assert!(is_hidden(".."));
        assert!(!is_hidden("README.md"));
        assert!(!is_hidden("main.rs"));
    }
}
