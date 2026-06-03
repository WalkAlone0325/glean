use super::walker::Walker;
use crate::db::Database;
use anyhow::Result;
use rusqlite::params;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;
use tracing::{error, info, warn};

type FileBatch = Vec<(String, String, Option<String>, Option<String>, i64, u64, i64)>;

#[derive(Debug, Clone, Serialize)]
pub struct IndexProgress {
    pub current: u64,
    pub total: u64,
    pub current_path: Option<String>,
    pub phase: IndexPhase,
    pub started_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub enum IndexPhase {
    Discovering,
    Indexing,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize)]
pub enum IndexEvent {
    Progress(IndexProgress),
    FileIndexed { path: String, hash: Option<String> },
    Error { path: String, message: String },
    Completed { total: u64, duration_ms: u128 },
}

pub struct Scheduler {
    db: Arc<Mutex<Database>>,
    running: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
}

impl Scheduler {
    pub fn new(db: Arc<Mutex<Database>>) -> Self {
        Self {
            db,
            running: Arc::new(AtomicBool::new(false)),
            paused: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }

    pub fn pause(&self) {
        self.paused.store(true, Ordering::SeqCst);
    }

    pub fn resume(&self) {
        self.paused.store(false, Ordering::SeqCst);
    }

    pub fn cancel(&self) {
        self.running.store(false, Ordering::SeqCst);
        self.paused.store(false, Ordering::SeqCst);
    }

    pub async fn index_paths(&self, app: AppHandle, roots: Vec<PathBuf>) -> Result<()> {
        if self.running.swap(true, Ordering::SeqCst) {
            warn!("indexing already in progress, skipping");
            return Ok(());
        }

        let started = chrono::Utc::now().timestamp();
        let walker = Walker::new(super::IgnoreRules::default());

        let _guard = CancelGuard(self.running.clone());

        let phase_progress = |phase: IndexPhase, current: u64, total: u64, path: Option<String>| {
            IndexProgress {
                current,
                total,
                current_path: path,
                phase,
                started_at: started,
            }
        };

        let _ = app.emit(
            "index-progress",
            IndexEvent::Progress(phase_progress(IndexPhase::Discovering, 0, 0, None)),
        );

        let files = walker.collect(&roots);
        let total = files.len() as u64;
        info!("discovered {} files", total);

        let _ = app.emit(
            "index-progress",
            IndexEvent::Progress(phase_progress(IndexPhase::Indexing, 0, total, None)),
        );

        let start = Instant::now();
        let mut count = 0u64;
        let mut errors = 0u64;
        let batch_size = 50;
        let mut batch: FileBatch = Vec::new();

        for (i, path) in files.iter().enumerate() {
            while self.paused.load(Ordering::SeqCst) && self.running.load(Ordering::SeqCst) {
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }
            if !self.running.load(Ordering::SeqCst) {
                let _ = app.emit(
                    "index-progress",
                    IndexEvent::Progress(phase_progress(
                        IndexPhase::Cancelled,
                        i as u64,
                        total,
                        None,
                    )),
                );
                return Ok(());
            }

            let path_str = path.to_string_lossy().to_string();
            let metadata = match walker.extract(path) {
                Ok(m) => m,
                Err(e) => {
                    errors += 1;
                    let _ = app.emit(
                        "index-error",
                        IndexEvent::Error {
                            path: path_str,
                            message: e.to_string(),
                        },
                    );
                    continue;
                }
            };

            let hash = if metadata.size < 50 * 1024 * 1024 {
                super::hash::hash_file(path).ok()
            } else {
                None
            };

            batch.push((
                metadata.path.clone(),
                metadata.name.clone(),
                metadata.ext.clone(),
                metadata.kind.clone(),
                metadata.mtime,
                metadata.size,
                chrono::Utc::now().timestamp(),
            ));

            count += 1;
            let _ = app.emit(
                "index-file",
                IndexEvent::FileIndexed {
                    path: path_str.clone(),
                    hash: hash.clone(),
                },
            );

            if batch.len() >= batch_size {
                if let Err(e) = self.flush_batch(&batch).await {
                    error!("batch flush failed: {}", e);
                    errors += batch.len() as u64;
                }
                batch.clear();
            }

            if count.is_multiple_of(100) {
                let _ = app.emit(
                    "index-progress",
                    IndexEvent::Progress(phase_progress(
                        IndexPhase::Indexing,
                        count,
                        total,
                        Some(path_str),
                    )),
                );
            }
        }

        if !batch.is_empty() {
            if let Err(e) = self.flush_batch(&batch).await {
                error!("final flush failed: {}", e);
            }
        }

        let duration_ms = start.elapsed().as_millis();
        info!(
            "indexed {} files in {}ms (errors: {})",
            count, duration_ms, errors
        );

        let _ = app.emit(
            "index-complete",
            IndexEvent::Completed {
                total: count,
                duration_ms,
            },
        );
        let _ = app.emit(
            "index-progress",
            IndexEvent::Progress(phase_progress(IndexPhase::Completed, count, total, None)),
        );

        Ok(())
    }

    async fn flush_batch(&self, batch: &FileBatch) -> Result<()> {
        let db = self.db.lock().await;
        let conn = db.conn.lock().map_err(|e| anyhow::anyhow!("db mutex: {}", e))?;

        let tx = conn.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare_cached(
                "INSERT OR REPLACE INTO files (path, name, ext, size, mtime, hash, kind, indexed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, ?7)",
            )?;
            for (path, name, ext, kind, mtime, size, indexed_at) in batch {
                let size_i64 = i64::try_from(*size).unwrap_or(i64::MAX);
                stmt.execute(params![path, name, ext, size_i64, mtime, kind, indexed_at])?;
            }
            let mut fts_stmt = tx.prepare_cached(
                "INSERT OR REPLACE INTO files_fts (rowid, name, content)
                 SELECT id, name, '' FROM files WHERE path = ?1",
            )?;
            for (path, _, _, _, _, _, _) in batch {
                fts_stmt.execute(params![path])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub async fn index_single(&self, path: PathBuf) -> Result<()> {
        let walker = Walker::new(super::IgnoreRules::default());
        if walker.is_ignored_path(&path) {
            return Ok(());
        }
        let meta = walker.extract(&path)?;
        let hash = if meta.size < 50 * 1024 * 1024 {
            super::hash::hash_file(&path).ok()
        } else {
            None
        };

        let content = super::extract_text(&path, &meta.ext).await;
        let size_i64 = i64::try_from(meta.size).unwrap_or(i64::MAX);

        let db = self.db.lock().await;
        let conn = db.conn.lock().map_err(|e| anyhow::anyhow!("db mutex: {}", e))?;
        let now = chrono::Utc::now().timestamp();

        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "INSERT OR REPLACE INTO files (path, name, ext, size, mtime, hash, kind, indexed_at, deleted_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL)",
            params![meta.path, meta.name, meta.ext, size_i64, meta.mtime, hash, meta.kind, now],
        )?;

        let file_id: i64 = tx.query_row(
            "SELECT id FROM files WHERE path = ?1",
            params![meta.path],
            |r| r.get(0),
        )?;

        if let Some(text) = content.clone() {
            tx.execute("DELETE FROM files_fts WHERE rowid = ?1", params![file_id])?;
            tx.execute(
                "INSERT INTO files_fts (rowid, name, content) VALUES (?1, ?2, ?3)",
                params![file_id, meta.name, text],
            )?;

            tx.execute("DELETE FROM chunks WHERE file_id = ?1", params![file_id])?;
            let chunks = super::chunk_file(&path, meta.kind.as_deref(), &text)?;
            let mut chunk_stmt = tx.prepare(
                "INSERT INTO chunks (file_id, content, page, position, token_count, embedding_status)
                 VALUES (?1, ?2, ?3, ?4, ?5, 'pending')",
            )?;
            for c in chunks {
                chunk_stmt.execute(params![
                    file_id,
                    c.content,
                    c.page,
                    c.position,
                    c.token_count,
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub async fn mark_deleted(&self, path: &str) -> Result<()> {
        let db = self.db.lock().await;
        let conn = db.conn.lock().map_err(|e| anyhow::anyhow!("db mutex: {}", e))?;
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "UPDATE files SET deleted_at = ?1 WHERE path = ?2",
            params![now, path],
        )?;
        Ok(())
    }
}

struct CancelGuard(Arc<AtomicBool>);

impl Drop for CancelGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

