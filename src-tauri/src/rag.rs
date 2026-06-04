use crate::agent::{ConfirmationRegistry, ToolRegistry};
use crate::db::Database;
use crate::llm::{openai::OpenAIProvider, ChatRequest, LLMProvider, Message, ProviderConfig};
use crate::search::hybrid::hybrid_search;
use crate::search::SearchFilter;
use anyhow::Result;
use rusqlite::params;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
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
            tool_calls: Vec::new(),
            tool_call_id: None,
            name: None,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

const MAX_AGENT_ITERATIONS: usize = 5;

#[derive(Debug, Clone, Serialize)]
struct AgentToolCallEvent {
    conversation_id: i64,
    message_id: i64,
    call_id: String,
    name: String,
    arguments: String,
}

#[derive(Debug, Clone, Serialize)]
struct AgentToolResultEvent {
    conversation_id: i64,
    message_id: i64,
    call_id: String,
    result: String,
    error: Option<String>,
    duration_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
struct AgentToolConfirmEvent {
    conversation_id: i64,
    message_id: i64,
    call_id: String,
    name: String,
    arguments: String,
}

fn build_system_prompt(rag_ctx: &Option<RagContext>) -> String {
    let mut sys = String::from(
        "你是 Glean 的文件助理，运行在用户的 macOS 设备上。你可以通过工具读取用户已索引的本地文件、检索内容、查找相似文件。\n\n\
         工具使用准则：\n\
         - 优先使用 search_files 检索相关文件，再用 read_file 查看具体内容。\n\
         - 如果用户问的是某个已知路径的相关文件，使用 list_similar。\n\
         - 工具结果中的 mtime 是 Unix 时间戳（秒）。\n\
         - 回答用中文，引用文件路径时使用完整路径。\n\
         - 如果工具返回 0 条结果，明确告诉用户没找到，不要编造。",
    );
    if let Some(rag) = rag_ctx {
        sys.push_str("\n\n[检索到的相关上下文]\n");
        sys.push_str(&rag.augmented_query);
    }
    sys
}

pub async fn run_chat(
    app: AppHandle,
    db: Arc<Mutex<Database>>,
    confirmations: Arc<ConfirmationRegistry>,
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

    let history = load_history(&db, conv_id).await?;
    let system_msg = Message::system(build_system_prompt(&rag_ctx));

    let provider = build_provider(&cfg);
    let registry = ToolRegistry::new(db.clone());
    let tool_defs = registry.definitions();

    let assistant_id = append_message(&db, conv_id, "assistant", "").await?;

    reset_stop();

    let mut messages: Vec<Message> = vec![system_msg];
    messages.extend(history);

    let mut final_content = String::new();
    let mut total_input_tokens: Option<u32> = None;
    let mut total_output_tokens: Option<u32> = None;
    let mut err: Option<String> = None;
    let mut stopped = false;

    for iteration in 0..MAX_AGENT_ITERATIONS {
        if is_stopped() {
            stopped = true;
            break;
        }

        let req = ChatRequest {
            messages: messages.clone(),
            model: None,
            temperature: Some(0.3),
            max_tokens: Some(2048),
            tools: tool_defs.clone(),
        };

        let app_clone = app.clone();
        let assistant_id_for_cb = assistant_id;
        let conv_id_for_cb = conv_id;
        let is_first_iteration = iteration == 0;

        let result = provider
            .chat_stream(req, Box::new(move |delta: String| {
                if is_stopped() {
                    return;
                }
                if !is_first_iteration {
                    return;
                }
                let evt = ChatChunkEvent {
                    conversation_id: conv_id_for_cb,
                    message_id: assistant_id_for_cb,
                    delta,
                };
                let _ = app_clone.emit("chat-delta", evt);
            }))
            .await;

        match result {
            Ok(resp) => {
                if let Some(n) = resp.input_tokens {
                    total_input_tokens = Some(total_input_tokens.unwrap_or(0) + n);
                }
                if let Some(n) = resp.output_tokens {
                    total_output_tokens = Some(total_output_tokens.unwrap_or(0) + n);
                }

                if resp.tool_calls.is_empty() {
                    final_content = resp.content;
                    break;
                }

                let assistant_msg = Message::assistant_with_tools(resp.content.clone(), resp.tool_calls.clone());
                messages.push(assistant_msg);

                if !resp.content.is_empty() && iteration == 0 {
                    final_content = resp.content.clone();
                }

                for tc in &resp.tool_calls {
                    if is_stopped() {
                        stopped = true;
                        break;
                    }
                    let evt = AgentToolCallEvent {
                        conversation_id: conv_id,
                        message_id: assistant_id,
                        call_id: tc.id.clone(),
                        name: tc.name.clone(),
                        arguments: tc.arguments.clone(),
                    };
                    let _ = app.emit("agent-tool-call", evt);

                    let tool_ref = registry.get(&tc.name);
                    let is_destructive = tool_ref.as_ref().map(|t| t.is_destructive()).unwrap_or(false);

                    let start = Instant::now();
                    let exec_result = if is_destructive {
                        let confirm_evt = AgentToolConfirmEvent {
                            conversation_id: conv_id,
                            message_id: assistant_id,
                            call_id: tc.id.clone(),
                            name: tc.name.clone(),
                            arguments: tc.arguments.clone(),
                        };
                        let _ = app.emit("agent-tool-confirm", confirm_evt);

                        let rx = confirmations.register(conv_id, tc.id.clone()).await;
                        let approved = match rx.await {
                            Ok(v) => v,
                            Err(_) => false,
                        };

                        if !approved {
                            Err(anyhow::anyhow!("user denied tool execution"))
                        } else if let Some(tool) = tool_ref {
                            tool.execute(&tc.arguments).await
                        } else {
                            Err(anyhow::anyhow!("unknown tool: {}", tc.name))
                        }
                    } else {
                        match tool_ref {
                            Some(tool) => tool.execute(&tc.arguments).await,
                            None => Err(anyhow::anyhow!("unknown tool: {}", tc.name)),
                        }
                    };
                    let duration = start.elapsed().as_millis() as u64;

                    let (result_str, error_str) = match exec_result {
                        Ok(s) => (s, None),
                        Err(e) => (String::new(), Some(e.to_string())),
                    };

                    let result_evt = AgentToolResultEvent {
                        conversation_id: conv_id,
                        message_id: assistant_id,
                        call_id: tc.id.clone(),
                        result: truncate_tool(&result_str, 4000),
                        error: error_str.clone(),
                        duration_ms: duration,
                    };
                    let _ = app.emit("agent-tool-result", result_evt);

                    let tool_msg_content = if let Some(e) = &error_str {
                        serde_json::json!({ "error": e }).to_string()
                    } else if result_str.is_empty() {
                        "(empty)".to_string()
                    } else {
                        result_str.clone()
                    };
                    messages.push(Message::tool_result(&tc.id, tool_msg_content));
                }
            }
            Err(e) => {
                err = Some(e.to_string());
                break;
            }
        }

        if iteration == MAX_AGENT_ITERATIONS - 1 {
            err = Some("agent loop exceeded max iterations".into());
        }
    }

    if final_content.is_empty() {
        if let Some(e) = &err {
            final_content = format!("[错误] {}", e);
        }
    }

    {
        let db_lock = db.lock().await;
        let conn_guard = db_lock.conn.lock();
        if let Ok(conn) = conn_guard {
            let _ = conn.execute(
                "UPDATE messages SET content = ?1, tokens = ?2 WHERE id = ?3",
                params![&final_content, total_output_tokens, assistant_id],
            );
        }
    }

    let done = ChatDoneEvent {
        conversation_id: conv_id,
        message_id: assistant_id,
        input_tokens: total_input_tokens,
        output_tokens: total_output_tokens,
        error: err.clone().or_else(|| {
            if stopped {
                Some("stopped".to_string())
            } else {
                None
            }
        }),
    };
    let _ = app.emit("chat-done", done);

    Ok((conv_id, assistant_id, rag_ctx))
}

fn truncate_tool(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        let mut cut = max;
        while !s.is_char_boundary(cut) && cut > 0 {
            cut -= 1;
        }
        format!("{}...[truncated]", &s[..cut])
    }
}
