use crate::db::Database;
use crate::llm::{openai::OpenAIProvider, ChatRequest, LLMProvider, Message, ProviderConfig};
use crate::search::hybrid::hybrid_search;
use crate::search::SearchFilter;
use anyhow::Result;
use rusqlite::params;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;

static STOP_FLAG: AtomicBool = AtomicBool::new(false);

pub fn request_stop() {
    STOP_FLAG.store(true, Ordering::SeqCst);
}

pub fn is_stopped() -> bool {
    STOP_FLAG.load(Ordering::SeqCst)
}

pub fn reset_stop() {
    STOP_FLAG.store(false, Ordering::SeqCst);
}

const MAX_CONTEXT_CHUNKS: i64 = 6;
const MAX_CHARS_PER_CHUNK: usize = 1200;

#[derive(Debug, Clone, Serialize)]
pub struct RagContext {
    pub references: Vec<RagReference>,
    pub augmented_query: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RagReference {
    pub file_id: i64,
    pub path: String,
    pub name: String,
    pub snippet: Option<String>,
    pub score: f64,
    pub source: String,
}

pub fn build_provider(cfg: &ProviderConfig) -> Arc<dyn LLMProvider> {
    Arc::new(OpenAIProvider::new(&cfg.base_url, &cfg.api_key, &cfg.model))
}

pub async fn load_provider_config(db: &Arc<Mutex<Database>>) -> Result<ProviderConfig> {
    let db_lock = db.lock().await;
    let conn = db_lock.conn.lock().map_err(|e| anyhow::anyhow!("db mutex: {}", e))?;
    let get = |k: &str| -> Option<String> {
        conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![k],
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

pub async fn retrieve_context(
    db: &Arc<Mutex<Database>>,
    query: &str,
) -> Result<Option<RagContext>> {
    let db_lock = db.lock().await;
    let hits = hybrid_search(
        &db_lock.conn,
        query,
        SearchFilter::default(),
        MAX_CONTEXT_CHUNKS,
    )?;
    drop(db_lock);

    if hits.is_empty() {
        return Ok(None);
    }

    let mut refs = Vec::new();
    let mut context_blocks = Vec::new();
    for (i, h) in hits.iter().enumerate() {
        let snippet = h
            .base
            .snippet
            .as_deref()
            .map(|s| truncate(s, MAX_CHARS_PER_CHUNK))
            .unwrap_or_default();
        context_blocks.push(format!("[{}] ({})\n{}", i + 1, h.base.name, snippet));
        refs.push(RagReference {
            file_id: h.base.id,
            path: h.base.path.clone(),
            name: h.base.name.clone(),
            snippet: h.base.snippet.clone(),
            score: h.base.rank,
            source: match h.source {
                crate::search::hybrid::HitSource::Both => "both",
                crate::search::hybrid::HitSource::VectorOnly => "vector",
                crate::search::hybrid::HitSource::FtsOnly => "fts",
            }
            .to_string(),
        });
    }

    let context_text = context_blocks.join("\n\n");
    let augmented = format!(
        "请根据以下检索到的本地文件内容回答用户问题。引用时使用 [序号] 标注。\n\n\
         === 检索结果 ===\n\
         {}\n\
         === 检索结果结束 ===\n\n\
         用户问题：{}",
        context_text, query
    );

    Ok(Some(RagContext {
        references: refs,
        augmented_query: augmented,
    }))
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let cut: String = s.chars().take(max).collect();
    format!("{}…", cut)
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatChunkEvent {
    pub conversation_id: i64,
    pub message_id: i64,
    pub delta: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatDoneEvent {
    pub conversation_id: i64,
    pub message_id: i64,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub error: Option<String>,
}

pub async fn ensure_conversation(
    db: &Arc<Mutex<Database>>,
    conversation_id: Option<i64>,
    title: &str,
) -> Result<i64> {
    let db_lock = db.lock().await;
    let conn = db_lock.conn.lock().map_err(|e| anyhow::anyhow!("db mutex: {}", e))?;
    if let Some(id) = conversation_id {
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM conversations WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if exists > 0 {
            return Ok(id);
        }
    }
    conn.execute(
        "INSERT INTO conversations (title) VALUES (?1)",
        params![title],
    )?;
    let id: i64 = conn.query_row(
        "SELECT last_insert_rowid()",
        [],
        |r| r.get(0),
    )?;
    Ok(id)
}

pub async fn append_message(
    db: &Arc<Mutex<Database>>,
    conversation_id: i64,
    role: &str,
    content: &str,
) -> Result<i64> {
    let db_lock = db.lock().await;
    let conn = db_lock.conn.lock().map_err(|e| anyhow::anyhow!("db mutex: {}", e))?;
    conn.execute(
        "INSERT INTO messages (conversation_id, role, content) VALUES (?1, ?2, ?3)",
        params![conversation_id, role, content],
    )?;
    let id: i64 = conn.query_row(
        "SELECT last_insert_rowid()",
        [],
        |r| r.get(0),
    )?;
    conn.execute(
        "UPDATE conversations SET updated_at = strftime('%s','now') WHERE id = ?1",
        params![conversation_id],
    )?;
    Ok(id)
}

pub async fn load_history(
    db: &Arc<Mutex<Database>>,
    conversation_id: i64,
) -> Result<Vec<Message>> {
    let db_lock = db.lock().await;
    let conn = db_lock.conn.lock().map_err(|e| anyhow::anyhow!("db mutex: {}", e))?;
    let mut stmt = conn.prepare(
        "SELECT role, content FROM messages WHERE conversation_id = ?1 ORDER BY id ASC",
    )?;
    let rows = stmt.query_map(params![conversation_id], |r| {
        Ok(Message {
            role: r.get::<_, String>(0)?,
            content: r.get::<_, String>(1)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub async fn run_chat(
    app: AppHandle,
    db: Arc<Mutex<Database>>,
    conversation_id: Option<i64>,
    user_message: String,
    use_rag: bool,
) -> Result<(i64, i64, Option<RagContext>)> {
    let cfg = load_provider_config(&db).await?;
    if cfg.api_key.is_empty() {
        anyhow::bail!("请先在设置中填入 API Key");
    }

    let conv_id = ensure_conversation(&db, conversation_id, &user_message).await?;
    append_message(&db, conv_id, "user", &user_message).await?;

    let rag_ctx = if use_rag {
        retrieve_context(&db, &user_message).await?
    } else {
        None
    };

    let mut history = load_history(&db, conv_id).await?;
    if let Some(rag) = &rag_ctx {
        if let Some(last) = history.last_mut() {
            if last.role == "user" {
                last.content = rag.augmented_query.clone();
            }
        }
    }

    let provider = build_provider(&cfg);
    let assistant_id = append_message(&db, conv_id, "assistant", "").await?;

    reset_stop();

    let chunk_event = ChatChunkEvent {
        conversation_id: conv_id,
        message_id: assistant_id,
        delta: String::new(),
    };
    let done_event = ChatDoneEvent {
        conversation_id: conv_id,
        message_id: assistant_id,
        input_tokens: None,
        output_tokens: None,
        error: None,
    };

    let app_clone = app.clone();
    let req = ChatRequest {
        messages: history,
        model: None,
        temperature: Some(0.3),
        max_tokens: Some(2048),
    };

    let result = provider
        .chat_stream(req, Box::new(move |delta: String| {
            if is_stopped() {
                return;
            }
            let evt = ChatChunkEvent {
                conversation_id: chunk_event.conversation_id,
                message_id: chunk_event.message_id,
                delta,
            };
            let _ = app_clone.emit("chat-delta", evt);
        }))
        .await;

    let stopped = is_stopped();

    let final_content;
    let mut input_tokens = None;
    let mut output_tokens = None;
    let mut err: Option<String> = None;

    match result {
        Ok(resp) => {
            final_content = resp.content;
            input_tokens = resp.input_tokens;
            output_tokens = resp.output_tokens;
        }
        Err(e) => {
            err = Some(e.to_string());
            final_content = format!("[错误] {}", err.as_ref().unwrap());
        }
    }

    {
        let db_lock = db.lock().await;
        let conn_guard = db_lock.conn.lock();
        if let Ok(conn) = conn_guard {
            let _ = conn.execute(
                "UPDATE messages SET content = ?1, tokens = ?2 WHERE id = ?3",
                params![&final_content, output_tokens, assistant_id],
            );
        }
    }

    let done = ChatDoneEvent {
        input_tokens,
        output_tokens,
        error: err.clone().or_else(|| {
            if stopped {
                Some("stopped".to_string())
            } else {
                None
            }
        }),
        ..done_event
    };
    let _ = app.emit("chat-done", done);

    Ok((conv_id, assistant_id, rag_ctx))
}
