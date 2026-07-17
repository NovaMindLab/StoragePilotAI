//! Complete SQLite schema definition.
//!
//! All tables, indexes, and virtual tables are defined here.
//! This serves as the single source of truth for the database schema.

/// Current schema version. Increment for each migration.
pub const SCHEMA_VERSION: u32 = 2;

/// SQL for the complete schema (version 1).
pub const SCHEMA_V1: &str = r#"
-- ============================================================
-- Core File Index
-- ============================================================

CREATE TABLE IF NOT EXISTS files (
    id              INTEGER PRIMARY KEY,
    path            TEXT    NOT NULL UNIQUE,
    name            TEXT    NOT NULL,
    extension       TEXT,
    size            INTEGER NOT NULL DEFAULT 0,
    kind            TEXT    NOT NULL DEFAULT 'regular',  -- regular|directory|symlink|other
    category        TEXT    NOT NULL DEFAULT 'other',    -- image|video|audio|document|archive|code|system|other
    parent_id       INTEGER REFERENCES files(id) ON DELETE CASCADE,
    depth           INTEGER NOT NULL DEFAULT 0,
    inode           INTEGER,
    is_hidden       INTEGER NOT NULL DEFAULT 0,          -- 0=false, 1=true
    created_at      INTEGER,   -- Unix timestamp ms
    modified_at     INTEGER,   -- Unix timestamp ms
    accessed_at     INTEGER,   -- Unix timestamp ms
    indexed_at      INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000),
    scan_id         TEXT,      -- which scan session created this entry
    stage           INTEGER NOT NULL DEFAULT 1  -- 1=basic, 2=metadata, 3=ai
);

CREATE INDEX IF NOT EXISTS idx_files_parent ON files(parent_id);
CREATE INDEX IF NOT EXISTS idx_files_name ON files(name);
CREATE INDEX IF NOT EXISTS idx_files_extension ON files(extension);
CREATE INDEX IF NOT EXISTS idx_files_kind ON files(kind);
CREATE INDEX IF NOT EXISTS idx_files_category ON files(category);
CREATE INDEX IF NOT EXISTS idx_files_size ON files(size);
CREATE INDEX IF NOT EXISTS idx_files_modified_at ON files(modified_at);
CREATE INDEX IF NOT EXISTS idx_files_depth ON files(depth);
CREATE INDEX IF NOT EXISTS idx_files_stage ON files(stage);

-- ============================================================
-- Extended Metadata (Stage 2 — populated after initial scan)
-- ============================================================

CREATE TABLE IF NOT EXISTS file_metadata (
    file_id         INTEGER PRIMARY KEY REFERENCES files(id) ON DELETE CASCADE,
    width           INTEGER,               -- image/video width px
    height          INTEGER,               -- image/video height px
    duration_secs   REAL,                  -- audio/video duration
    bitrate         INTEGER,               -- audio/video bitrate
    codec           TEXT,                  -- video/audio codec
    color_space     TEXT,                  -- image color space
    has_thumbnail   INTEGER DEFAULT 0,
    exif_data       TEXT,                  -- JSON blob
    updated_at      INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000)
);

-- ============================================================
-- AI Embeddings (Stage 3 — MobileCLIP text/image embeddings)
-- ============================================================

CREATE VIRTUAL TABLE IF NOT EXISTS file_embeddings USING vec0(
    file_id INTEGER PRIMARY KEY,
    embedding float[512]
);

-- ============================================================
-- File Hashes (Stage 2 — for duplicate detection)
-- ============================================================

CREATE TABLE IF NOT EXISTS file_hashes (
    file_id         INTEGER PRIMARY KEY REFERENCES files(id) ON DELETE CASCADE,
    hash_blake3     TEXT,
    hash_xxh3       TEXT,
    computed_at     INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000)
);

CREATE INDEX IF NOT EXISTS idx_file_hashes_blake3 ON file_hashes(hash_blake3);
CREATE INDEX IF NOT EXISTS idx_file_hashes_xxh3 ON file_hashes(hash_xxh3);

-- ============================================================
-- Duplicate Groups
-- ============================================================

CREATE TABLE IF NOT EXISTS duplicate_groups (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    hash            TEXT    NOT NULL UNIQUE,
    file_count      INTEGER NOT NULL DEFAULT 0,
    total_size      INTEGER NOT NULL DEFAULT 0,
    wasted_size     INTEGER NOT NULL DEFAULT 0,
    discovered_at   INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000)
);

CREATE TABLE IF NOT EXISTS duplicate_members (
    group_id        INTEGER NOT NULL REFERENCES duplicate_groups(id) ON DELETE CASCADE,
    file_id         INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    PRIMARY KEY (group_id, file_id)
);

-- ============================================================
-- Thumbnails
-- ============================================================

CREATE TABLE IF NOT EXISTS thumbnails (
    file_id         INTEGER PRIMARY KEY REFERENCES files(id) ON DELETE CASCADE,
    data            BLOB,                  -- JPEG bytes
    width           INTEGER NOT NULL,
    height          INTEGER NOT NULL,
    size_bytes      INTEGER NOT NULL,
    generated_at    INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000)
);

-- ============================================================
-- Task Queue
-- ============================================================

CREATE TABLE IF NOT EXISTS tasks (
    id              TEXT    PRIMARY KEY,   -- UUID
    kind            TEXT    NOT NULL,      -- scan|hash|thumbnail|ocr|embedding|clean
    priority        INTEGER NOT NULL DEFAULT 5,  -- 0=highest, 9=lowest
    status          TEXT    NOT NULL DEFAULT 'queued',  -- queued|running|paused|cancelled|done|failed
    payload         TEXT    NOT NULL DEFAULT '{}',  -- JSON
    progress        REAL    NOT NULL DEFAULT 0.0,   -- 0.0..1.0
    error_msg       TEXT,
    created_at      INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000),
    started_at      INTEGER,
    completed_at    INTEGER
);

CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
CREATE INDEX IF NOT EXISTS idx_tasks_priority ON tasks(priority, created_at);

-- ============================================================
-- Scan Sessions
-- ============================================================

CREATE TABLE IF NOT EXISTS scan_sessions (
    id              TEXT    PRIMARY KEY,   -- UUID
    drive_path      TEXT    NOT NULL,
    status          TEXT    NOT NULL DEFAULT 'scanning',
    files_found     INTEGER NOT NULL DEFAULT 0,
    dirs_found      INTEGER NOT NULL DEFAULT 0,
    bytes_scanned   INTEGER NOT NULL DEFAULT 0,
    started_at      INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000),
    completed_at    INTEGER
);

-- ============================================================
-- Cleaner Rules and Results
-- ============================================================

CREATE TABLE IF NOT EXISTS clean_rules (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT    NOT NULL,
    description     TEXT,
    rule_type       TEXT    NOT NULL,      -- temp_files|large_files|duplicates|old_downloads|custom
    config          TEXT    NOT NULL DEFAULT '{}',  -- JSON rule parameters
    enabled         INTEGER NOT NULL DEFAULT 1,
    created_at      INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000)
);

CREATE TABLE IF NOT EXISTS clean_results (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    rule_id         INTEGER REFERENCES clean_rules(id),
    file_id         INTEGER REFERENCES files(id),
    action          TEXT    NOT NULL,     -- deleted|trashed|moved
    bytes_freed     INTEGER NOT NULL DEFAULT 0,
    cleaned_at      INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000)
);

-- ============================================================
-- Settings (key-value store for UI preferences)
-- ============================================================

CREATE TABLE IF NOT EXISTS settings (
    key             TEXT    PRIMARY KEY,
    value           TEXT    NOT NULL,
    updated_at      INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000)
);

-- ============================================================
-- FTS5 Full-Text Search (mirrors files table for search)
-- ============================================================

CREATE VIRTUAL TABLE IF NOT EXISTS files_fts USING fts5(
    name,
    path,
    extension,
    content='files',
    content_rowid='id',
    tokenize='unicode61 remove_diacritics 1'
);

-- Triggers to keep FTS index in sync with files table
CREATE TRIGGER IF NOT EXISTS files_ai AFTER INSERT ON files BEGIN
    INSERT INTO files_fts(rowid, name, path, extension)
    VALUES (new.id, new.name, new.path, new.extension);
END;

CREATE TRIGGER IF NOT EXISTS files_ad AFTER DELETE ON files BEGIN
    INSERT INTO files_fts(files_fts, rowid, name, path, extension)
    VALUES ('delete', old.id, old.name, old.path, old.extension);
END;

CREATE TRIGGER IF NOT EXISTS files_au AFTER UPDATE ON files BEGIN
    INSERT INTO files_fts(files_fts, rowid, name, path, extension)
    VALUES ('delete', old.id, old.name, old.path, old.extension);
    INSERT INTO files_fts(rowid, name, path, extension)
    VALUES (new.id, new.name, new.path, new.extension);
END;

-- ============================================================
-- Schema Version Tracking
-- ============================================================

CREATE TABLE IF NOT EXISTS schema_versions (
    version         INTEGER PRIMARY KEY,
    applied_at      INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000),
    description     TEXT
);
"#;
