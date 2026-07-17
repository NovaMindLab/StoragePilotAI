use std::sync::Arc;
use tauri::State;
use serde::Serialize;
use rusqlite::params;

use sm_types::file::FileEntry;
use sm_ai::engine::MobileClipEngine;
use sm_ai::text::encode_text;
use crate::AppState;

#[derive(Debug, Serialize)]
pub struct SemanticSearchResult {
    pub file: FileEntry,
    pub distance: f32,
}

#[tauri::command]
pub async fn cmd_search_semantic(
    query: String,
    state: State<'_, AppState>,
) -> Result<Vec<SemanticSearchResult>, String> {
    let engine = state.ai_engine.as_ref().ok_or("AI Engine not initialized")?;
    
    // 1. Convert text query to embedding
    let embedding = encode_text(
        engine.text_session(),
        engine.tokenizer(),
        &query
    ).map_err(|e| e.to_string())?;

    // 2. Search sqlite-vec
    // Serialize embedding array to raw bytes (f32 array) for sqlite-vec
    let embedding_bytes = unsafe {
        std::slice::from_raw_parts(
            embedding.as_ptr() as *const u8,
            embedding.len() * std::mem::size_of::<f32>(),
        )
    };

    let pool = state.pool.clone();
    
    // Use spawn_blocking for sqlite operations to avoid blocking async thread
    let results = tokio::task::spawn_blocking(move || -> Result<Vec<SemanticSearchResult>, String> {
        let conn = pool.get().map_err(|e| e.to_string())?;
        
        let sql = r#"
            SELECT f.id, f.path, f.name, f.extension, f.size, f.kind, f.category, 
                   f.created_at, f.modified_at, f.accessed_at, f.depth, f.parent_id,
                   vec_distance_L2(e.embedding, ?1) as distance
            FROM file_embeddings e
            JOIN files f ON e.file_id = f.id
            ORDER BY distance ASC
            LIMIT 50
        "#;
        
        let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
        
        let rows = stmt.query_map(params![embedding_bytes], |row| {
            let id: i64 = row.get(0)?;
            let path: String = row.get(1)?;
            let name: String = row.get(2)?;
            let extension: Option<String> = row.get(3)?;
            let size: i64 = row.get(4)?;
            let kind_str: String = row.get(5)?;
            let category_str: String = row.get(6)?;
            let created_at: Option<i64> = row.get(7)?;
            let modified_at: Option<i64> = row.get(8)?;
            let accessed_at: Option<i64> = row.get(9)?;
            let depth: u32 = row.get(10)?;
            let parent_id: Option<i64> = row.get(11)?;
            let distance: f32 = row.get(12)?;

            // Use JSON parsing as a workaround for FromStr
            let kind = serde_json::from_str(&format!("\"{}\"", kind_str)).unwrap_or(sm_types::file::FileKind::Other);
            let category = serde_json::from_str(&format!("\"{}\"", category_str)).unwrap_or(sm_types::file::FileCategory::Other);

            let to_datetime = |ts: Option<i64>| {
                ts.map(|t| chrono::DateTime::from_timestamp_millis(t).unwrap_or_default())
            };

            let file = FileEntry {
                id: sm_types::file::FileId(id as u64),
                path: std::path::PathBuf::from(path),
                name,
                extension,
                size: size as u64,
                kind,
                category,
                created_at: to_datetime(created_at),
                modified_at: to_datetime(modified_at),
                accessed_at: to_datetime(accessed_at),
                inode: None,
                is_hidden: false,
                depth,
                parent_id: parent_id.map(|id| sm_types::file::FileId(id as u64)),
            };

            Ok(SemanticSearchResult { file, distance })
        }).map_err(|e| e.to_string())?;
        
        let mut results = Vec::new();
        for row in rows {
            if let Ok(r) = row {
                results.push(r);
            }
        }
        
        Ok(results)
    }).await.map_err(|e| e.to_string())??;

    Ok(results)
}
