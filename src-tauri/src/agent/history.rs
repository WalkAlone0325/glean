use crate::db::Database;
use anyhow::{anyhow, Result};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub id: i64,
    pub kind: String,
    pub source_path: Option<String>,
    pub target_path: Option<String>,
    pub payload: Option<String>,
    pub undone_at: Option<i64>,
    pub created_at: i64,
}

pub async fn record(
    db: &Arc<Mutex<Database>>,
    kind: &str,
    source_path: Option<&str>,
    target_path: Option<&str>,
    payload: Option<&str>,
) -> Result<i64> {
    let db_lock = db.lock().await;
    let conn = db_lock.conn.lock().map_err(|e| anyhow!("db mutex: {}", e))?;
    conn.execute(
        "INSERT INTO operations (kind, source_path, target_path, payload) VALUES (?1, ?2, ?3, ?4)",
        params![kind, source_path, target_path, payload],
    )?;
    Ok(conn.last_insert_rowid())
}

pub async fn list_pending(db: &Arc<Mutex<Database>>, limit: i64) -> Result<Vec<Operation>> {
    let db_lock = db.lock().await;
    let conn = db_lock.conn.lock().map_err(|e| anyhow!("db mutex: {}", e))?;
    let mut stmt = conn.prepare(
        "SELECT id, kind, source_path, target_path, payload, undone_at, created_at
         FROM operations WHERE undone_at IS NULL
         ORDER BY id DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit], |r| {
        Ok(Operation {
            id: r.get(0)?,
            kind: r.get(1)?,
            source_path: r.get(2)?,
            target_path: r.get(3)?,
            payload: r.get(4)?,
            undone_at: r.get(5)?,
            created_at: r.get(6)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub async fn undo(db: &Arc<Mutex<Database>>, operation_id: i64) -> Result<Operation> {
    let db_lock = db.lock().await;
    let conn = db_lock.conn.lock().map_err(|e| anyhow!("db mutex: {}", e))?;

    let op: Operation = conn
        .query_row(
            "SELECT id, kind, source_path, target_path, payload, undone_at, created_at
             FROM operations WHERE id = ?1",
            params![operation_id],
            |r| {
                Ok(Operation {
                    id: r.get(0)?,
                    kind: r.get(1)?,
                    source_path: r.get(2)?,
                    target_path: r.get(3)?,
                    payload: r.get(4)?,
                    undone_at: r.get(5)?,
                    created_at: r.get(6)?,
                })
            },
        )
        .map_err(|e| anyhow!("operation not found: {}", e))?;

    if op.undone_at.is_some() {
        return Err(anyhow!("operation already undone"));
    }

    match op.kind.as_str() {
        "move" => {
            let src = op
                .source_path
                .as_ref()
                .ok_or_else(|| anyhow!("move op missing source_path"))?;
            let dst = op
                .target_path
                .as_ref()
                .ok_or_else(|| anyhow!("move op missing target_path"))?;

            if let Some(payload) = &op.payload {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(payload) {
                    if v.get("overwritten").and_then(|b| b.as_bool()).unwrap_or(false) {
                        return Err(anyhow!(
                            "无法撤销：原目标已被覆盖，撤销将丢失被覆盖的文件"
                        ));
                    }
                }
            }

            if !std::path::Path::new(dst).exists() {
                return Err(anyhow!("无法撤销：目标文件已不存在 {}", dst));
            }
            if std::path::Path::new(src).exists() {
                return Err(anyhow!("无法撤销：源路径已存在 {}", src));
            }
            std::fs::rename(dst, src)?;

            let _ = conn.execute(
                "UPDATE files SET path = ?1 WHERE path = ?2",
                params![src, dst],
            );
        }
        "tag" => {
            let payload_str = op
                .payload
                .as_ref()
                .ok_or_else(|| anyhow!("tag op missing payload"))?;
            let payload: TagOpPayload = serde_json::from_str(payload_str)?;
            let file_id = payload.file_id;

            for tag_name in &payload.attached {
                conn.execute(
                    "DELETE FROM file_tags WHERE file_id = ?1 AND tag_id = (
                         SELECT id FROM tags WHERE name = ?2
                     )",
                    params![file_id, tag_name],
                )?;
            }
        }
        other => return Err(anyhow!("unknown operation kind: {}", other)),
    }

    conn.execute(
        "UPDATE operations SET undone_at = strftime('%s','now') WHERE id = ?1",
        params![operation_id],
    )?;

    Ok(Operation {
        undone_at: Some(now_ts()),
        ..op
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TagOpPayload {
    file_id: i64,
    attached: Vec<String>,
}

fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
