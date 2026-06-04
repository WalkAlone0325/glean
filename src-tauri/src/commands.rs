use crate::db::Database;
use crate::llm::{openai::OpenAIProvider, ChatRequest, LLMProvider, ProviderConfig};
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
pub fn pause_indexing(scheduler: State<'_, std::sync::Arc<Scheduler>>) {
    scheduler.pause();
}

#[tauri::command]
pub fn resume_indexing(scheduler: State<'_, std::sync::Arc<Scheduler>>) {
    scheduler.resume();
}

#[tauri::command]
pub fn is_indexing(scheduler: State<'_, std::sync::Arc<Scheduler>>) -> bool {
    scheduler.is_running()
}

#[tauri::command]
pub fn is_paused(scheduler: State<'_, std::sync::Arc<Scheduler>>) -> bool {
    scheduler.is_paused()
}

#[derive(serde::Serialize)]
pub struct ConsistencyReport {
    pub checked: u64,
    pub missing: u64,
    pub marked_deleted: u64,
}

#[tauri::command]
pub async fn consistency_check(
    db: State<'_, std::sync::Arc<tokio::sync::Mutex<Database>>>,
) -> Result<ConsistencyReport, String> {
    let db = db.lock().await;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let total: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM files WHERE deleted_at IS NULL",
            [],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare("SELECT id, path FROM files WHERE deleted_at IS NULL")
        .map_err(|e| e.to_string())?;
    let rows: Vec<(i64, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    drop(stmt);

    let mut missing_ids: Vec<i64> = Vec::new();
    for (id, path) in &rows {
        if !std::path::Path::new(path).exists() {
            missing_ids.push(*id);
        }
    }
    let missing = missing_ids.len() as u64;

    let now = chrono::Utc::now().timestamp();
    let mut marked = 0u64;
    for chunk in missing_ids.chunks(200) {
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        for id in chunk {
            tx.execute(
                "UPDATE files SET deleted_at = ?1 WHERE id = ?2",
                rusqlite::params![now, id],
            )
            .map_err(|e| e.to_string())?;
            marked += 1;
        }
        tx.commit().map_err(|e| e.to_string())?;
    }

    tracing::info!(
        "consistency check: {} checked, {} missing, {} marked deleted",
        total,
        missing,
        marked
    );

    Ok(ConsistencyReport {
        checked: total as u64,
        missing,
        marked_deleted: marked,
    })
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

#[derive(serde::Serialize)]
pub struct RecentFile {
    pub id: i64,
    pub path: String,
    pub name: String,
    pub ext: Option<String>,
    pub size: i64,
    pub mtime: i64,
    pub kind: Option<String>,
    pub viewed_at: i64,
}

#[tauri::command]
pub async fn track_file_view(
    db: State<'_, std::sync::Arc<tokio::sync::Mutex<Database>>>,
    file_id: i64,
) -> Result<(), String> {
    let db_lock = db.lock().await;
    let conn = db_lock.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM recently_viewed WHERE file_id = ?1", rusqlite::params![file_id])
        .map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO recently_viewed (file_id) VALUES (?1)",
        rusqlite::params![file_id],
    )
    .map_err(|e| e.to_string())?;
    // keep only top 50
    conn.execute(
        "DELETE FROM recently_viewed WHERE rowid NOT IN (
            SELECT rowid FROM recently_viewed ORDER BY viewed_at DESC LIMIT 50
        )",
        [],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn list_recent_files(
    db: State<'_, std::sync::Arc<tokio::sync::Mutex<Database>>>,
    limit: Option<i64>,
) -> Result<Vec<RecentFile>, String> {
    let limit = limit.unwrap_or(20).clamp(1, 50);
    let db_lock = db.lock().await;
    let conn = db_lock.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT f.id, f.path, f.name, f.ext, f.size, f.mtime, f.kind, rv.viewed_at
             FROM recently_viewed rv
             JOIN files f ON f.id = rv.file_id
             WHERE f.deleted_at IS NULL
             ORDER BY rv.viewed_at DESC
             LIMIT ?1",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params![limit], |r| {
            Ok(RecentFile {
                id: r.get(0)?,
                path: r.get(1)?,
                name: r.get(2)?,
                ext: r.get(3)?,
                size: r.get(4)?,
                mtime: r.get(5)?,
                kind: r.get(6)?,
                viewed_at: r.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| e.to_string())?);
    }
    Ok(out)
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

#[tauri::command]
pub fn get_setting(
    db: State<'_, std::sync::Arc<tokio::sync::Mutex<Database>>>,
    key: String,
) -> Result<Option<String>, String> {
    let db = db.blocking_lock();
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT value FROM settings WHERE key = ?1")
        .map_err(|e| e.to_string())?;
    let mut rows = stmt
        .query_map(rusqlite::params![&key], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    if let Some(r) = rows.next() {
        Ok(Some(r.map_err(|e| e.to_string())?))
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub fn set_setting(
    db: State<'_, std::sync::Arc<tokio::sync::Mutex<Database>>>,
    key: String,
    value: String,
) -> Result<(), String> {
    let db = db.blocking_lock();
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value, updated_at) VALUES (?1, ?2, strftime('%s','now'))",
        rusqlite::params![&key, &value],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_provider_config(
    db: State<'_, std::sync::Arc<tokio::sync::Mutex<Database>>>,
) -> Result<ProviderConfig, String> {
    let db = db.blocking_lock();
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let get = |k: &str| -> Option<String> {
        conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            rusqlite::params![k],
            |r| r.get::<_, String>(0),
        )
        .ok()
    };
    Ok(ProviderConfig {
        provider: get("llm.provider").unwrap_or_else(|| "openai".to_string()),
        base_url: get("llm.base_url").unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
        api_key: get("llm.api_key").unwrap_or_default(),
        model: get("llm.model").unwrap_or_else(|| "gpt-4o-mini".to_string()),
    })
}

#[tauri::command]
pub async fn chat_stop(
    confirmations: State<'_, std::sync::Arc<crate::agent::ConfirmationRegistry>>,
    conversation_id: Option<i64>,
) -> Result<(), String> {
    crate::rag::request_stop();
    if let Some(cid) = conversation_id {
        confirmations.inner().cancel_conversation(cid).await;
    }
    Ok(())
}

#[tauri::command]
pub async fn chat_send(
    app: AppHandle,
    db: State<'_, std::sync::Arc<tokio::sync::Mutex<Database>>>,
    confirmations: State<'_, std::sync::Arc<crate::agent::ConfirmationRegistry>>,
    conversation_id: Option<i64>,
    message: String,
    use_rag: Option<bool>,
) -> Result<ChatStartResult, String> {
    let db = db.inner().clone();
    let confirmations = confirmations.inner().clone();
    let use_rag = use_rag.unwrap_or(true);
    let (conv_id, msg_id, rag) =
        crate::rag::run_chat(app, db, confirmations, conversation_id, message, use_rag)
            .await
            .map_err(|e| e.to_string())?;
    Ok(ChatStartResult {
        conversation_id: conv_id,
        message_id: msg_id,
        rag_context: rag,
    })
}

#[tauri::command]
pub async fn tool_confirm(
    confirmations: State<'_, std::sync::Arc<crate::agent::ConfirmationRegistry>>,
    conversation_id: i64,
    call_id: String,
    approved: bool,
) -> Result<bool, String> {
    let found = confirmations
        .inner()
        .resolve(conversation_id, &call_id, approved)
        .await;
    Ok(found)
}

#[derive(serde::Serialize)]
pub struct OperationSummary {
    pub id: i64,
    pub kind: String,
    pub source_path: Option<String>,
    pub target_path: Option<String>,
    pub payload: Option<String>,
    pub created_at: i64,
}

#[tauri::command]
pub async fn list_pending_operations(
    db: State<'_, std::sync::Arc<tokio::sync::Mutex<Database>>>,
    limit: Option<i64>,
) -> Result<Vec<OperationSummary>, String> {
    let limit = limit.unwrap_or(20).clamp(1, 100);
    let ops = crate::agent::history::list_pending(db.inner(), limit)
        .await
        .map_err(|e| e.to_string())?;
    Ok(ops
        .into_iter()
        .map(|o| OperationSummary {
            id: o.id,
            kind: o.kind,
            source_path: o.source_path,
            target_path: o.target_path,
            payload: o.payload,
            created_at: o.created_at,
        })
        .collect())
}

#[tauri::command]
pub async fn undo_operation(
    db: State<'_, std::sync::Arc<tokio::sync::Mutex<Database>>>,
    operation_id: i64,
) -> Result<(), String> {
    crate::agent::history::undo(db.inner(), operation_id)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[derive(serde::Serialize)]
pub struct ChatStartResult {
    pub conversation_id: i64,
    pub message_id: i64,
    pub rag_context: Option<crate::rag::RagContext>,
}

#[tauri::command]
pub async fn list_messages(
    db: State<'_, std::sync::Arc<tokio::sync::Mutex<Database>>>,
    conversation_id: i64,
) -> Result<Vec<crate::llm::Message>, String> {
    let db = db.inner().clone();
    crate::rag::load_history(&db, conversation_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_conversations(
    db: State<'_, std::sync::Arc<tokio::sync::Mutex<Database>>>,
) -> Result<Vec<ConversationSummary>, String> {
    let db_lock = db.lock().await;
    let conn = db_lock.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, COALESCE(NULLIF(title, ''), '新对话'), created_at, updated_at FROM conversations ORDER BY updated_at DESC LIMIT 100",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(ConversationSummary {
                id: r.get(0)?,
                title: r.get::<_, String>(1)?,
                created_at: r.get::<_, i64>(2).unwrap_or(0),
                updated_at: r.get::<_, i64>(3).unwrap_or(0),
            })
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

#[tauri::command]
pub async fn delete_conversation(
    db: State<'_, std::sync::Arc<tokio::sync::Mutex<Database>>>,
    conversation_id: i64,
) -> Result<(), String> {
    let db_lock = db.lock().await;
    let conn = db_lock.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "DELETE FROM messages WHERE conversation_id = ?1",
        rusqlite::params![conversation_id],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "DELETE FROM conversations WHERE id = ?1",
        rusqlite::params![conversation_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn rename_conversation(
    db: State<'_, std::sync::Arc<tokio::sync::Mutex<Database>>>,
    conversation_id: i64,
    title: String,
) -> Result<(), String> {
    let db_lock = db.lock().await;
    let conn = db_lock.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE conversations SET title = ?1, updated_at = strftime('%s','now') WHERE id = ?2",
        rusqlite::params![title, conversation_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(serde::Serialize)]
pub struct ConversationSummary {
    pub id: i64,
    pub title: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[tauri::command]
pub async fn test_llm(
    db: State<'_, std::sync::Arc<tokio::sync::Mutex<Database>>>,
) -> Result<String, String> {
    let cfg = {
        let db_lock = db.lock().await;
        let conn = db_lock.conn.lock().map_err(|e| e.to_string())?;
        let get = |k: &str| -> Option<String> {
            conn.query_row(
                "SELECT value FROM settings WHERE key = ?1",
                rusqlite::params![k],
                |r| r.get::<_, String>(0),
            )
            .ok()
        };
        ProviderConfig {
            provider: get("llm.provider").unwrap_or_else(|| "openai".to_string()),
            base_url: get("llm.base_url").unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
            api_key: get("llm.api_key").unwrap_or_default(),
            model: get("llm.model").unwrap_or_else(|| "gpt-4o-mini".to_string()),
        }
    };

    if cfg.api_key.is_empty() {
        return Err("请先填入 API Key".to_string());
    }

    let provider = OpenAIProvider::new(&cfg.base_url, &cfg.api_key, &cfg.model);
    let resp = provider
        .chat(ChatRequest {
            messages: vec![crate::llm::Message::user("回复\"OK\"两个字符")],
            model: None,
            temperature: Some(0.0),
            max_tokens: Some(10),
            tools: Vec::new(),
        })
        .await
        .map_err(|e| e.to_string())?;
    Ok(resp.content)
}

// ── Tags ──

#[derive(serde::Serialize)]
pub struct TagSummary {
    pub id: i64,
    pub name: String,
    pub color: Option<String>,
    pub file_count: i64,
}

#[tauri::command]
pub async fn list_tags(
    db: State<'_, std::sync::Arc<tokio::sync::Mutex<Database>>>,
) -> Result<Vec<TagSummary>, String> {
    let db_lock = db.lock().await;
    let conn = db_lock.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT t.id, t.name, t.color, (SELECT COUNT(*) FROM file_tags ft WHERE ft.tag_id = t.id)
             FROM tags t ORDER BY t.name",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(TagSummary {
                id: r.get(0)?,
                name: r.get(1)?,
                color: r.get(2)?,
                file_count: r.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

#[tauri::command]
pub async fn create_tag(
    db: State<'_, std::sync::Arc<tokio::sync::Mutex<Database>>>,
    name: String,
    color: Option<String>,
) -> Result<TagSummary, String> {
    let trimmed = name.trim().to_string();
    if trimmed.is_empty() {
        return Err("标签名不能为空".to_string());
    }
    let db_lock = db.lock().await;
    let conn = db_lock.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR IGNORE INTO tags (name, color) VALUES (?1, ?2)",
        rusqlite::params![&trimmed, &color],
    )
    .map_err(|e| e.to_string())?;
    let id: i64 = conn
        .query_row("SELECT id FROM tags WHERE name = ?1", rusqlite::params![&trimmed], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    Ok(TagSummary {
        id,
        name: trimmed,
        color,
        file_count: 0,
    })
}

#[tauri::command]
pub async fn delete_tag(
    db: State<'_, std::sync::Arc<tokio::sync::Mutex<Database>>>,
    tag_id: i64,
) -> Result<(), String> {
    let db_lock = db.lock().await;
    let conn = db_lock.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM tags WHERE id = ?1", rusqlite::params![tag_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_file_tags(
    db: State<'_, std::sync::Arc<tokio::sync::Mutex<Database>>>,
    file_id: i64,
) -> Result<Vec<TagSummary>, String> {
    let db_lock = db.lock().await;
    let conn = db_lock.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT t.id, t.name, t.color, (SELECT COUNT(*) FROM file_tags ft WHERE ft.tag_id = t.id)
             FROM tags t
             JOIN file_tags ft ON ft.tag_id = t.id
             WHERE ft.file_id = ?1
             ORDER BY t.name",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params![file_id], |r| {
            Ok(TagSummary {
                id: r.get(0)?,
                name: r.get(1)?,
                color: r.get(2)?,
                file_count: r.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

#[tauri::command]
pub async fn set_file_tags(
    db: State<'_, std::sync::Arc<tokio::sync::Mutex<Database>>>,
    file_id: i64,
    tag_ids: Vec<i64>,
) -> Result<(), String> {
    let db_lock = db.lock().await;
    let conn = db_lock.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM file_tags WHERE file_id = ?1", rusqlite::params![file_id])
        .map_err(|e| e.to_string())?;
    for tid in &tag_ids {
        conn.execute(
            "INSERT OR IGNORE INTO file_tags (file_id, tag_id) VALUES (?1, ?2)",
            rusqlite::params![file_id, tid],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[derive(serde::Serialize)]
pub struct Stats {
    pub files: u64,
    pub chunks: u64,
    pub tags: u64,
}

