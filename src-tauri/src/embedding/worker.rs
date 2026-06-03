use crate::db::Database;
use crate::embedding::{EmbeddingService, EMBED_DIM};
use anyhow::Result;
use rusqlite::params;
use serde::Serialize;
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;
use tracing::{error, info, warn};

fn vec_to_bytes(vec: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(vec.len() * 4);
    for f in vec {
        bytes.extend_from_slice(&f.to_le_bytes());
    }
    bytes
}

const BATCH_SIZE: usize = 16;
const SLEEP_BETWEEN_BATCHES_MS: u64 = 50;

#[derive(Debug, Clone, Serialize)]
pub struct EmbedProgress {
    pub embedded: u64,
    pub total: u64,
    pub current_chunk: Option<i64>,
    pub phase: EmbedPhase,
}

#[derive(Debug, Clone, Serialize)]
pub enum EmbedPhase {
    Idle,
    Downloading,
    Embedding,
    Completed,
    Failed,
}

pub struct EmbeddingWorker {
    db: Arc<Mutex<Database>>,
    running: Arc<std::sync::atomic::AtomicBool>,
    vec_table_ready: Arc<std::sync::OnceLock<()>>,
}

impl EmbeddingWorker {
    pub fn new(db: Arc<Mutex<Database>>) -> Self {
        Self {
            db,
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            vec_table_ready: Arc::new(std::sync::OnceLock::new()),
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub async fn run_once(&self, app: AppHandle) -> Result<()> {
        if self
            .running
            .swap(true, std::sync::atomic::Ordering::SeqCst)
        {
            return Ok(());
        }

        let _guard = CancelGuard(self.running.clone());

        let _ = app.emit(
            "embedding-progress",
            EmbedProgress {
                embedded: 0,
                total: 0,
                current_chunk: None,
                phase: EmbedPhase::Downloading,
            },
        );

        if let Err(e) = EmbeddingService::init() {
            error!("embedding model init failed: {}", e);
            let _ = app.emit(
                "embedding-progress",
                EmbedProgress {
                    embedded: 0,
                    total: 0,
                    current_chunk: None,
                    phase: EmbedPhase::Failed,
                },
            );
            return Err(e);
        }

        self.ensure_vec_table().await?;

        let _ = app.emit(
            "embedding-progress",
            EmbedProgress {
                embedded: 0,
                total: 0,
                current_chunk: None,
                phase: EmbedPhase::Embedding,
            },
        );

        let start = Instant::now();
        let mut embedded_total: u64 = 0;

        loop {
            if !self.running.load(std::sync::atomic::Ordering::SeqCst) {
                break;
            }

            let batch = self.fetch_pending_batch(BATCH_SIZE).await?;
            if batch.is_empty() {
                break;
            }

            let total_remaining = self.count_pending().await?;
            let texts: Vec<String> = batch.iter().map(|b| b.content.clone()).collect();
            let ids: Vec<i64> = batch.iter().map(|b| b.chunk_id).collect();

            let embeddings = match EmbeddingService::embed_batch(texts.clone()) {
                Ok(v) => v,
                Err(e) => {
                    error!("embed batch failed: {}", e);
                    self.mark_failed(&ids).await?;
                    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                    continue;
                }
            };

            self.write_embeddings(&ids, &embeddings).await?;
            embedded_total += embeddings.len() as u64;

            let _ = app.emit(
                "embedding-progress",
                EmbedProgress {
                    embedded: embedded_total,
                    total: embedded_total + total_remaining,
                    current_chunk: Some(ids[ids.len() / 2]),
                    phase: EmbedPhase::Embedding,
                },
            );

            if SLEEP_BETWEEN_BATCHES_MS > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(SLEEP_BETWEEN_BATCHES_MS))
                    .await;
            }
        }

        let dur = start.elapsed().as_millis();
        info!("embedding pass: {} chunks in {}ms", embedded_total, dur);
        let _ = app.emit(
            "embedding-progress",
            EmbedProgress {
                embedded: embedded_total,
                total: embedded_total,
                current_chunk: None,
                phase: EmbedPhase::Completed,
            },
        );
        Ok(())
    }

    async fn ensure_vec_table(&self) -> Result<()> {
        if self.vec_table_ready.get().is_some() {
            return Ok(());
        }
        let db = self.db.lock().await;
        let conn = db.conn.lock().map_err(|e| anyhow::anyhow!("db mutex: {}", e))?;
        conn.execute_batch(&format!(
            "CREATE VIRTUAL TABLE IF NOT EXISTS chunks_vec USING vec0(
                embedding float[{}]
            );",
            EMBED_DIM
        ))?;
        let _ = self.vec_table_ready.set(());
        Ok(())
    }

    async fn fetch_pending_batch(&self, limit: usize) -> Result<Vec<PendingChunk>> {
        let db = self.db.lock().await;
        let conn = db.conn.lock().map_err(|e| anyhow::anyhow!("db mutex: {}", e))?;
        let mut stmt = conn.prepare(
            "SELECT c.id, c.content
             FROM chunks c
             WHERE c.embedding_status = 'pending'
             ORDER BY c.id ASC
             LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok(PendingChunk {
                chunk_id: row.get(0)?,
                content: row.get(1)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    async fn count_pending(&self) -> Result<u64> {
        let db = self.db.lock().await;
        let conn = db.conn.lock().map_err(|e| anyhow::anyhow!("db mutex: {}", e))?;
        let n: i64 = conn.query_row(
            "SELECT COUNT(*) FROM chunks WHERE embedding_status = 'pending'",
            [],
            |r| r.get(0),
        )?;
        Ok(n as u64)
    }

    async fn write_embeddings(&self, ids: &[i64], embeddings: &[Vec<f32>]) -> Result<()> {
        let db = self.db.lock().await;
        let mut conn = db.conn.lock().map_err(|e| anyhow::anyhow!("db mutex: {}", e))?;
        let tx = conn.transaction()?;
        {
            let mut del_stmt =
                tx.prepare("DELETE FROM chunks_vec WHERE rowid = ?1")?;
            for id in ids {
                del_stmt.execute(params![id])?;
            }
            let mut ins_stmt = tx.prepare(
                "INSERT INTO chunks_vec (rowid, embedding) VALUES (?1, ?2)",
            )?;
            for (id, vec) in ids.iter().zip(embeddings.iter()) {
                ins_stmt.execute(params![id, vec_to_bytes(vec)])?;
            }
            let mut upd_stmt =
                tx.prepare("UPDATE chunks SET embedding_status = 'ready' WHERE id = ?1")?;
            for id in ids {
                upd_stmt.execute(params![id])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    async fn mark_failed(&self, ids: &[i64]) -> Result<()> {
        let db = self.db.lock().await;
        let conn = db.conn.lock().map_err(|e| anyhow::anyhow!("db mutex: {}", e))?;
        for id in ids {
            let _ = conn.execute(
                "UPDATE chunks SET embedding_status = 'failed' WHERE id = ?1",
                params![id],
            );
        }
        Ok(())
    }
}

struct PendingChunk {
    chunk_id: i64,
    content: String,
}

struct CancelGuard(Arc<std::sync::atomic::AtomicBool>);

impl Drop for CancelGuard {
    fn drop(&mut self) {
        self.0.store(false, std::sync::atomic::Ordering::SeqCst);
    }
}

pub fn spawn_worker(
    db: Arc<Mutex<Database>>,
    app: AppHandle,
) -> Arc<EmbeddingWorker> {
    let worker = Arc::new(EmbeddingWorker::new(db));
    let w = worker.clone();
    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(10));
        ticker.tick().await;
        loop {
            ticker.tick().await;
            if w.is_running() {
                continue;
            }
            let pending = match w.count_pending().await {
                Ok(n) => n,
                Err(e) => {
                    warn!("count pending chunks failed: {}", e);
                    continue;
                }
            };
            if pending == 0 {
                continue;
            }
            if let Err(e) = w.run_once(app_clone.clone()).await {
                error!("embedding worker tick failed: {}", e);
            }
        }
    });
    worker
}
