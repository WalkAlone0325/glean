use anyhow::Result;
use serde::Serialize;
use std::path::Path;
use std::process::Command;
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone, Serialize)]
pub struct FileMetadata {
    pub path: String,
    pub name: String,
    pub ext: Option<String>,
    pub size: u64,
    pub mtime: i64,
    pub kind: Option<String>,
    pub spotlight: SpotlightMetadata,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct SpotlightMetadata {
    pub authors: Vec<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub keywords: Vec<String>,
    pub content_type: Option<String>,
    pub page_count: Option<u32>,
    pub pixel_width: Option<u32>,
    pub pixel_height: Option<u32>,
    pub duration_seconds: Option<f64>,
}

pub fn extract(path: &Path) -> Result<FileMetadata> {
    let meta = std::fs::metadata(path)?;

    let path_str = path.to_string_lossy().to_string();
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase());

    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let kind = ext.as_ref().map(|e| infer_kind(e));

    let spotlight = extract_spotlight(path).unwrap_or_default();

    Ok(FileMetadata {
        path: path_str,
        name,
        ext,
        size: meta.len(),
        mtime,
        kind,
        spotlight,
    })
}

fn infer_kind(ext: &str) -> String {
    match ext {
        "pdf" => "pdf".into(),
        "md" | "markdown" => "markdown".into(),
        "txt" | "log" => "text".into(),
        "doc" | "docx" | "rtf" => "document".into(),
        "xls" | "xlsx" | "csv" => "spreadsheet".into(),
        "ppt" | "pptx" => "presentation".into(),
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "heic" | "tiff" => "image".into(),
        "mp4" | "mov" | "avi" | "mkv" => "video".into(),
        "mp3" | "wav" | "flac" | "m4a" => "audio".into(),
        "zip" | "tar" | "gz" | "rar" | "7z" => "archive".into(),
        "js" | "ts" | "tsx" | "jsx" | "vue" | "py" | "rs" | "go" | "java" | "cpp" | "c" => {
            "code".into()
        }
        "html" | "htm" => "html".into(),
        "json" | "yaml" | "yml" | "toml" | "xml" => "data".into(),
        "eml" | "msg" => "email".into(),
        _ => "other".into(),
    }
}

fn extract_spotlight(path: &Path) -> Result<SpotlightMetadata> {
    let output = Command::new("mdls")
        .arg("-raw")
        .arg(path)
        .output()?;

    if !output.status.success() {
        return Ok(SpotlightMetadata::default());
    }

    let text = String::from_utf8_lossy(&output.stdout);
    Ok(parse_mdls(&text))
}

fn parse_mdls(text: &str) -> SpotlightMetadata {
    let mut meta = SpotlightMetadata::default();

    for line in text.lines() {
        let line = line.trim();
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim().trim_matches('"');

        match key {
            "kMDItemAuthors" => {
                meta.authors = parse_array(value);
            }
            "kMDItemTitle" if !value.is_empty() => meta.title = Some(value.to_string()),
            "kMDItemDescription" if !value.is_empty() => {
                meta.description = Some(value.to_string())
            }
            "kMDItemKeywords" => meta.keywords = parse_array(value),
            "kMDItemContentType" if !value.is_empty() => {
                meta.content_type = Some(value.to_string())
            }
            "kMDItemNumberOfPages" => meta.page_count = value.parse().ok(),
            "kMDItemPixelWidth" => meta.pixel_width = value.parse().ok(),
            "kMDItemPixelHeight" => meta.pixel_height = value.parse().ok(),
            "kMDItemDurationSeconds" => meta.duration_seconds = value.parse().ok(),
            _ => {}
        }
    }

    meta
}

fn parse_array(value: &str) -> Vec<String> {
    let trimmed = value
        .trim_start_matches('(')
        .trim_end_matches(')')
        .trim();

    if trimmed.is_empty() || trimmed == "(null)" {
        return Vec::new();
    }

    trimmed
        .split(',')
        .map(|s| s.trim().trim_matches('"').to_string())
        .filter(|s| !s.is_empty() && s != "(null)")
        .collect()
}
