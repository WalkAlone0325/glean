pub mod hybrid;
pub mod vector;

use anyhow::Result;
use jieba_rs::Jieba;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchFilter {
    pub ext: Option<String>,
    pub kind: Option<String>,
    pub path_contains: Option<String>,
    pub since: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub id: i64,
    pub path: String,
    pub name: String,
    pub ext: Option<String>,
    pub size: i64,
    pub mtime: i64,
    pub kind: Option<String>,
    pub snippet: Option<String>,
    pub rank: f64,
}

pub struct Searcher {
    jieba: &'static Jieba,
}

impl Searcher {
    pub fn new() -> Self {
        static JIEBA: OnceLock<Jieba> = OnceLock::new();
        let jieba = JIEBA.get_or_init(Jieba::default);
        Self { jieba }
    }

    pub fn search(
        &self,
        conn: &Mutex<Connection>,
        query: &str,
        filter: SearchFilter,
        limit: i64,
    ) -> Result<Vec<SearchResult>> {
        let words = self.tokenize(query);
        if words.is_empty() {
            return Ok(Vec::new());
        }

        let fts_query = words
            .iter()
            .map(|w| escape_fts(w))
            .filter(|w| !w.is_empty())
            .collect::<Vec<_>>()
            .join(" ");

        if fts_query.is_empty() {
            return Ok(Vec::new());
        }

        let conn = conn.lock().map_err(|e| anyhow::anyhow!("db mutex: {}", e))?;

        let mut sql = String::from(
            "SELECT f.id, f.path, f.name, f.ext, f.size, f.mtime, f.kind,
                    snippet(files_fts, 1, '[', '] ', '...', 16) AS snippet,
                    bm25(files_fts) AS rank
             FROM files_fts
             JOIN files f ON f.id = files_fts.rowid
             WHERE files_fts MATCH ?1
               AND f.deleted_at IS NULL",
        );

        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> =
            vec![Box::new(fts_query.clone())];

        let mut idx = 2;
        if let Some(ext) = &filter.ext {
            sql.push_str(&format!(" AND (f.ext = ?{} OR f.name LIKE ?{})", idx, idx + 1));
            params_vec.push(Box::new(ext.clone()));
            params_vec.push(Box::new(format!("%.{}", ext)));
            idx += 2;
        }
        if let Some(kind) = &filter.kind {
            sql.push_str(&format!(" AND f.kind = ?{}", idx));
            params_vec.push(Box::new(kind.clone()));
            idx += 1;
        }
        if let Some(path_contains) = &filter.path_contains {
            sql.push_str(&format!(" AND f.path LIKE ?{}", idx));
            params_vec.push(Box::new(format!("%{}%", path_contains)));
            idx += 1;
        }
        if let Some(since) = filter.since {
            sql.push_str(&format!(" AND f.mtime >= ?{}", idx));
            params_vec.push(Box::new(since));
        }

        sql.push_str(&format!(" ORDER BY rank LIMIT {}", limit));

        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();

        let rows = stmt.query_map(param_refs.as_slice(), |row: &rusqlite::Row<'_>| {
            Ok(SearchResult {
                id: row.get(0)?,
                path: row.get(1)?,
                name: row.get(2)?,
                ext: row.get(3)?,
                size: row.get(4)?,
                mtime: row.get(5)?,
                kind: row.get(6)?,
                snippet: row.get(7)?,
                rank: row.get(8)?,
            })
        })?;

        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }

    fn tokenize(&self, query: &str) -> Vec<String> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Vec::new();
        }

        let mut words: Vec<String> = self
            .jieba
            .cut(trimmed, true)
            .into_iter()
            .map(|s| s.to_string())
            .filter(|s| !s.trim().is_empty())
            .collect();

        if words.is_empty() || words.iter().any(|w| w.chars().count() > 1) {
            return words;
        }

        let chars: Vec<String> = trimmed
            .chars()
            .filter(|c| !c.is_whitespace())
            .map(|c| c.to_string())
            .collect();
        if !chars.is_empty() {
            words.extend(chars);
        }
        words
    }
}

fn escape_fts(word: &str) -> String {
    let needs_quote = word
        .chars()
        .any(|c| matches!(c, '"' | '*' | '?' | '(' | ')' | ' ' | '\t' | ':'));
    let sanitized = word.replace('"', "");
    if needs_quote {
        format!("\"{}\"", sanitized)
    } else {
        sanitized
    }
}
