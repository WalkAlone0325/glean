use super::ignore::IgnoreRules;
use super::metadata::{self, FileMetadata};
use anyhow::Result;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

#[allow(dead_code)]
pub struct ScanResult {
    pub discovered: u64,
    pub indexed: u64,
    pub skipped: u64,
    pub errors: u64,
    pub duration_ms: u128,
}

pub struct Walker {
    ignore: IgnoreRules,
}

impl Walker {
    pub fn new(ignore: IgnoreRules) -> Self {
        Self { ignore }
    }

    pub fn collect(&self, roots: &[PathBuf]) -> Vec<PathBuf> {
        let mut out = Vec::new();
        for root in roots {
            if !root.exists() {
                continue;
            }
            let depth = std::cmp::max(1, 3);
            let it = WalkDir::new(root)
                .max_depth(depth)
                .follow_links(false)
                .into_iter()
                .filter_entry(|e| !self.is_ignored_entry(e));

            for entry in it {
                let Ok(entry) = entry else { continue };
                if entry.file_type().is_file()
                    && self.should_include(entry.metadata().ok())
                {
                    out.push(entry.path().to_path_buf());
                }
            }
        }
        out
    }

    pub fn extract(&self, path: &Path) -> Result<FileMetadata> {
        metadata::extract(path)
    }

    pub fn is_ignored_path(&self, path: &Path) -> bool {
        if !path.exists() {
            return true;
        }
        let meta = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(_) => return true,
        };
        if !meta.file_type().is_file() {
            return true;
        }
        if self.ignore.is_too_large(meta.len()) || self.ignore.is_ignored(path) {
            return true;
        }
        false
    }

    fn is_ignored_entry(&self, entry: &DirEntry) -> bool {
        entry
            .file_name()
            .to_str()
            .is_some_and(|name| self.ignore.is_ignored(Path::new(name)))
    }

    fn should_include(&self, meta: Option<std::fs::Metadata>) -> bool {
        let Some(meta) = meta else {
            return false;
        };
        !self.ignore.is_too_large(meta.len())
    }
}
