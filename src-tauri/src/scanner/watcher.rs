use crate::scanner::Scheduler;
use notify::{
    Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use tracing::{error, info, warn};

const DEBOUNCE_MS: u64 = 400;
const BATCH_COALESCE_MS: u64 = 250;

pub struct FsWatcher {
    watcher: Option<RecommendedWatcher>,
    scheduler: Arc<Scheduler>,
}

impl FsWatcher {
    pub fn new(scheduler: Arc<Scheduler>) -> Self {
        Self {
            watcher: None,
            scheduler,
        }
    }

    pub fn watch(&mut self, app: AppHandle, roots: Vec<PathBuf>) -> anyhow::Result<()> {
        let scheduler = self.scheduler.clone();
        let (tx, mut rx) = mpsc::channel::<notify::Result<notify::Event>>(256);

        let mut watcher: RecommendedWatcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.blocking_send(res);
            },
            Config::default().with_poll_interval(Duration::from_millis(500)),
        )?;

        for root in &roots {
            if root.exists() {
                if let Err(e) = watcher.watch(root.as_path(), RecursiveMode::Recursive) {
                    warn!("watch {} failed: {}", root.display(), e);
                } else {
                    info!("watching {}", root.display());
                }
            }
        }

        let app_handle = app.clone();
        let sched = scheduler.clone();
        tokio::spawn(async move {
            let mut pending: HashMap<PathBuf, Instant> = HashMap::new();
            loop {
                tokio::select! {
                    Some(event) = rx.recv() => {
                        match event {
                            Ok(ev) => handle_event(&sched, &app_handle, &ev, &mut pending).await,
                            Err(e) => error!("watch error: {}", e),
                        }
                    }
                    _ = tokio::time::sleep(Duration::from_millis(BATCH_COALESCE_MS)) => {
                        flush_pending(&sched, &mut pending).await;
                    }
                }
            }
        });

        self.watcher = Some(watcher);
        Ok(())
    }
}

async fn handle_event(
    scheduler: &Arc<Scheduler>,
    app: &AppHandle,
    event: &notify::Event,
    pending: &mut HashMap<PathBuf, Instant>,
) {
    let kind = event.kind;
    let paths: Vec<PathBuf> = event
        .paths
        .iter()
        .filter(|p| !is_ignored_rough(p))
        .cloned()
        .collect();
    if paths.is_empty() {
        return;
    }

    match kind {
        EventKind::Create(_) | EventKind::Modify(_) => {
            for path in &paths {
                if path.is_file() {
                    pending.insert(path.clone(), Instant::now());
                }
            }
        }
        EventKind::Remove(_) => {
            for path in &paths {
                let path_str = path.to_string_lossy().to_string();
                if let Err(e) = scheduler.mark_deleted(&path_str).await {
                    warn!("mark deleted {} failed: {}", path_str, e);
                }
                let _ = app.emit("fs-removed", &path_str);
            }
        }
        _ => {}
    }
}

async fn flush_pending(scheduler: &Arc<Scheduler>, pending: &mut HashMap<PathBuf, Instant>) {
    if pending.is_empty() {
        return;
    }
    let now = Instant::now();
    let due: Vec<PathBuf> = pending
        .iter()
        .filter(|(_, t)| now.duration_since(**t) >= Duration::from_millis(DEBOUNCE_MS))
        .map(|(p, _)| p.clone())
        .collect();
    for path in due {
        pending.remove(&path);
        let sched = scheduler.clone();
        tokio::spawn(async move {
            if let Err(e) = sched.index_single(path.clone()).await {
                warn!("reindex {} failed: {}", path.display(), e);
            }
        });
    }
}

fn is_ignored_rough(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    matches!(
        name,
        ".DS_Store" | ".localized" | ".icloud" | ".tmp" | ".swp"
    ) || name.starts_with(".git")
}
