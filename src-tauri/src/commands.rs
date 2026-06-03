use crate::db::Database;
use crate::scanner::Scheduler;
use std::path::PathBuf;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

#[tauri::command]
pub fn get_stats(db: State<'_, std::sync::Arc<tokio::sync::Mutex<Database>>>) -> Result<Stats, String> {
    let db = db.blocking_lock();
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let files: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM files WHERE deleted_at IS NULL",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let chunks: i64 = conn
        .query_row("SELECT COUNT(*) FROM chunks", [], |r| r.get(0))
        .unwrap_or(0);
    let tags: i64 = conn
        .query_row("SELECT COUNT(*) FROM tags", [], |r| r.get(0))
        .unwrap_or(0);
    Ok(Stats {
        files: files as u64,
        chunks: chunks as u64,
        tags: tags as u64,
    })
}

#[tauri::command]
pub async fn start_indexing(
    app: AppHandle,
    scheduler: State<'_, std::sync::Arc<Scheduler>>,
    paths: Vec<PathBuf>,
) -> Result<(), String> {
    let scheduler = scheduler.inner().clone();
    tokio::spawn(async move {
        if let Err(e) = scheduler.index_paths(app, paths).await {
            tracing::error!("indexing failed: {}", e);
        }
    });
    Ok(())
}

#[tauri::command]
pub fn cancel_indexing(scheduler: State<'_, std::sync::Arc<Scheduler>>) {
    scheduler.cancel();
}

#[tauri::command]
pub fn is_indexing(scheduler: State<'_, std::sync::Arc<Scheduler>>) -> bool {
    scheduler.is_running()
}

#[derive(serde::Serialize)]
pub struct Stats {
    pub files: u64,
    pub chunks: u64,
    pub tags: u64,
}
