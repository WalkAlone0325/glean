use super::vector::{vector_search, VectorHit};
use super::{SearchFilter, SearchResult, Searcher};
use rusqlite::Connection;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;

const RRF_K: f64 = 60.0;

#[derive(Debug, Clone, Serialize)]
pub struct HybridResult {
    #[serde(flatten)]
    pub base: SearchResult,
    pub source: HitSource,
    pub vector_score: f64,
    pub fts_rank: f64,
}

#[derive(Debug, Clone, Serialize)]
pub enum HitSource {
    Both,
    VectorOnly,
    FtsOnly,
}

pub fn hybrid_search(
    conn: &Mutex<Connection>,
    query: &str,
    filter: SearchFilter,
    limit: i64,
) -> anyhow::Result<Vec<HybridResult>> {
    let over_fetch = (limit * 3).max(20);

    let fts = Searcher::new().search(conn, query, filter.clone(), over_fetch)?;
    let vec_hits = match vector_search(conn, query, &filter, over_fetch) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("vector search failed, falling back to FTS only: {}", e);
            Vec::new()
        }
    };

    let mut fts_rank_by_id: HashMap<i64, (f64, usize)> = HashMap::new();
    for (rank_idx, hit) in fts.iter().enumerate() {
        fts_rank_by_id.insert(hit.id, (hit.rank, rank_idx));
    }

    let mut vec_rank_by_id: HashMap<i64, (VectorHit, usize)> = HashMap::new();
    for (rank_idx, hit) in vec_hits.iter().enumerate() {
        vec_rank_by_id.insert(hit.file_id, (hit.clone(), rank_idx));
    }

    let mut all_file_ids: std::collections::HashSet<i64> = std::collections::HashSet::new();
    all_file_ids.extend(fts_rank_by_id.keys().copied());
    all_file_ids.extend(vec_rank_by_id.keys().copied());

    let mut merged: Vec<HybridResult> = all_file_ids
        .iter()
        .map(|&file_id| {
            let fts_entry = fts_rank_by_id.get(&file_id);
            let vec_entry = vec_rank_by_id.get(&file_id);

            let fts_rank = fts_entry.map(|(r, _)| *r).unwrap_or(0.0);
            let fts_idx = fts_entry.map(|(_, i)| *i);
            let vec_idx = vec_entry.map(|(_, i)| *i);
            let vec_hit = vec_entry.map(|(h, _)| h.clone());

            let fts_score = match fts_idx {
                Some(i) => 1.0 / (RRF_K + (i + 1) as f64),
                None => 0.0,
            };
            let vec_score = match vec_idx {
                Some(i) => 1.0 / (RRF_K + (i + 1) as f64),
                None => 0.0,
            };
            let combined = fts_score + vec_score;

            let source = match (fts_entry.is_some(), vec_entry.is_some()) {
                (true, true) => HitSource::Both,
                (false, true) => HitSource::VectorOnly,
                _ => HitSource::FtsOnly,
            };

            let base = if let Some(fts_hit) = fts.iter().find(|h| h.id == file_id) {
                let mut b = fts_hit.clone();
                if let Some(v) = &vec_hit {
                    if b.snippet.is_none() {
                        b.snippet = v.snippet.clone();
                    }
                }
                b
            } else if let Some(v) = &vec_hit {
                SearchResult {
                    id: v.file_id,
                    path: v.path.clone(),
                    name: v.name.clone(),
                    ext: v.ext.clone(),
                    size: 0,
                    mtime: 0,
                    kind: v.kind.clone(),
                    snippet: v.snippet.clone(),
                    rank: v.score as f64,
                }
            } else {
                unreachable!()
            };

            HybridResult {
                base,
                source,
                vector_score: vec_score,
                fts_rank,
            }
            .with_combined_rank(combined)
        })
        .collect();

    merged.sort_by(|a, b| {
        b.base.rank.partial_cmp(&a.base.rank).unwrap_or(std::cmp::Ordering::Equal)
    });
    merged.truncate(limit as usize);
    Ok(merged)
}

trait WithCombined {
    fn with_combined_rank(self, combined: f64) -> Self;
}

impl WithCombined for HybridResult {
    fn with_combined_rank(mut self, combined: f64) -> Self {
        self.base.rank = combined;
        self
    }
}
