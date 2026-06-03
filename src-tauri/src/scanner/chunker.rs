use anyhow::Result;
use std::path::Path;

const TARGET_CHARS: usize = 800;
const OVERLAP_CHARS: usize = 120;
const MAX_CHUNKS_PER_FILE: usize = 64;

#[derive(Debug, Clone)]
pub struct Chunk {
    pub content: String,
    pub page: Option<i32>,
    pub position: i32,
    pub token_count: Option<i32>,
}

pub fn chunk_text(text: &str, kind: Option<&str>) -> Vec<Chunk> {
    if text.trim().is_empty() {
        return Vec::new();
    }
    match kind {
        Some("markdown") => chunk_by_heading(text),
        Some("code") => chunk_by_lines(text, TARGET_CHARS * 2, OVERLAP_CHARS * 2),
        _ => chunk_paragraphs(text),
    }
}

pub fn chunk_file(path: &Path, kind: Option<&str>, text: &str) -> Result<Vec<Chunk>> {
    let mut chunks = chunk_text(text, kind);
    if chunks.len() > MAX_CHUNKS_PER_FILE {
        chunks.truncate(MAX_CHUNKS_PER_FILE);
    }
    let page = extract_page_from_path(path);
    for (i, c) in chunks.iter_mut().enumerate() {
        if c.page.is_none() {
            c.page = page;
        }
        c.position = i as i32;
        c.token_count = Some(estimate_tokens(&c.content));
    }
    Ok(chunks)
}

fn chunk_paragraphs(text: &str) -> Vec<Chunk> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut pos = 0i32;

    for para in text.split("\n\n") {
        let para = para.trim();
        if para.is_empty() {
            continue;
        }
        if buf.len() + para.len() + 2 > TARGET_CHARS && !buf.is_empty() {
            out.push(Chunk {
                content: std::mem::take(&mut buf),
                page: None,
                position: pos,
                token_count: None,
            });
            pos += 1;
        }
        if !buf.is_empty() {
            buf.push_str("\n\n");
        }
        buf.push_str(para);
    }

    if !buf.is_empty() {
        out.push(Chunk {
            content: buf,
            page: None,
            position: pos,
            token_count: None,
        });
    }
    out
}

fn chunk_by_heading(text: &str) -> Vec<Chunk> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut pos = 0i32;

    for line in text.lines() {
        let is_heading = line.starts_with('#');
        if is_heading && !current.is_empty() && current.len() > TARGET_CHARS / 2 {
            out.push(Chunk {
                content: std::mem::take(&mut current),
                page: None,
                position: pos,
                token_count: None,
            });
            pos += 1;
        }
        if !current.is_empty() {
            current.push('\n');
        }
        current.push_str(line);
    }
    if !current.is_empty() {
        out.push(Chunk {
            content: current,
            page: None,
            position: pos,
            token_count: None,
        });
    }

    if out.iter().any(|c| c.content.len() > TARGET_CHARS * 2) {
        out = out
            .into_iter()
            .flat_map(|c| chunk_paragraphs(&c.content))
            .collect();
        for (i, c) in out.iter_mut().enumerate() {
            c.position = i as i32;
        }
    }
    out
}

fn chunk_by_lines(text: &str, target: usize, overlap: usize) -> Vec<Chunk> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut pos = 0i32;

    for line in text.lines() {
        if buf.len() + line.len() + 1 > target && !buf.is_empty() {
            out.push(Chunk {
                content: std::mem::take(&mut buf),
                page: None,
                position: pos,
                token_count: None,
            });
            pos += 1;
            if let Some(tail) = out.last().map(|c: &Chunk| c.content.chars().rev().take(overlap).collect::<String>().chars().rev().collect::<String>()) {
                buf = tail;
            }
        }
        if !buf.is_empty() {
            buf.push('\n');
        }
        buf.push_str(line);
    }
    if !buf.is_empty() {
        out.push(Chunk {
            content: buf,
            page: None,
            position: pos,
            token_count: None,
        });
    }
    out
}

fn extract_page_from_path(_path: &Path) -> Option<i32> {
    None
}

fn estimate_tokens(text: &str) -> i32 {
    let chars = text.chars().count() as f32;
    let cjk = text.chars().filter(|c| ('\u{4E00}'..='\u{9FFF}').contains(c)).count() as f32;
    ((chars - cjk) / 4.0 + cjk / 1.5).ceil() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunks_short_text_into_one() {
        let out = chunk_text("hello world", None);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].content, "hello world");
    }

    #[test]
    fn chunks_by_paragraph() {
        let text = "para1 line1\n\npara2 line2\n\npara3";
        let out = chunk_text(text, None);
        assert!(out.len() >= 1);
    }

    #[test]
    fn markdown_split_by_heading() {
        let text = "# Title 1\nbody body body\n# Title 2\nmore body";
        let out = chunk_text(text, Some("markdown"));
        assert!(out.len() >= 1);
    }
}
