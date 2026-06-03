use crate::embedding::EmbeddingService;
use crate::search::SearchFilter;
use anyhow::Result;
use rusqlite::Connection;
use serde::Serialize;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize)]
pub struct VectorHit {
    pub chunk_id: i64,
    pub file_id: i64,
    pub path: String,
    pub name: String,
    pub ext: Option<String>,
    pub kind: Option<String>,
    pub snippet: Option<String>,
    pub distance: f32,
    pub score: f32,
}

pub fn vector_search(
    conn: &Mutex<Connection>,
    query: &str,
    filter: &SearchFilter,
    limit: i64,
) -> Result<Vec<VectorHit>> {
    let vec_bytes = embed_query_to_bytes(query)?;

    let mut sql = String::from(
        "SELECT v.rowid, v.distance, c.file_id, c.content, f.path, f.name, f.ext, f.kind
         FROM chunks_vec v
         JOIN chunks c ON c.id = v.rowid
         JOIN files f ON f.id = c.file_id
         WHERE v.embedding MATCH ?1
           AND k = ?2
           AND f.deleted_at IS NULL
           AND c.embedding_status = 'ready'",
    );

    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> =
        vec![Box::new(vec_bytes.clone()), Box::new(limit * 2)];
    let mut idx = 3;

    if let Some(ext) = &filter.ext {
        sql.push_str(&format!(" AND (f.ext = ?{} OR f.name LIKE ?{})", idx, idx + 1));
        params_vec.push(Box::new(ext.clone()));
        params_vec.push(Box::new(format!("%.{}", ext)));
        idx += 2;
    }
    if let Some(kind) = &filter.kind {
        sql.push_str(&format!(" AND f.kind = ?{}", idx));
        params_vec.push(Box::new(kind.clone()));
        idx += 1;
    }
    if let Some(path_contains) = &filter.path_contains {
        sql.push_str(&format!(" AND f.path LIKE ?{}", idx));
        params_vec.push(Box::new(format!("%{}%", path_contains)));
        idx += 1;
    }
    if let Some(since) = filter.since {
        sql.push_str(&format!(" AND f.mtime >= ?{}", idx));
        params_vec.push(Box::new(since));
    }

    sql.push_str(&format!(" ORDER BY v.distance ASC LIMIT {}", limit));

    let conn = conn.lock().map_err(|e| anyhow::anyhow!("db mutex: {}", e))?;
    let mut stmt = conn.prepare(&sql)?;
    let param_refs: Vec<&dyn rusqlite::ToSql> =
        params_vec.iter().map(|p| p.as_ref()).collect();

    let rows = stmt.query_map(param_refs.as_slice(), |row| {
        let distance: f32 = row.get(1)?;
        let snippet: String = row.get::<_, String>(3)?;
        Ok(VectorHit {
            chunk_id: row.get(0)?,
            distance,
            file_id: row.get(2)?,
            snippet: truncate_snippet(&snippet, 240),
            path: row.get(4)?,
            name: row.get(5)?,
            ext: row.get(6)?,
            kind: row.get(7)?,
            score: 1.0 / (1.0 + distance),
        })
    })?;

    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

fn embed_query_to_bytes(query: &str) -> Result<Vec<u8>> {
    let vec = EmbeddingService::embed_one(query)?;
    let mut bytes = Vec::with_capacity(vec.len() * 4);
    for f in &vec {
        bytes.extend_from_slice(&f.to_le_bytes());
    }
    Ok(bytes)
}

fn truncate_snippet(s: &str, max_chars: usize) -> Option<String> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if s.chars().count() <= max_chars {
        return Some(s.to_string());
    }
    let cut: String = s.chars().take(max_chars).collect();
    Some(format!("{}…", cut))
}
