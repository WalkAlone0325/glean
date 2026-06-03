use crate::db::Database;
use crate::scanner::Scheduler;
use crate::search::{SearchFilter, SearchResult, Searcher};
use std::path::PathBuf;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

#[tauri::command]
pub fn get_stats(
    db: State<'_, std::sync::Arc<tokio::sync::Mutex<Database>>>,
) -> Result<Stats, String> {
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

#[tauri::command]
pub async fn search_files(
    db: State<'_, std::sync::Arc<tokio::sync::Mutex<Database>>>,
    query: String,
    filter: Option<SearchFilter>,
    limit: Option<i64>,
) -> Result<Vec<SearchResult>, String> {
    let db = db.lock().await;
    let searcher = Searcher::new();
    searcher
        .search(&db.conn, &query, filter.unwrap_or_default(), limit.unwrap_or(50))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn open_file(path: String) -> Result<(), String> {
    std::process::Command::new("open")
        .arg(&path)
        .status()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_indexed_roots(
    db: State<'_, std::sync::Arc<tokio::sync::Mutex<Database>>>,
) -> Result<Vec<String>, String> {
    let db = db.blocking_lock();
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT substr(path, 1, instr(substr(path, 2), '/') + 1) AS root
             FROM files
             WHERE deleted_at IS NULL
             GROUP BY root
             ORDER BY COUNT(*) DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

#[derive(serde::Serialize)]
pub struct Stats {
    pub files: u64,
    pub chunks: u64,
    pub tags: u64,
}

