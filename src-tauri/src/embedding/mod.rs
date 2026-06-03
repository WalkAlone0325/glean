pub mod worker;

use anyhow::Result;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use std::sync::{Mutex, OnceLock};

pub const EMBED_DIM: usize = 384;

static EMBEDDER: OnceLock<Mutex<TextEmbedding>> = OnceLock::new();

pub struct EmbeddingService;

impl EmbeddingService {
    pub fn init() -> Result<()> {
        if EMBEDDER.get().is_some() {
            return Ok(());
        }

        let endpoint = std::env::var("GLEAN_HF_ENDPOINT")
            .unwrap_or_else(|_| "https://hf-mirror.com".to_string());
        std::env::set_var("HF_ENDPOINT", &endpoint);
        tracing::info!("using huggingface endpoint: {}", endpoint);

        let cache_dir = std::env::var("GLEAN_CACHE_DIR").ok().filter(|s| !s.is_empty());

        let mut opts = InitOptions::new(EmbeddingModel::BGESmallENV15)
            .with_show_download_progress(true);
        if let Some(dir) = cache_dir.clone() {
            opts = opts.with_cache_dir(std::path::PathBuf::from(dir));
        }

        let model = TextEmbedding::try_new(opts)?;
        let _ = EMBEDDER.set(Mutex::new(model));
        tracing::info!("embedding model loaded (cache: {:?})", cache_dir);
        Ok(())
    }

    pub fn embed_batch(texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        if EMBEDDER.get().is_none() {
            Self::init()?;
        }
        let mtx = EMBEDDER.get().expect("embedder initialized");
        let mut model = mtx.lock().map_err(|e| anyhow::anyhow!("embed mutex: {}", e))?;
        let embeddings = model.embed(texts, Some(32))?;
        Ok(embeddings)
    }

    pub fn embed_one(text: &str) -> Result<Vec<f32>> {
        let out = Self::embed_batch(vec![text.to_string()])?;
        Ok(out.into_iter().next().unwrap_or_default())
    }

    pub fn model_name() -> &'static str {
        "BAAI/bge-small-en-v1.5"
    }
}
