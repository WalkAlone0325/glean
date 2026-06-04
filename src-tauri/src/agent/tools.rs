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

        let db_lock = self.db.lock().await;
        let searcher = Searcher::new();
        let results = searcher
            .search(&db_lock.conn, &parsed.query, filter, limit)
            .map_err(|e| anyhow!("search failed: {}", e))?;
        drop(db_lock);

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
