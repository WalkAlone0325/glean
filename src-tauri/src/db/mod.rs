pub mod migrations;

use anyhow::Result;
use rusqlite::Connection;
use std::sync::Mutex;

pub struct Database {
    #[allow(dead_code)]
    pub conn: Mutex<Connection>,
}

impl Database {
    pub fn open(path: &std::path::Path) -> Result<Self> {
        let mut conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;
             PRAGMA temp_store = MEMORY;",
        )?;
        migrations::migrations().to_latest(&mut conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }
}
