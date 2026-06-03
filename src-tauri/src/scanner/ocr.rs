use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Command;

const OCR_TIMEOUT_SECS: u64 = 30;

#[derive(Debug, Clone, Deserialize)]
pub struct OCRResult {
    pub text: String,
    pub confidence: f32,
    #[allow(dead_code)]
    pub observations: Vec<Observation>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Observation {
    pub text: String,
    pub confidence: f32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

pub fn ocr_binary_path() -> Result<PathBuf> {
    let candidates: Vec<PathBuf> = if let Some(p) = option_env!("OCR_BIN_PATH") {
        vec![PathBuf::from(p)]
    } else {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut v = vec![manifest_dir.join("bin").join("glean-ocr")];
        if let Ok(exe) = std::env::current_exe() {
            if let Some(parent) = exe.parent() {
                v.push(parent.join("glean-ocr"));
            }
        }
        v
    };

    for c in &candidates {
        if c.exists() {
            return Ok(c.clone());
        }
    }
    Ok(candidates.into_iter().next().unwrap_or_default())
}

pub async fn run_ocr(path: &Path) -> Result<OCRResult> {
    let binary = ocr_binary_path()?;
    if !binary.exists() {
        anyhow::bail!("ocr binary not found at {}", binary.display());
    }

    let output = Command::new(&binary)
        .arg(path)
        .output()
        .await
        .with_context(|| format!("failed to spawn {}", binary.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("ocr failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        return Ok(OCRResult {
            text: String::new(),
            confidence: 0.0,
            observations: Vec::new(),
        });
    }

    let parsed: OCRResult = serde_json::from_str(&stdout)
        .with_context(|| format!("failed to parse ocr output: {}", stdout.chars().take(200).collect::<String>()))?;
    Ok(parsed)
}

pub async fn run_ocr_with_timeout(path: &Path) -> Result<OCRResult> {
    match tokio::time::timeout(Duration::from_secs(OCR_TIMEOUT_SECS), run_ocr(path)).await {
        Ok(inner) => inner,
        Err(_) => anyhow::bail!("ocr timeout after {}s", OCR_TIMEOUT_SECS),
    }
}
