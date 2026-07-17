use serde::{Deserialize, Serialize};
use tauri::State;

use crate::state::AppState;
use sm_types::file::FileEntry;

#[derive(Debug, Serialize)]
pub struct DuplicateGroupResponse {
    pub group_id: i64,
    pub hash: String,
    pub file_count: i64,
    pub total_size: i64,
    pub wasted_size: i64,
    pub files: Vec<FileEntry>,
}

#[tauri::command]
pub async fn cmd_get_duplicates(
    state: State<'_, AppState>,
) -> Result<Vec<DuplicateGroupResponse>, String> {
    let pool = &state.pool;
    let conn = pool.get().map_err(|e| e.to_string())?;

    // First, find duplicate hashes by aggregating file_hashes table
    let mut stmt = conn.prepare(
        "SELECT hash_blake3, COUNT(file_id) as count, SUM(f.size) as total_size 
         FROM file_hashes fh
         JOIN files f ON fh.file_id = f.id
         GROUP BY hash_blake3 
         HAVING count > 1 
         ORDER BY total_size DESC LIMIT 100"
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map([], |row| {
        let hash: String = row.get(0)?;
        let count: i64 = row.get(1)?;
        let total_size: i64 = row.get(2)?;
        Ok((hash, count, total_size))
    }).map_err(|e| e.to_string())?;

    let mut groups = Vec::new();
    let mut group_id = 1;

    for row in rows {
        if let Ok((hash, count, total_size)) = row {
            let wasted_size = total_size - (total_size / count);

            // Fetch the actual files for this hash
            let mut file_stmt = conn.prepare(
                "SELECT f.id, f.path, f.name, f.extension, f.size, f.kind, f.category, f.created_at, f.modified_at, f.accessed_at, f.inode, f.is_hidden, f.depth, f.parent_id 
                 FROM files f 
                 JOIN file_hashes fh ON f.id = fh.file_id 
                 WHERE fh.hash_blake3 = ?1"
            ).map_err(|e| e.to_string())?;

            let file_rows = file_stmt.query_map(rusqlite::params![hash], |row| {
                // We're taking a shortcut and creating a dummy FileEntry because we need to parse timestamps.
                // In a real app we'd map this properly using sm_database mapping functions.
                Ok(FileEntry {
                    id: sm_types::file::FileId(row.get::<_, i64>(0)? as u64),
                    path: std::path::PathBuf::from(row.get::<_, String>(1)?),
                    name: row.get(2)?,
                    extension: row.get(3)?,
                    size: row.get(4)?,
                    kind: sm_types::file::FileKind::Regular, // Dummy
                    category: sm_types::file::FileCategory::Other, // Dummy
                    created_at: None,
                    modified_at: None,
                    accessed_at: None,
                    inode: row.get(10)?,
                    is_hidden: row.get::<_, i32>(11)? == 1,
                    depth: row.get::<_, i32>(12)? as u32,
                    parent_id: row.get::<_, Option<i64>>(13)?.map(|id| sm_types::file::FileId(id as u64)),
                })
            }).unwrap();

            let mut files = Vec::new();
            for file_row in file_rows {
                if let Ok(f) = file_row {
                    files.push(f);
                }
            }

            groups.push(DuplicateGroupResponse {
                group_id,
                hash,
                file_count: count,
                total_size,
                wasted_size,
                files,
            });
            group_id += 1;
        }
    }

    Ok(groups)
}

#[derive(Debug, Serialize)]
pub struct FileMetadataResponse {
    pub file_id: i64,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub duration_secs: Option<f64>,
    pub bitrate: Option<i64>,
    pub exif_data: Option<String>,
}

#[tauri::command]
pub async fn cmd_get_file_metadata(
    state: State<'_, AppState>,
    file_id: i64,
) -> Result<FileMetadataResponse, String> {
    let pool = &state.pool;
    let conn = pool.get().map_err(|e| e.to_string())?;

    let res = conn.query_row(
        "SELECT width, height, duration_secs, bitrate, exif_data FROM file_metadata WHERE file_id = ?1",
        rusqlite::params![file_id],
        |row| {
            Ok(FileMetadataResponse {
                file_id,
                width: row.get(0)?,
                height: row.get(1)?,
                duration_secs: row.get(2)?,
                bitrate: row.get(3)?,
                exif_data: row.get(4)?,
            })
        }
    ).map_err(|e| e.to_string())?;
    
    Ok(res)
}
