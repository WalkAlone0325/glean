use anyhow::Result;
use std::io::{BufReader, Read};
use std::path::Path;
use std::process::Command;

const MAX_TEXT_BYTES: usize = 512 * 1024;

pub async fn extract_text(path: &Path, ext: &Option<String>) -> Option<String> {
    let ext = ext.clone()?;
    let ext = ext.to_lowercase();
    let raw = match ext.as_str() {
        "txt" | "log" | "md" | "markdown" | "rst" | "csv" | "tsv" | "json" | "yaml" | "yml"
        | "toml" | "xml" | "html" | "htm" | "js" | "ts" | "tsx" | "jsx" | "vue" | "py" | "rs"
        | "go" | "java" | "cpp" | "c" | "h" | "css" | "scss" | "sh" | "bash" | "zsh" | "fish"
        | "sql" | "env" | "ini" | "conf" => read_truncated(path).ok(),
        "rtf" => read_truncated(path).ok(),
        "pdf" => extract_via_mdimport(path),
        _ => None,
    }?;
    Some(truncate(&raw, MAX_TEXT_BYTES))
}

fn read_truncated(path: &Path) -> Result<String> {
    let file = std::fs::File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut buf = Vec::with_capacity(64 * 1024);
    reader.read_to_end(&mut buf)?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

fn extract_via_mdimport(path: &Path) -> Option<String> {
    let output = Command::new("mdimport").arg("-d2").arg(path).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stderr);
    Some(text.to_string())
}

fn truncate(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    s[..end].to_string()
}
