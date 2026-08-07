use rusqlite::{Connection, Result};
use std::sync::Mutex;

pub struct Database {
    pub conn: Mutex<Connection>,
}

impl Database {
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        Ok(Database { conn: Mutex::new(conn) })
    }

    pub fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let version: i32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;

        if version < 1 {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS sources (
                    id TEXT PRIMARY KEY,
                    title TEXT NOT NULL,
                    type TEXT NOT NULL CHECK(type IN ('url','text','file')),
                    content TEXT NOT NULL,
                    tags TEXT NOT NULL DEFAULT '[]',
                    origin TEXT NOT NULL CHECK(origin IN ('user','ai_search')),
                    source_url TEXT,
                    hidden INTEGER NOT NULL DEFAULT 0,
                    created_at TEXT NOT NULL DEFAULT (datetime('now'))
                );

                CREATE TABLE IF NOT EXISTS knowledge_points (
                    id TEXT PRIMARY KEY,
                    title TEXT NOT NULL,
                    summary TEXT NOT NULL DEFAULT '',
                    content TEXT NOT NULL DEFAULT '',
                    tags TEXT NOT NULL DEFAULT '[]',
                    source_ids TEXT NOT NULL DEFAULT '[]',
                    created_at TEXT NOT NULL DEFAULT (datetime('now'))
                );

                CREATE TABLE IF NOT EXISTS relations (
                    id TEXT PRIMARY KEY,
                    from_kp_id TEXT NOT NULL REFERENCES knowledge_points(id),
                    to_kp_id TEXT NOT NULL REFERENCES knowledge_points(id),
                    relation_type TEXT NOT NULL CHECK(relation_type IN ('prerequisite','related','extension')),
                    created_at TEXT NOT NULL DEFAULT (datetime('now'))
                );

                CREATE TABLE IF NOT EXISTS quiz_questions (
                    id TEXT PRIMARY KEY,
                    kp_id TEXT NOT NULL REFERENCES knowledge_points(id),
                    type TEXT NOT NULL CHECK(type IN ('multiple_choice','fill_blank','analysis')),
                    question TEXT NOT NULL,
                    options TEXT,
                    answer TEXT NOT NULL,
                    explanation TEXT NOT NULL DEFAULT ''
                );

                CREATE TABLE IF NOT EXISTS quiz_attempts (
                    id TEXT PRIMARY KEY,
                    question_id TEXT NOT NULL REFERENCES quiz_questions(id),
                    user_answer TEXT NOT NULL,
                    is_correct INTEGER NOT NULL,
                    attempted_at TEXT NOT NULL DEFAULT (datetime('now'))
                );

                CREATE TABLE IF NOT EXISTS mastery_records (
                    id TEXT PRIMARY KEY,
                    kp_id TEXT NOT NULL UNIQUE REFERENCES knowledge_points(id),
                    ease_factor REAL NOT NULL DEFAULT 2.5,
                    interval_days INTEGER NOT NULL DEFAULT 0,
                    repetitions INTEGER NOT NULL DEFAULT 0,
                    next_review_at TEXT NOT NULL DEFAULT (datetime('now')),
                    last_reviewed_at TEXT
                );

                CREATE TABLE IF NOT EXISTS learning_plans (
                    id TEXT PRIMARY KEY,
                    title TEXT NOT NULL,
                    goal TEXT NOT NULL DEFAULT '',
                    kp_ids TEXT NOT NULL DEFAULT '[]',
                    status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active','completed','paused')),
                    created_at TEXT NOT NULL DEFAULT (datetime('now'))
                );

                CREATE VIRTUAL TABLE IF NOT EXISTS kp_fts USING fts5(title, summary, content, content='knowledge_points', content_rowid='rowid');
                CREATE VIRTUAL TABLE IF NOT EXISTS sources_fts USING fts5(title, content, content='sources', content_rowid='rowid');

                -- FTS5 sync triggers for knowledge_points
                CREATE TRIGGER IF NOT EXISTS kp_fts_ai AFTER INSERT ON knowledge_points BEGIN
                    INSERT INTO kp_fts(rowid, title, summary, content) VALUES (new.rowid, new.title, new.summary, new.content);
                END;
                CREATE TRIGGER IF NOT EXISTS kp_fts_ad AFTER DELETE ON knowledge_points BEGIN
                    INSERT INTO kp_fts(kp_fts, rowid, title, summary, content) VALUES('delete', old.rowid, old.title, old.summary, old.content);
                END;
                CREATE TRIGGER IF NOT EXISTS kp_fts_au AFTER UPDATE ON knowledge_points BEGIN
                    INSERT INTO kp_fts(kp_fts, rowid, title, summary, content) VALUES('delete', old.rowid, old.title, old.summary, old.content);
                    INSERT INTO kp_fts(rowid, title, summary, content) VALUES (new.rowid, new.title, new.summary, new.content);
                END;

                -- FTS5 sync triggers for sources
                CREATE TRIGGER IF NOT EXISTS sources_fts_ai AFTER INSERT ON sources BEGIN
                    INSERT INTO sources_fts(rowid, title, content) VALUES (new.rowid, new.title, new.content);
                END;
                CREATE TRIGGER IF NOT EXISTS sources_fts_ad AFTER DELETE ON sources BEGIN
                    INSERT INTO sources_fts(sources_fts, rowid, title, content) VALUES('delete', old.rowid, old.title, old.content);
                END;
                CREATE TRIGGER IF NOT EXISTS sources_fts_au AFTER UPDATE ON sources BEGIN
                    INSERT INTO sources_fts(sources_fts, rowid, title, content) VALUES('delete', old.rowid, old.title, old.content);
                    INSERT INTO sources_fts(rowid, title, content) VALUES (new.rowid, new.title, new.content);
                END;

                CREATE TABLE IF NOT EXISTS settings (
                    key TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS model_providers (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    base_url TEXT NOT NULL,
                    api_key TEXT NOT NULL DEFAULT '',
                    api_format TEXT NOT NULL DEFAULT 'openai_compatible',
                    is_preset INTEGER NOT NULL DEFAULT 0,
                    is_default INTEGER NOT NULL DEFAULT 0,
                    created_at TEXT NOT NULL DEFAULT (datetime('now'))
                );

                CREATE TABLE IF NOT EXISTS provider_models (
                    id TEXT PRIMARY KEY,
                    provider_id TEXT NOT NULL REFERENCES model_providers(id),
                    model_name TEXT NOT NULL,
                    temperature REAL NOT NULL DEFAULT 0.7,
                    max_tokens INTEGER NOT NULL DEFAULT 4096,
                    is_default INTEGER NOT NULL DEFAULT 0
                );

                CREATE TABLE IF NOT EXISTS task_models (
                    id TEXT PRIMARY KEY,
                    task_name TEXT NOT NULL UNIQUE,
                    model_id TEXT,
                    FOREIGN KEY (model_id) REFERENCES provider_models(id)
                );"
            )?;
        }

        if version < 2 {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS audit_logs (
                    id TEXT PRIMARY KEY,
                    timestamp TEXT NOT NULL,
                    source TEXT NOT NULL,
                    level TEXT NOT NULL CHECK (level IN ('info','warn','error')),
                    category TEXT NOT NULL,
                    action TEXT NOT NULL,
                    user_action TEXT,
                    method TEXT,
                    path TEXT,
                    status_code INTEGER,
                    duration_ms INTEGER,
                    params_summary TEXT,
                    result_summary TEXT,
                    error_message TEXT
                );
                CREATE INDEX IF NOT EXISTS idx_audit_logs_created ON audit_logs(timestamp);"
            )?;
        }

        if version < 3 {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS chat_sessions (
                    id TEXT PRIMARY KEY,
                    title TEXT NOT NULL,
                    plan_id TEXT,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS chat_messages (
                    id TEXT PRIMARY KEY,
                    session_id TEXT NOT NULL REFERENCES chat_sessions(id) ON DELETE CASCADE,
                    role TEXT NOT NULL CHECK (role IN ('user','assistant')),
                    content TEXT NOT NULL,
                    actions TEXT,
                    context TEXT,
                    created_at TEXT NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_chat_messages_session ON chat_messages(session_id, created_at);"
            )?;
        }

        conn.pragma_update(None, "user_version", 3)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Database {
        let db = Database::new(":memory:").expect("in-memory db");
        db.migrate().expect("migrate");
        db
    }

    #[test]
    fn migrate_sets_user_version_3() {
        let db = test_db();
        let conn = db.conn.lock().unwrap();
        let v: i32 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, 3);
    }

    #[test]
    fn migrate_creates_new_tables() {
        let db = test_db();
        let conn = db.conn.lock().unwrap();
        for tbl in ["audit_logs", "chat_sessions", "chat_messages"] {
            let n: i32 = conn
                .query_row(
                    "SELECT count(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    [tbl],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(n, 1, "table {tbl} should exist");
        }
    }

    #[test]
    fn migrate_is_idempotent() {
        let db = test_db();
        db.migrate().expect("second migrate");
        let conn = db.conn.lock().unwrap();
        let v: i32 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, 3);
    }
}
