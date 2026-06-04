use rusqlite_migration::{Migrations, M};

pub fn migrations() -> Migrations<'static> {
    Migrations::new(vec![
        M::up(
            r#"
            CREATE TABLE IF NOT EXISTS files (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                path        TEXT NOT NULL UNIQUE,
                name        TEXT NOT NULL,
                ext         TEXT,
                size        INTEGER NOT NULL,
                mtime       INTEGER NOT NULL,
                hash        TEXT,
                kind        TEXT,
                indexed_at  INTEGER NOT NULL DEFAULT (strftime('%s','now')),
                deleted_at  INTEGER
            );
            CREATE INDEX idx_files_path ON files(path);
            CREATE INDEX idx_files_name ON files(name);
            CREATE INDEX idx_files_ext ON files(ext);
            CREATE INDEX idx_files_mtime ON files(mtime DESC);

            CREATE TABLE IF NOT EXISTS chunks (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                file_id     INTEGER NOT NULL,
                content     TEXT NOT NULL,
                page        INTEGER,
                position    INTEGER NOT NULL,
                embedding_id TEXT,
                created_at  INTEGER NOT NULL DEFAULT (strftime('%s','now')),
                FOREIGN KEY (file_id) REFERENCES files(id) ON DELETE CASCADE
            );
            CREATE INDEX idx_chunks_file ON chunks(file_id);

            CREATE TABLE IF NOT EXISTS tags (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                name        TEXT NOT NULL UNIQUE,
                color       TEXT,
                created_at  INTEGER NOT NULL DEFAULT (strftime('%s','now'))
            );

            CREATE TABLE IF NOT EXISTS file_tags (
                file_id     INTEGER NOT NULL,
                tag_id      INTEGER NOT NULL,
                PRIMARY KEY (file_id, tag_id),
                FOREIGN KEY (file_id) REFERENCES files(id) ON DELETE CASCADE,
                FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS notes (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                file_id     INTEGER,
                content     TEXT NOT NULL,
                created_at  INTEGER NOT NULL DEFAULT (strftime('%s','now')),
                FOREIGN KEY (file_id) REFERENCES files(id) ON DELETE SET NULL
            );

            CREATE TABLE IF NOT EXISTS conversations (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                title       TEXT,
                created_at  INTEGER NOT NULL DEFAULT (strftime('%s','now')),
                updated_at  INTEGER NOT NULL DEFAULT (strftime('%s','now'))
            );

            CREATE TABLE IF NOT EXISTS messages (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                conversation_id INTEGER NOT NULL,
                role            TEXT NOT NULL CHECK (role IN ('user','assistant','system','tool')),
                content         TEXT NOT NULL,
                tokens          INTEGER,
                created_at      INTEGER NOT NULL DEFAULT (strftime('%s','now')),
                FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
            );
            CREATE INDEX idx_messages_conv ON messages(conversation_id);

            CREATE TABLE IF NOT EXISTS llm_usage (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                provider    TEXT NOT NULL,
                model       TEXT NOT NULL,
                input_tokens  INTEGER,
                output_tokens INTEGER,
                cost        REAL,
                created_at  INTEGER NOT NULL DEFAULT (strftime('%s','now'))
            );

            CREATE TABLE IF NOT EXISTS settings (
                key         TEXT PRIMARY KEY,
                value       TEXT NOT NULL,
                updated_at  INTEGER NOT NULL DEFAULT (strftime('%s','now'))
            );

            CREATE TABLE IF NOT EXISTS operations (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                kind        TEXT NOT NULL,
                source_path TEXT,
                target_path TEXT,
                payload     TEXT,
                undone_at   INTEGER,
                created_at  INTEGER NOT NULL DEFAULT (strftime('%s','now'))
            );
            "#,
        ),
        M::up(
            r#"
            CREATE VIRTUAL TABLE IF NOT EXISTS files_fts USING fts5(
                name,
                content,
                tokenize = 'unicode61 remove_diacritics 2'
            );
            "#,
        ),
        M::up(
            r#"
            ALTER TABLE chunks ADD COLUMN embedding_status TEXT NOT NULL DEFAULT 'pending';
            ALTER TABLE chunks ADD COLUMN token_count INTEGER;
            CREATE INDEX IF NOT EXISTS idx_chunks_status ON chunks(embedding_status);

            CREATE TABLE IF NOT EXISTS embedding_jobs (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                chunk_id    INTEGER NOT NULL UNIQUE,
                created_at  INTEGER NOT NULL DEFAULT (strftime('%s','now')),
                FOREIGN KEY (chunk_id) REFERENCES chunks(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_embedding_jobs ON embedding_jobs(chunk_id);
            "#,
        ),
        M::up(
            r#"
            CREATE TABLE IF NOT EXISTS recently_viewed (
                file_id   INTEGER NOT NULL,
                viewed_at INTEGER NOT NULL DEFAULT (strftime('%s','now')),
                FOREIGN KEY (file_id) REFERENCES files(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_recently_viewed ON recently_viewed(viewed_at DESC);
            "#,
        ),
        M::up(
            r#"
            CREATE TABLE IF NOT EXISTS favorites (
                file_id    INTEGER NOT NULL UNIQUE,
                created_at INTEGER NOT NULL DEFAULT (strftime('%s','now')),
                FOREIGN KEY (file_id) REFERENCES files(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_favorites ON favorites(created_at DESC);
            "#,
        ),
    ])
}
