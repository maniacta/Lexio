use crate::db::Database;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    pub id: String,
    pub timestamp: String,
    pub source: String,
    pub level: String,
    pub category: String,
    pub action: String,
    pub user_action: Option<String>,
    pub method: Option<String>,
    pub path: Option<String>,
    pub status_code: Option<i32>,
    pub duration_ms: Option<i64>,
    pub params_summary: Option<String>,
    pub result_summary: Option<String>,
    pub error_message: Option<String>,
}

pub fn insert(db: &Database, record: &AuditRecord) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO audit_logs (id, timestamp, source, level, category, action, user_action, method, path, status_code, duration_ms, params_summary, result_summary, error_message)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        rusqlite::params![
            record.id,
            record.timestamp,
            record.source,
            record.level,
            record.category,
            record.action,
            record.user_action,
            record.method,
            record.path,
            record.status_code,
            record.duration_ms,
            record.params_summary,
            record.result_summary,
            record.error_message,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn batch_insert(db: &Database, records: &[AuditRecord]) -> Result<(), String> {
    if records.is_empty() {
        return Ok(());
    }
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "INSERT INTO audit_logs (id, timestamp, source, level, category, action, user_action, method, path, status_code, duration_ms, params_summary, result_summary, error_message)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        )
        .map_err(|e| e.to_string())?;
    for record in records {
        stmt.execute(rusqlite::params![
            record.id,
            record.timestamp,
            record.source,
            record.level,
            record.category,
            record.action,
            record.user_action,
            record.method,
            record.path,
            record.status_code,
            record.duration_ms,
            record.params_summary,
            record.result_summary,
            record.error_message,
        ])
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Most recent audit rows, newest first.
pub fn list(db: &Database, limit: i64) -> Result<Vec<AuditRecord>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, timestamp, source, level, category, action, user_action, method, path, status_code, duration_ms, params_summary, result_summary, error_message
             FROM audit_logs ORDER BY timestamp DESC LIMIT ?1",
        )
        .map_err(|e| e.to_string())?;
    let rows: Vec<AuditRecord> = stmt
        .query_map([limit], |row| {
            Ok(AuditRecord {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                source: row.get(2)?,
                level: row.get(3)?,
                category: row.get(4)?,
                action: row.get(5)?,
                user_action: row.get(6)?,
                method: row.get(7)?,
                path: row.get(8)?,
                status_code: row.get(9)?,
                duration_ms: row.get(10)?,
                params_summary: row.get(11)?,
                result_summary: row.get(12)?,
                error_message: row.get(13)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}

/// Delete audit rows older than `retention_days` (timestamps are RFC3339 UTC).
pub fn prune(db: &Database, retention_days: i64) {
    if retention_days <= 0 {
        return;
    }
    let cutoff = (chrono::Utc::now() - chrono::Duration::days(retention_days)).to_rfc3339();
    if let Ok(conn) = db.conn.lock() {
        let _ = conn.execute("DELETE FROM audit_logs WHERE timestamp < ?1", [&cutoff]);
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

    fn rec(action: &str, days_ago: i64) -> AuditRecord {
        let ts = (chrono::Utc::now() - chrono::Duration::days(days_ago))
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        AuditRecord {
            id: crate::models::new_id(),
            timestamp: ts,
            source: "backend".into(),
            level: "info".into(),
            category: "test".into(),
            action: action.into(),
            user_action: None,
            method: None,
            path: None,
            status_code: None,
            duration_ms: None,
            params_summary: None,
            result_summary: None,
            error_message: None,
        }
    }

    #[test]
    fn insert_and_list_roundtrip() {
        let db = test_db();
        insert(&db, &rec("a", 0)).unwrap();
        let rows = list(&db, 10).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].action, "a");
    }

    #[test]
    fn batch_insert_transactional() {
        let db = test_db();
        let mut batch = vec![rec("b1", 0), rec("b2", 0)];
        batch_insert(&db, &batch).unwrap();
        assert_eq!(list(&db, 10).unwrap().len(), 2);
    }

    #[test]
    fn prune_removes_old_rows_keeps_recent() {
        let db = test_db();
        insert(&db, &rec("old", 31)).unwrap();
        insert(&db, &rec("new", 1)).unwrap();
        prune(&db, 30);
        let rows = list(&db, 10).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].action, "new");
    }
}
