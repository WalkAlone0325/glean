use std::path::PathBuf;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager,
};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod agent;
mod commands;
mod db;
mod embedding;
mod llm;
mod rag;
mod scanner;
mod search;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_tracing();

    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle();
            init_tray(handle)?;
            init_db(handle)?;
            init_global_shortcut(handle)?;
            init_watcher(handle)?;
            init_embedding_worker(handle)?;
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::greet,
            commands::get_stats,
            commands::get_indexed_roots,
            commands::list_files,
            commands::read_text_preview,
            commands::start_indexing,
            commands::cancel_indexing,
            commands::pause_indexing,
            commands::resume_indexing,
            commands::is_indexing,
            commands::is_paused,
            commands::consistency_check,
            commands::search_files,
            commands::hybrid_search_files,
            commands::open_file,
            commands::reveal_in_finder,
            commands::track_file_view,
            commands::list_recent_files,
            commands::toggle_favorite,
            commands::list_favorite_files,
            commands::get_setting,
            commands::set_setting,
            commands::get_provider_config,
            commands::test_llm,
            commands::chat_send,
            commands::chat_stop,
            commands::tool_confirm,
            commands::list_pending_operations,
            commands::undo_operation,
            commands::list_messages,
            commands::list_conversations,
            commands::delete_conversation,
            commands::rename_conversation,
            commands::list_tags,
            commands::create_tag,
            commands::delete_tag,
            commands::get_file_tags,
            commands::set_file_tags,
            commands::get_ignore_rules,
            commands::set_ignore_rules,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn init_tracing() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,glean=debug"));

    let console_layer = fmt::layer().with_target(false).compact();

    let file_layer = match dirs::data_dir() {
        Some(dir) => {
            let log_dir = dir.join("Glean").join("logs");
            if std::fs::create_dir_all(&log_dir).is_err() {
                None
            } else {
                let appender = tracing_appender::rolling::daily(&log_dir, "glean.log");
                Some(fmt::layer().with_ansi(false).with_writer(appender))
            }
        }
        None => None,
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(console_layer)
        .with(file_layer)
        .init();
}

fn init_tray(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let quit = MenuItem::with_id(app, "quit", "退出 Glean", true, None::<&str>)?;
    let show = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    let icon: tauri::image::Image = match tauri::image::Image::from_path("icons/tray-icon.png") {
        Ok(img) => img,
        Err(_) => app
            .default_window_icon()
            .cloned()
            .ok_or::<Box<dyn std::error::Error>>("missing default window icon".into())?,
    };

    TrayIconBuilder::new()
        .icon(icon)
        .icon_as_template(true)
        .menu(&menu)
        .tooltip("Glean")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "quit" => app.exit(0),
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            _ => {}
        })
        .build(app)?;
    Ok(())
}

fn init_db(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&dir)?;
    let db_path = dir.join("glean.sqlite");
    let database = db::Database::open(&db_path)?;
    tracing::info!("database initialized at {}", db_path.display());
    let db_arc = std::sync::Arc::new(tokio::sync::Mutex::new(database));
    let scheduler = std::sync::Arc::new(scanner::Scheduler::new(db_arc.clone()));
    let confirmations = std::sync::Arc::new(agent::ConfirmationRegistry::new());
    app.manage(db_arc);
    app.manage(scheduler);
    app.manage(confirmations);

    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let db = app_clone
            .state::<std::sync::Arc<tokio::sync::Mutex<db::Database>>>()
            .inner()
            .clone();

        let rows: Vec<(i64, String)> = {
            let db_lock = db.lock().await;
            let conn = match db_lock.conn.lock() {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!("consistency check db lock failed: {}", e);
                    return;
                }
            };
            let mut stmt = match conn.prepare("SELECT id, path FROM files WHERE deleted_at IS NULL") {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("consistency prepare failed: {}", e);
                    return;
                }
            };
            let rows: Vec<(i64, String)> = match stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?))) {
                Ok(it) => it.filter_map(|r| r.ok()).collect(),
                Err(e) => {
                    tracing::warn!("consistency query failed: {}", e);
                    return;
                }
            };
            rows
        };

        if rows.is_empty() {
            return;
        }

        let missing: Vec<i64> = rows
            .iter()
            .filter(|(_, p)| !std::path::Path::new(p).exists())
            .map(|(id, _)| *id)
            .collect();

        if missing.is_empty() {
            tracing::info!("consistency check: all {} files present", rows.len());
            return;
        }

        let now = chrono::Utc::now().timestamp();
        {
            let db_lock = db.lock().await;
            let conn_guard = db_lock.conn.lock();
            if let Ok(conn) = conn_guard {
                let res: rusqlite::Result<()> = (|| {
                    let tx = conn.unchecked_transaction()?;
                    for id in &missing {
                        tx.execute(
                            "UPDATE files SET deleted_at = ?1 WHERE id = ?2",
                            rusqlite::params![now, id],
                        )?;
                    }
                    tx.commit()?;
                    Ok(())
                })();
                if let Err(e) = res {
                    tracing::warn!("consistency mark deleted failed: {}", e);
                    return;
                }
            }
        }

        tracing::info!(
            "consistency check: {} checked, {} marked deleted",
            rows.len(),
            missing.len()
        );
        let _ = app_clone.emit("consistency-report", missing.len());
    });

    Ok(())
}

fn init_watcher(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let scheduler = app.state::<std::sync::Arc<scanner::Scheduler>>().inner().clone();
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
    let defaults = vec![
        home.join("Desktop"),
        home.join("Documents"),
        home.join("Downloads"),
        home.join("Pictures").join("Screenshots"),
    ];
    let roots = defaults
        .into_iter()
        .filter(|p| p.exists())
        .collect::<Vec<_>>();
    if roots.is_empty() {
        tracing::warn!("no default folders to watch");
        return Ok(());
    }

    let needs_initial_scan = {
        let db = app
            .state::<std::sync::Arc<tokio::sync::Mutex<db::Database>>>()
            .inner()
            .clone();
        let db = db.blocking_lock();
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM files", [], |r| r.get(0))
            .unwrap_or(0);
        count == 0
    };

    if needs_initial_scan {
        let scheduler_clone = scheduler.clone();
        let app_clone = app.clone();
        let scan_roots = roots.clone();
        tracing::info!("starting initial scan of {} folders", scan_roots.len());
        tauri::async_runtime::spawn(async move {
            if let Err(e) = scheduler_clone.index_paths(app_clone, scan_roots).await {
                tracing::error!("initial scan failed: {}", e);
            }
        });
    } else {
        tracing::info!("database has files, skipping initial scan");
    }

    let mut watcher = scanner::FsWatcher::new(scheduler);
    let app_handle = app.clone();
    std::thread::spawn(move || {
        let runtime = match tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                tracing::error!("create watcher runtime failed: {}", e);
                return;
            }
        };
        runtime.block_on(async move {
            if let Err(e) = watcher.watch(app_handle, roots) {
                tracing::error!("watcher init failed: {}", e);
            }
            std::mem::forget(watcher);
        });
    });
    Ok(())
}

fn init_global_shortcut(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};

    let shortcut = Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::Space);
    app.global_shortcut()
        .on_shortcut(shortcut, move |app, _shortcut, _event| {
            if let Some(window) = app.get_webview_window("main") {
                if window.is_visible().unwrap_or(false) {
                    let _ = window.hide();
                } else {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })?;
    tracing::info!("global shortcut registered: Cmd+Shift+Space");
    Ok(())
}

fn init_embedding_worker(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let db = app
        .state::<std::sync::Arc<tokio::sync::Mutex<db::Database>>>()
        .inner()
        .clone();
    let worker = embedding::worker::spawn_worker(db, app.clone());
    app.manage(worker);
    tracing::info!("embedding worker spawned");
    Ok(())
}
