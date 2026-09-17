use crate::db::Database;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    pub id: String,
    /// Server clock, always. The authority for ordering and retention.
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
    /// The clock of the originating client, kept for diagnostics only.
    ///
    /// Untrusted: it is never used for ordering or for retention pruning, so a
    /// client cannot back-date an event out of the trail or place it in the
    /// future. `None` for rows written by the backend itself.
    pub client_timestamp: Option<String>,
}

/// Column list shared by every read, so the mapping in [`row_to_record`] stays
/// in step with the query.
const COLUMNS: &str = "id, timestamp, source, level, category, action, user_action, \
     method, path, status_code, duration_ms, params_summary, result_summary, \
     error_message, client_timestamp";

/// Largest page the API will return, regardless of what a client asks for.
pub const MAX_PAGE_SIZE: i64 = 200;

/// Row mapping for [`COLUMNS`].
fn row_to_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<AuditRecord> {
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
        client_timestamp: row.get(14)?,
    })
}

pub fn insert(db: &Database, record: &AuditRecord) -> Result<(), String> {
    let conn = db.conn.lock().map_err(crate::error::internal)?;
    conn.execute(
        "INSERT INTO audit_logs (id, timestamp, source, level, category, action, user_action, method, path, status_code, duration_ms, params_summary, result_summary, error_message, client_timestamp)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
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
            record.client_timestamp,
        ],
    )
    .map_err(crate::error::internal)?;
    Ok(())
}

/// Insert a batch in one transaction.
///
/// Without the transaction each `execute` committed on its own, so a failure
/// partway through (an oversized value, a duplicate id) left the earlier rows
/// written while the caller saw an error. The caller cannot tell how much
/// landed, so a retry duplicates those rows and the 500 on `/api/logs/batch`
/// was misleading about what state the trail was in.
pub fn batch_insert(db: &Database, records: &[AuditRecord]) -> Result<(), String> {
    if records.is_empty() {
        return Ok(());
    }
    let mut conn = db.conn.lock().map_err(crate::error::internal)?;
    let tx = conn.transaction().map_err(crate::error::internal)?;
    {
        let mut stmt = tx
            .prepare(
                "INSERT INTO audit_logs (id, timestamp, source, level, category, action, user_action, method, path, status_code, duration_ms, params_summary, result_summary, error_message, client_timestamp)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            )
            .map_err(crate::error::internal)?;
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
                record.client_timestamp,
            ])
            .map_err(crate::error::internal)?;
        }
    }
    tx.commit().map_err(crate::error::internal)?;
    Ok(())
}

/// Filter for reading the audit trail back.
///
/// Every field is optional; `offset`/`limit` bound the page. `limit` is clamped
/// to [`MAX_PAGE_SIZE`] so a query can never pull the whole table into memory.
#[derive(Debug, Clone)]
pub struct AuditFilter {
    pub level: Option<String>,
    pub source: Option<String>,
    pub category: Option<String>,
    /// Substring match over action / user_action / path / error_message.
    pub search: Option<String>,
    /// RFC3339 lower bound (inclusive) on the server timestamp.
    pub since: Option<String>,
    /// RFC3339 upper bound (inclusive) on the server timestamp.
    pub until: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

/// A filter that selects everything, bounded by one maximum page.
///
/// The limit is set here rather than left at zero: `query` clamps a non-positive
/// limit to one row, so a derived `Default` would silently turn "no filter" into
/// "one row".
impl Default for AuditFilter {
    fn default() -> Self {
        Self {
            level: None,
            source: None,
            category: None,
            search: None,
            since: None,
            until: None,
            offset: 0,
            limit: MAX_PAGE_SIZE,
        }
    }
}

/// Escape LIKE wildcards so a search for "50%" cannot match everything.
fn escape_like(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// Build the `WHERE` clause and its bound parameters.
fn build_filter(filter: &AuditFilter) -> (String, Vec<Box<dyn rusqlite::ToSql>>) {
    let mut clauses: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    let eq = |clauses: &mut Vec<String>,
              params: &mut Vec<Box<dyn rusqlite::ToSql>>,
              column: &str,
              value: &Option<String>| {
        if let Some(v) = value.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
            clauses.push(format!("{column} = ?"));
            params.push(Box::new(v.to_string()));
        }
    };
    eq(&mut clauses, &mut params, "level", &filter.level);
    eq(&mut clauses, &mut params, "source", &filter.source);
    eq(&mut clauses, &mut params, "category", &filter.category);

    if let Some(v) = filter
        .search
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        clauses.push(
            "(action LIKE ? ESCAPE '\\' OR user_action LIKE ? ESCAPE '\\' \
             OR path LIKE ? ESCAPE '\\' OR error_message LIKE ? ESCAPE '\\')"
                .to_string(),
        );
        let pattern = format!("%{}%", escape_like(v));
        for _ in 0..4 {
            params.push(Box::new(pattern.clone()));
        }
    }

    // Timestamp bounds are compared as text, which is correct for RFC3339 UTC
    // values written in a single format.
    if let Some(v) = filter
        .since
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        clauses.push("timestamp >= ?".to_string());
        params.push(Box::new(v.to_string()));
    }
    if let Some(v) = filter
        .until
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        clauses.push("timestamp <= ?".to_string());
        params.push(Box::new(v.to_string()));
    }

    let where_sql = if clauses.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", clauses.join(" AND "))
    };
    (where_sql, params)
}

/// A page of audit rows, newest first.
pub fn query(db: &Database, filter: &AuditFilter) -> Result<Vec<AuditRecord>, String> {
    let conn = db.conn.lock().map_err(crate::error::internal)?;
    let (where_sql, mut params) = build_filter(filter);
    let limit = filter.limit.clamp(1, MAX_PAGE_SIZE);
    let offset = filter.offset.max(0);

    let sql = format!(
        "SELECT {COLUMNS} FROM audit_logs{where_sql} \
         ORDER BY timestamp DESC, id DESC LIMIT ? OFFSET ?"
    );
    params.push(Box::new(limit));
    params.push(Box::new(offset));

    let mut stmt = conn.prepare(&sql).map_err(crate::error::internal)?;
    let rows = stmt
        .query_map(
            rusqlite::params_from_iter(params.iter().map(|p| &**p as &dyn rusqlite::ToSql)),
            row_to_record,
        )
        .map_err(crate::error::internal)?
        .collect::<rusqlite::Result<Vec<AuditRecord>>>()
        .map_err(crate::error::internal)?;
    Ok(rows)
}

/// Total number of rows matching `filter` (ignoring offset/limit), for paging.
pub fn count(db: &Database, filter: &AuditFilter) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(crate::error::internal)?;
    let (where_sql, params) = build_filter(filter);
    let sql = format!("SELECT count(*) FROM audit_logs{where_sql}");
    let total = conn
        .query_row(
            &sql,
            rusqlite::params_from_iter(params.iter().map(|p| &**p as &dyn rusqlite::ToSql)),
            |r| r.get(0),
        )
        .map_err(crate::error::internal)?;
    Ok(total)
}

/// Distinct values actually present, so the filter UI offers real choices
/// instead of a hard-coded list that drifts from what is being written.
#[derive(Debug, Clone, Default, Serialize)]
pub struct AuditFacets {
    pub levels: Vec<String>,
    pub sources: Vec<String>,
    pub categories: Vec<String>,
}

pub fn facets(db: &Database) -> Result<AuditFacets, String> {
    let conn = db.conn.lock().map_err(crate::error::internal)?;
    let distinct = |column: &str| -> Result<Vec<String>, String> {
        let sql = format!("SELECT DISTINCT {column} FROM audit_logs ORDER BY {column}");
        let mut stmt = conn.prepare(&sql).map_err(crate::error::internal)?;
        let values = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(crate::error::internal)?
            .collect::<rusqlite::Result<Vec<String>>>()
            .map_err(crate::error::internal)?;
        Ok(values)
    };
    Ok(AuditFacets {
        levels: distinct("level")?,
        sources: distinct("source")?,
        categories: distinct("category")?,
    })
}

/// Most recent audit rows, newest first.
pub fn list(db: &Database, limit: i64) -> Result<Vec<AuditRecord>, String> {
    query(
        db,
        &AuditFilter {
            limit,
            ..Default::default()
        },
    )
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
            client_timestamp: None,
        }
    }

    /// A row shaped like one that arrived from the frontend logger.
    fn frontend_rec(level: &str, category: &str, action: &str) -> AuditRecord {
        let mut r = rec(action, 0);
        r.source = "frontend".into();
        r.level = level.into();
        r.category = category.into();
        r
    }

    #[test]
    fn insert_and_list_roundtrip() {
        let db = test_db();
        insert(&db, &rec("a", 0)).unwrap();
        let rows = list(&db, 10).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].action, "a");
        assert_eq!(rows[0].client_timestamp, None);
    }

    /// The client's own clock must survive as data, but in its own column.
    #[test]
    fn client_timestamp_is_stored_separately() {
        let db = test_db();
        let mut r = rec("a", 0);
        r.client_timestamp = Some("2020-01-01T00:00:00.000Z".into());
        insert(&db, &r).unwrap();

        let row = list(&db, 1).unwrap().remove(0);
        assert_eq!(
            row.client_timestamp.as_deref(),
            Some("2020-01-01T00:00:00.000Z"),
            "the client value is kept for diagnostics"
        );
        assert_ne!(
            row.timestamp, "2020-01-01T00:00:00.000Z",
            "but the server timestamp is authoritative"
        );
    }

    /// A batch is all-or-nothing: a failure partway through must not leave the
    /// earlier rows behind, or a retry would duplicate them.
    #[test]
    fn batch_insert_is_atomic() {
        let db = test_db();
        let batch = vec![rec("b1", 0), rec("b2", 0)];
        batch_insert(&db, &batch).unwrap();
        assert_eq!(list(&db, 10).unwrap().len(), 2);

        // Two records sharing an id: the second INSERT hits the primary key.
        let dup = rec("dup", 0);
        let failing = vec![rec("first", 0), dup.clone(), dup];
        assert!(batch_insert(&db, &failing).is_err());

        let rows = list(&db, 10).unwrap();
        assert_eq!(rows.len(), 2, "the failed batch must leave no partial rows");
        assert!(
            rows.iter().all(|r| r.action != "first"),
            "the row written before the failure must be rolled back"
        );
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

    // ── Query interface ──

    #[test]
    fn filters_by_level_source_and_category() {
        let db = test_db();
        insert(&db, &frontend_rec("error", "ui", "render_error")).unwrap();
        insert(&db, &frontend_rec("info", "ai", "send_chat")).unwrap();
        insert(&db, &frontend_rec("info", "ui", "window_error")).unwrap();

        let by_level = query(
            &db,
            &AuditFilter {
                level: Some("error".into()),
                limit: 10,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(by_level.len(), 1);
        assert_eq!(by_level[0].action, "render_error");

        let by_source = query(
            &db,
            &AuditFilter {
                source: Some("frontend".into()),
                limit: 10,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(by_source.len(), 3);

        let by_category = query(
            &db,
            &AuditFilter {
                category: Some("ui".into()),
                limit: 10,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(by_category.len(), 2);
    }

    #[test]
    fn search_matches_action_and_error_message() {
        let db = test_db();
        let mut failing = frontend_rec("error", "chat", "persist_message_error");
        failing.error_message = Some("UNIQUE constraint failed".into());
        insert(&db, &failing).unwrap();
        insert(&db, &frontend_rec("info", "ai", "send_chat")).unwrap();

        let hits = query(
            &db,
            &AuditFilter {
                search: Some("UNIQUE".into()),
                limit: 10,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].action, "persist_message_error");

        let by_action = query(
            &db,
            &AuditFilter {
                search: Some("send_chat".into()),
                limit: 10,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(by_action.len(), 1);
    }

    /// A search term full of LIKE metacharacters must be treated literally.
    #[test]
    fn search_escapes_like_wildcards() {
        let db = test_db();
        insert(&db, &frontend_rec("info", "ui", "click")).unwrap();
        insert(&db, &frontend_rec("info", "ui", "100%_done")).unwrap();

        let wildcard = query(
            &db,
            &AuditFilter {
                search: Some("%".into()),
                limit: 10,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(wildcard.len(), 1, "'%' must not match every row");
        assert_eq!(wildcard[0].action, "100%_done");

        let underscore = query(
            &db,
            &AuditFilter {
                search: Some("_".into()),
                limit: 10,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(underscore.len(), 1, "'_' must not match any single char");
    }

    #[test]
    fn page_size_is_clamped_and_offset_pages() {
        let db = test_db();
        for i in 0..5 {
            insert(&db, &frontend_rec("info", "ui", &format!("a{i}"))).unwrap();
        }

        let all = query(
            &db,
            &AuditFilter {
                limit: 1000,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(all.len(), 5);

        let page1 = query(
            &db,
            &AuditFilter {
                limit: 2,
                offset: 0,
                ..Default::default()
            },
        )
        .unwrap();
        let page2 = query(
            &db,
            &AuditFilter {
                limit: 2,
                offset: 2,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(page1.len(), 2);
        assert_eq!(page2.len(), 2);
        let mut ids: Vec<&String> = page1.iter().chain(page2.iter()).map(|r| &r.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 4, "pages must not overlap");
    }

    /// An over-large limit is clamped rather than honoured.
    #[test]
    fn oversized_limit_is_clamped() {
        let db = test_db();
        let rows = query(
            &db,
            &AuditFilter {
                limit: i64::MAX,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(rows.is_empty());
        assert_eq!(MAX_PAGE_SIZE, 200);
    }

    #[test]
    fn count_ignores_paging_but_respects_filter() {
        let db = test_db();
        for _ in 0..3 {
            insert(&db, &frontend_rec("info", "ai", "send_chat")).unwrap();
        }
        insert(&db, &frontend_rec("error", "ui", "render_error")).unwrap();

        let total = count(&db, &AuditFilter::default()).unwrap();
        assert_eq!(total, 4, "count must not be limited by the page size");

        let errors = count(
            &db,
            &AuditFilter {
                level: Some("error".into()),
                limit: 1,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(errors, 1);
    }

    #[test]
    fn time_range_bounds_are_inclusive() {
        let db = test_db();
        insert(&db, &rec("old", 10)).unwrap();
        insert(&db, &rec("new", 0)).unwrap();

        let recent = query(
            &db,
            &AuditFilter {
                since: Some((chrono::Utc::now() - chrono::Duration::days(1)).to_rfc3339()),
                limit: 10,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].action, "new");
    }

    #[test]
    fn facets_report_values_in_use() {
        let db = test_db();
        insert(&db, &frontend_rec("error", "ui", "render_error")).unwrap();
        insert(&db, &frontend_rec("info", "ai", "send_chat")).unwrap();

        let f = facets(&db).unwrap();
        assert_eq!(f.levels, vec!["error".to_string(), "info".to_string()]);
        assert_eq!(f.sources, vec!["frontend".to_string()]);
        assert_eq!(f.categories, vec!["ai".to_string(), "ui".to_string()]);
    }

    #[test]
    fn empty_filter_returns_everything_newest_first() {
        let db = test_db();
        insert(&db, &rec("older", 2)).unwrap();
        insert(&db, &rec("newer", 0)).unwrap();
        let rows = query(&db, &AuditFilter::default()).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].action, "newer");
    }
}
