use crate::db::Database;
use crate::scanner::Scheduler;
use crate::search::hybrid::{hybrid_search, HybridResult};
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
pub async fn hybrid_search_files(
    db: State<'_, std::sync::Arc<tokio::sync::Mutex<Database>>>,
    query: String,
    filter: Option<SearchFilter>,
    limit: Option<i64>,
) -> Result<Vec<HybridResult>, String> {
    let db = db.lock().await;
    hybrid_search(
        &db.conn,
        &query,
        filter.unwrap_or_default(),
        limit.unwrap_or(30),
    )
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
pub async fn reveal_in_finder(path: String) -> Result<(), String> {
    std::process::Command::new("open")
        .args(["-R", &path])
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
pub struct FileEntry {
    pub id: i64,
    pub path: String,
    pub name: String,
    pub ext: Option<String>,
    pub size: i64,
    pub mtime: i64,
    pub kind: Option<String>,
}

#[tauri::command]
pub fn list_files(
    db: State<'_, std::sync::Arc<tokio::sync::Mutex<Database>>>,
    sort: Option<String>,
    dir: Option<String>,
    kind: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<FileEntry>, String> {
    let db = db.blocking_lock();
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let sort_col = match sort.as_deref().unwrap_or("mtime") {
        "name" => "name",
        "size" => "size",
        "ext" => "ext",
        _ => "mtime",
    };
    let dir = match dir.as_deref() {
        Some("asc") => "ASC",
        _ => "DESC",
    };
    let kind_filter = match kind.as_deref() {
        Some(k) if !k.is_empty() => Some(k),
        _ => None,
    };

    let mut sql = String::from(
        "SELECT id, path, name, ext, size, mtime, kind FROM files
         WHERE deleted_at IS NULL",
    );
    if kind_filter.is_some() {
        sql.push_str(" AND kind = :kind");
    }
    sql.push_str(&format!(" ORDER BY {} {} LIMIT :limit OFFSET :offset", sort_col, dir));

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let limit = limit.unwrap_or(200).clamp(1, 2000);
    let offset = offset.unwrap_or(0).max(0);

    let mapper = |r: &rusqlite::Row<'_>| -> rusqlite::Result<FileEntry> {
        Ok(FileEntry {
            id: r.get(0)?,
            path: r.get(1)?,
            name: r.get(2)?,
            ext: r.get(3)?,
            size: r.get(4)?,
            mtime: r.get(5)?,
            kind: r.get(6)?,
        })
    };

    let rows: Vec<FileEntry> = if let Some(k) = kind_filter {
        stmt.query_map(
            &[
                (":kind", &k as &dyn rusqlite::ToSql),
                (":limit", &limit as &dyn rusqlite::ToSql),
                (":offset", &offset as &dyn rusqlite::ToSql),
            ],
            mapper,
        )
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?
    } else {
        stmt.query_map(
            &[
                (":limit", &limit as &dyn rusqlite::ToSql),
                (":offset", &offset as &dyn rusqlite::ToSql),
            ],
            mapper,
        )
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?
    };

    Ok(rows)
}

#[tauri::command]
pub fn read_text_preview(
    path: String,
    max_bytes: Option<i64>,
) -> Result<String, String> {
    let p = std::path::Path::new(&path);
    if !p.exists() {
        return Err("file not found".to_string());
    }
    let max = max_bytes.unwrap_or(8 * 1024).clamp(256, 256 * 1024) as usize;
    let meta = std::fs::metadata(p).map_err(|e| e.to_string())?;
    if meta.len() > 2 * 1024 * 1024 {
        return Err("file too large for preview".to_string());
    }
    let mut file = std::fs::File::open(p).map_err(|e| e.to_string())?;
    use std::io::Read;
    let mut buf = vec![0u8; max];
    let n = file.read(&mut buf).map_err(|e| e.to_string())?;
    buf.truncate(n);
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

#[derive(serde::Serialize)]
pub struct Stats {
    pub files: u64,
    pub chunks: u64,
    pub tags: u64,
}

