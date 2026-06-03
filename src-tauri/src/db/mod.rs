pub mod migrations;

use anyhow::Result;
use rusqlite::{ffi::sqlite3_auto_extension, Connection};
use std::sync::Mutex;

pub struct Database {
    #[allow(dead_code)]
    pub conn: Mutex<Connection>,
}

unsafe fn register_sqlite_vec() {
    sqlite3_auto_extension(Some(std::mem::transmute(
        sqlite_vec::sqlite3_vec_init as *const (),
    )));
}

impl Database {
    pub fn open(path: &std::path::Path) -> Result<Self> {
        unsafe { register_sqlite_vec() };
        let mut conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;
             PRAGMA temp_store = MEMORY;",
        )?;
        let vec_version: String = conn
            .query_row("SELECT vec_version()", [], |r| r.get(0))
            .unwrap_or_else(|_| "unknown".to_string());
        tracing::info!("sqlite-vec {} loaded", vec_version);
        migrations::migrations().to_latest(&mut conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }
}
