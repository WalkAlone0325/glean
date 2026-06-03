use std::collections::HashSet;
use std::path::Path;

pub struct IgnoreRules {
    builtin: HashSet<String>,
    user: HashSet<String>,
    max_size: u64,
}

impl Default for IgnoreRules {
    fn default() -> Self {
        let builtin: HashSet<String> = [
            ".git",
            "node_modules",
            ".Trash",
            ".DS_Store",
            ".npm",
            ".pnpm-store",
            ".cache",
            "target",
            "dist",
            "build",
            ".next",
            ".nuxt",
            ".svelte-kit",
            ".turbo",
            ".vercel",
            ".parcel-cache",
            "Coverage",
            "__pycache__",
            ".pytest_cache",
            ".mypy_cache",
            ".venv",
            "venv",
            ".idea",
            ".vscode",
            ".history",
            "Library",
            ".Trash-0",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();

        Self {
            builtin,
            user: HashSet::new(),
            max_size: 100 * 1024 * 1024,
        }
    }
}

impl IgnoreRules {
    #[allow(dead_code)]
    pub fn with_user_rules(mut self, rules: Vec<String>) -> Self {
        self.user = rules.into_iter().collect();
        self
    }

    #[allow(dead_code)]
    pub fn with_max_size(mut self, size: u64) -> Self {
        self.max_size = size;
        self
    }

    pub fn is_ignored(&self, path: &Path) -> bool {
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();

        if file_name.starts_with('.')
            && matches!(file_name, ".DS_Store" | ".localized" | ".icloud")
        {
            return true;
        }

        for component in path.components() {
            if let std::path::Component::Normal(name) = component {
                if let Some(name_str) = name.to_str() {
                    if self.builtin.contains(name_str) || self.user.contains(name_str) {
                        return true;
                    }
                }
            }
        }

        if matches!(
            ext.as_str(),
            "pyc" | "pyo" | "class" | "o" | "obj" | "dll" | "so" | "dylib"
        ) {
            return true;
        }

        false
    }

    pub fn is_too_large(&self, size: u64) -> bool {
        size > self.max_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_builtin_paths() {
        let rules = IgnoreRules::default();
        assert!(rules.is_ignored(Path::new("/x/.git/HEAD")));
        assert!(rules.is_ignored(Path::new("/x/node_modules/foo")));
        assert!(rules.is_ignored(Path::new("/x/.DS_Store")));
        assert!(rules.is_ignored(Path::new("/x/.icloud")));
    }

    #[test]
    fn ignores_by_extension() {
        let rules = IgnoreRules::default();
        assert!(rules.is_ignored(Path::new("/x/foo.pyc")));
        assert!(rules.is_ignored(Path::new("/x/lib.so")));
    }

    #[test]
    fn allows_normal_files() {
        let rules = IgnoreRules::default();
        assert!(!rules.is_ignored(Path::new("/x/document.pdf")));
        assert!(!rules.is_ignored(Path::new("/x/photo.jpg")));
    }

    #[test]
    fn respects_size_limit() {
        let rules = IgnoreRules::default();
        assert!(rules.is_too_large(200 * 1024 * 1024));
        assert!(!rules.is_too_large(50 * 1024 * 1024));
    }
}
