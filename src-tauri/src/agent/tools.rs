use super::Tool;
use crate::db::Database;
use crate::llm::ToolDefinition;
use crate::search::{SearchFilter, Searcher};
use anyhow::{anyhow, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;

fn schema(properties: Value, required: &[&str]) -> Value {
    let required_arr: Vec<Value> = required.iter().map(|s| Value::String(s.to_string())).collect();
    json!({
        "type": "object",
        "properties": properties,
        "required": required_arr,
    })
}

pub struct SearchFilesTool {
    db: Arc<Mutex<Database>>,
}

impl SearchFilesTool {
    pub fn new(db: Arc<Mutex<Database>>) -> Self {
        Self { db }
    }
}

#[derive(Deserialize)]
struct SearchFilesArgs {
    query: String,
    #[serde(default)]
    ext: Option<String>,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    path_contains: Option<String>,
    #[serde(default)]
    limit: Option<i64>,
}

#[async_trait::async_trait]
impl Tool for SearchFilesTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "search_files".into(),
            description: "在用户已索引的文件中按关键词全文检索。返回匹配文件的路径、名称、扩展名、大小、修改时间和内容片段。".into(),
            parameters: schema(
                json!({
                    "query": {
                        "type": "string",
                        "description": "搜索关键词或自然语言查询"
                    },
                    "ext": {
                        "type": "string",
                        "description": "可选：扩展名过滤，例如 pdf、md"
                    },
                    "kind": {
                        "type": "string",
                        "description": "可选：文件类型过滤，例如 image、pdf、code"
                    },
                    "path_contains": {
                        "type": "string",
                        "description": "可选：路径必须包含的子串"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "可选：返回数量上限，默认 10，最大 30",
                        "default": 10
                    }
                }),
                &["query"],
            ),
        }
    }

    async fn execute(&self, args: &str) -> Result<String> {
        let parsed: SearchFilesArgs = if args.trim().is_empty() {
            return Err(anyhow!("missing query"));
        } else if let Ok(v) = serde_json::from_str::<Value>(args) {
            serde_json::from_value(v)?
        } else {
            return Err(anyhow!("invalid arguments json"));
        };

        let limit = parsed.limit.unwrap_or(10).clamp(1, 30);
        let filter = SearchFilter {
            ext: parsed.ext,
            kind: parsed.kind,
            path_contains: parsed.path_contains,
            since: None,
        };

        let (results, fts_query) = {
            let db_lock = self.db.lock().await;
            let conn = db_lock.conn.lock().map_err(|e| anyhow!("db mutex: {}", e))?;
            let searcher = Searcher::new();
            let words = searcher.tokenize_query(&parsed.query);
            if words.is_empty() {
                return Ok(json!({ "results": [], "count": 0 }).to_string());
            }
            let fts_query = words
                .iter()
                .map(|w| crate::search::escape_fts(w))
                .filter(|w| !w.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            let results = if fts_query.is_empty() {
                Vec::new()
            } else {
                searcher
                    .search_on_conn(&conn, &fts_query, filter, limit)
                    .map_err(|e| anyhow!("search failed: {}", e))?
            };
            (results, fts_query)
        };
        let _ = fts_query;

        let arr: Vec<Value> = results
            .into_iter()
            .map(|r| {
                json!({
                    "path": r.path,
                    "name": r.name,
                    "ext": r.ext,
                    "size": r.size,
                    "mtime": r.mtime,
                    "kind": r.kind,
                    "snippet": r.snippet,
                })
            })
            .collect();

        Ok(json!({ "results": arr, "count": arr.len() }).to_string())
    }
}

pub struct ReadFileTool {}

impl ReadFileTool {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(Deserialize)]
struct ReadFileArgs {
    path: String,
    #[serde(default)]
    max_bytes: Option<i64>,
}

#[async_trait::async_trait]
impl Tool for ReadFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "read_file".into(),
            description: "读取本地文本文件的内容预览（最多 32KB）。仅支持文本类文件（txt、md、code、json 等），二进制文件会返回错误。".into(),
            parameters: schema(
                json!({
                    "path": {
                        "type": "string",
                        "description": "文件绝对路径"
                    },
                    "max_bytes": {
                        "type": "integer",
                        "description": "可选：最多读取的字节数，默认 16384，上限 32768",
                        "default": 16384
                    }
                }),
                &["path"],
            ),
        }
    }

    async fn execute(&self, args: &str) -> Result<String> {
        let parsed: ReadFileArgs = if args.trim().is_empty() {
            return Err(anyhow!("missing path"));
        } else {
            serde_json::from_str(args)?
        };

        let p = std::path::Path::new(&parsed.path);
        if !p.exists() {
            return Err(anyhow!("file not found: {}", parsed.path));
        }
        let meta = std::fs::metadata(p)?;
        if meta.len() > 2 * 1024 * 1024 {
            return Err(anyhow!(
                "file too large for preview ({} bytes > 2MB)",
                meta.len()
            ));
        }

        let max = parsed.max_bytes.unwrap_or(16384).clamp(256, 32 * 1024) as usize;
        let mut file = std::fs::File::open(p)?;
        use std::io::Read;
        let mut buf = vec![0u8; max];
        let n = file.read(&mut buf)?;
        buf.truncate(n);
        let content = String::from_utf8_lossy(&buf).into_owned();

        Ok(json!({
            "path": parsed.path,
            "size": meta.len(),
            "previewed_bytes": n,
            "content": content,
        })
        .to_string())
    }
}

pub struct ListSimilarTool {
    db: Arc<Mutex<Database>>,
}

impl ListSimilarTool {
    pub fn new(db: Arc<Mutex<Database>>) -> Self {
        Self { db }
    }
}

#[derive(Deserialize)]
struct ListSimilarArgs {
    path: String,
    #[serde(default)]
    limit: Option<i64>,
}

#[async_trait::async_trait]
impl Tool for ListSimilarTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "list_similar".into(),
            description: "根据给定文件的路径，列出与之名称/扩展名/所在目录相似的其他已索引文件。适合用来发现相关文件或重复文件。".into(),
            parameters: schema(
                json!({
                    "path": {
                        "type": "string",
                        "description": "参考文件的绝对路径"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "可选：返回数量上限，默认 8，最大 20",
                        "default": 8
                    }
                }),
                &["path"],
            ),
        }
    }

    async fn execute(&self, args: &str) -> Result<String> {
        let parsed: ListSimilarArgs = if args.trim().is_empty() {
            return Err(anyhow!("missing path"));
        } else {
            serde_json::from_str(args)?
        };

        let limit = parsed.limit.unwrap_or(8).clamp(1, 20);
        let p = std::path::Path::new(&parsed.path);
        let name = p
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow!("invalid path"))?;
        let ext = p.extension().and_then(|s| s.to_str()).map(|s| s.to_string());
        let parent = p.parent().and_then(|s| s.to_str()).map(|s| s.to_string());

        let db_lock = self.db.lock().await;
        let conn = db_lock.conn.lock().map_err(|e| anyhow!("db mutex: {}", e))?;

        let mut sql = String::from(
            "SELECT id, path, name, ext, size, mtime, kind FROM files
             WHERE deleted_at IS NULL AND path != ?1",
        );
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(parsed.path.clone())];
        let mut idx = 2;

        if let Some(parent_path) = &parent {
            sql.push_str(&format!(" AND (path LIKE ?{} OR name LIKE ?{} OR ", idx, idx + 1));
            params_vec.push(Box::new(format!("{}%", parent_path)));
            params_vec.push(Box::new(format!("%{}%", name)));
            idx += 2;
        } else {
            sql.push_str(&format!(" AND (name LIKE ?{} ", idx));
            params_vec.push(Box::new(format!("%{}%", name)));
            idx += 1;
        }

        if let Some(e) = &ext {
            sql.push_str(&format!(" OR ext = ?{}", idx));
            params_vec.push(Box::new(e.clone()));
        }
        sql.push_str(")");

        sql.push_str(&format!(" LIMIT {}", limit));

        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(param_refs.as_slice(), |row| {
            Ok(json!({
                "id": row.get::<_, i64>(0)?,
                "path": row.get::<_, String>(1)?,
                "name": row.get::<_, String>(2)?,
                "ext": row.get::<_, Option<String>>(3)?,
                "size": row.get::<_, i64>(4)?,
                "mtime": row.get::<_, i64>(5)?,
                "kind": row.get::<_, Option<String>>(6)?,
            }))
        })?;

        let mut results: Vec<Value> = Vec::new();
        for r in rows {
            results.push(r?);
        }
        drop(stmt);
        drop(conn);
        drop(db_lock);

        Ok(json!({ "results": results, "count": results.len() }).to_string())
    }
}

pub struct MoveFileTool {
    db: Arc<Mutex<Database>>,
}

impl MoveFileTool {
    pub fn new(db: Arc<Mutex<Database>>) -> Self {
        Self { db }
    }
}

#[derive(Deserialize)]
struct MoveFileArgs {
    src: String,
    dst: String,
    #[serde(default)]
    overwrite: bool,
}

#[async_trait::async_trait]
impl Tool for MoveFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "move_file".into(),
            description:
                "移动或重命名本地文件。源路径和目标路径必须是绝对路径。默认不覆盖已有文件。".into(),
            parameters: schema(
                json!({
                    "src": {
                        "type": "string",
                        "description": "源文件绝对路径"
                    },
                    "dst": {
                        "type": "string",
                        "description": "目标路径（可为目标目录下的新文件名）"
                    },
                    "overwrite": {
                        "type": "boolean",
                        "description": "目标已存在时是否覆盖，默认 false",
                        "default": false
                    }
                }),
                &["src", "dst"],
            ),
        }
    }

    fn is_destructive(&self) -> bool {
        true
    }

    async fn execute(&self, args: &str) -> Result<String> {
        let parsed: MoveFileArgs = if args.trim().is_empty() {
            return Err(anyhow!("missing src/dst"));
        } else {
            serde_json::from_str(args)?
        };

        let src = std::path::Path::new(&parsed.src);
        let dst = std::path::Path::new(&parsed.dst);

        if !src.exists() {
            return Err(anyhow!("source not found: {}", parsed.src));
        }
        if !src.is_file() {
            return Err(anyhow!("source is not a regular file: {}", parsed.src));
        }
        if dst.exists() && !parsed.overwrite {
            return Err(anyhow!(
                "destination already exists: {} (set overwrite=true to replace)",
                parsed.dst
            ));
        }
        if let Some(parent) = dst.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let overwritten = parsed.overwrite && dst.exists();
        let src_size = src.metadata()?.len();
        std::fs::rename(src, dst)?;

        let db_update_ok = {
            let db_lock = self.db.lock().await;
            let conn_guard = db_lock.conn.lock();
            match conn_guard {
                Ok(conn) => conn
                    .execute(
                        "UPDATE files SET path = ?1 WHERE path = ?2",
                        rusqlite::params![&parsed.dst, &parsed.src],
                    )
                    .is_ok(),
                Err(_) => false,
            }
        };
        if !db_update_ok {
            tracing::warn!("files 表路径同步失败: {} -> {}", parsed.src, parsed.dst);
        }

        let payload = json!({
            "size": src_size,
            "overwritten": overwritten,
        })
        .to_string();
        let op_id = super::history::record(
            &self.db,
            "move",
            Some(&parsed.src),
            Some(&parsed.dst),
            Some(&payload),
        )
        .await
        .unwrap_or(-1);

        Ok(json!({
            "src": parsed.src,
            "dst": parsed.dst,
            "size": src_size,
            "overwritten": overwritten,
            "operation_id": op_id,
            "undoable": !overwritten,
        })
        .to_string())
    }
}

pub struct TagFileTool {
    db: Arc<Mutex<Database>>,
}

impl TagFileTool {
    pub fn new(db: Arc<Mutex<Database>>) -> Self {
        Self { db }
    }
}

#[derive(Deserialize)]
struct TagFileArgs {
    path: String,
    tags: Vec<String>,
}

#[async_trait::async_trait]
impl Tool for TagFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "tag_file".into(),
            description: "为已索引的文件打上一个或多个标签。已存在的标签会复用，不会重复创建。".into(),
            parameters: schema(
                json!({
                    "path": {
                        "type": "string",
                        "description": "文件绝对路径（必须已索引）"
                    },
                    "tags": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "标签名列表"
                    }
                }),
                &["path", "tags"],
            ),
        }
    }

    fn is_destructive(&self) -> bool {
        true
    }

    async fn execute(&self, args: &str) -> Result<String> {
        let parsed: TagFileArgs = if args.trim().is_empty() {
            return Err(anyhow!("missing path/tags"));
        } else {
            serde_json::from_str(args)?
        };

        if parsed.tags.is_empty() {
            return Err(anyhow!("tags must not be empty"));
        }

        let (file_id, attached) = {
            let db_lock = self.db.lock().await;
            let conn = db_lock.conn.lock().map_err(|e| anyhow!("db mutex: {}", e))?;

            let fid: i64 = conn
                .query_row(
                    "SELECT id FROM files WHERE path = ?1 AND deleted_at IS NULL LIMIT 1",
                    rusqlite::params![&parsed.path],
                    |r| r.get(0),
                )
                .map_err(|_| anyhow!("file not indexed: {}", parsed.path))?;

            let mut attached: Vec<String> = Vec::new();
            for tag in &parsed.tags {
                let trimmed = tag.trim();
                if trimmed.is_empty() {
                    continue;
                }
                conn.execute(
                    "INSERT OR IGNORE INTO tags (name) VALUES (?1)",
                    rusqlite::params![trimmed],
                )?;
                let tag_id: i64 = conn.query_row(
                    "SELECT id FROM tags WHERE name = ?1",
                    rusqlite::params![trimmed],
                    |r| r.get(0),
                )?;
                conn.execute(
                    "INSERT OR IGNORE INTO file_tags (file_id, tag_id) VALUES (?1, ?2)",
                    rusqlite::params![fid, tag_id],
                )?;
                attached.push(trimmed.to_string());
            }
            (fid, attached)
        };

        let payload = json!({
            "file_id": file_id,
            "attached": attached,
        })
        .to_string();
        let op_id = super::history::record(&self.db, "tag", Some(&parsed.path), None, Some(&payload))
            .await
            .unwrap_or(-1);

        Ok(json!({
            "path": parsed.path,
            "file_id": file_id,
            "tags": attached,
            "count": attached.len(),
            "operation_id": op_id,
            "undoable": true,
        })
        .to_string())
    }
}
