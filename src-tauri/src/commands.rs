use crate::db::Database;
use tauri::State;

#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

#[tauri::command]
pub fn get_stats(db: State<'_, Database>) -> Result<Stats, String> {
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

#[derive(serde::Serialize)]
pub struct Stats {
    pub files: u64,
    pub chunks: u64,
    pub tags: u64,
}
