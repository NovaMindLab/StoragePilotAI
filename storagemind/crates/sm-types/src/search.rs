//! Search query and result types.
//!
//! [`SearchQuery`] is the canonical input structure accepted by the search
//! subsystem.  [`SearchResult`] is the paginated response returned to callers
//! (including the Vue frontend via IPC).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::file::{FileCategory, FileEntry};

// ---------------------------------------------------------------------------
// SearchQuery
// ---------------------------------------------------------------------------

/// A structured search request submitted to the Storage Index.
///
/// All filter fields are optional; omitting them widens the result set.
/// Pagination is handled via `limit` / `offset`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchQuery {
    /// Free-text search string.  Matched against file name, path, and
    /// (where available) AI-generated tags or OCR text.
    pub query: String,

    /// Restrict results to files under this absolute directory path.
    pub path_filter: Option<String>,

    /// Restrict results to files whose [`FileCategory`] is in this list.
    pub category_filter: Option<Vec<FileCategory>>,

    /// Minimum file size in bytes (inclusive).
    pub size_min: Option<u64>,

    /// Maximum file size in bytes (inclusive).
    pub size_max: Option<u64>,

    /// Only include files modified on or after this timestamp.
    pub modified_after: Option<DateTime<Utc>>,

    /// Only include files modified on or before this timestamp.
    pub modified_before: Option<DateTime<Utc>>,

    /// Maximum number of entries to return in a single response.
    ///
    /// Defaults to `50` when constructed via [`SearchQuery::default`].
    pub limit: usize,

    /// Zero-based offset into the full result set for pagination.
    pub offset: usize,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            query: String::new(),
            path_filter: None,
            category_filter: None,
            size_min: None,
            size_max: None,
            modified_after: None,
            modified_before: None,
            limit: 50,
            offset: 0,
        }
    }
}

impl SearchQuery {
    /// Creates a simple name-only query with default pagination.
    pub fn simple(q: impl Into<String>) -> Self {
        Self {
            query: q.into(),
            ..Default::default()
        }
    }

    /// Returns `true` if no filters other than the query string are set.
    pub fn is_simple(&self) -> bool {
        self.path_filter.is_none()
            && self.category_filter.is_none()
            && self.size_min.is_none()
            && self.size_max.is_none()
            && self.modified_after.is_none()
            && self.modified_before.is_none()
    }
}

// ---------------------------------------------------------------------------
// SearchResult
// ---------------------------------------------------------------------------

/// The paginated response returned by the search subsystem.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    /// The page of matching [`FileEntry`] objects.
    pub entries: Vec<FileEntry>,

    /// Total number of entries matching the query (ignoring pagination).
    pub total: u64,

    /// Wall-clock milliseconds spent executing the query.
    pub elapsed_ms: u64,
}

impl SearchResult {
    /// Returns `true` when no entries matched the query.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns the number of entries in this page.
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_limit() {
        assert_eq!(SearchQuery::default().limit, 50);
    }

    #[test]
    fn simple_query_is_simple() {
        let q = SearchQuery::simple("hello");
        assert!(q.is_simple());
        assert_eq!(q.query, "hello");
    }

    #[test]
    fn search_result_empty() {
        let r = SearchResult::default();
        assert!(r.is_empty());
        assert_eq!(r.len(), 0);
    }

    #[test]
    fn search_query_serde_roundtrip() {
        let q = SearchQuery {
            query: "test".into(),
            limit: 10,
            offset: 20,
            size_min: Some(1024),
            ..Default::default()
        };
        let json = serde_json::to_string(&q).unwrap();
        let back: SearchQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(q, back);
    }
}
