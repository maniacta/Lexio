use crate::db::Database;
use crate::models::{new_id, CreateKnowledgePointRequest, KnowledgePoint};

pub fn create_kp(db: &Database, req: &CreateKnowledgePointRequest) -> Result<KnowledgePoint, String> {
    let id = new_id();
    let tags = serde_json::to_string(&req.tags).unwrap_or_default();
    let source_ids = serde_json::to_string(&req.source_ids).unwrap_or_default();
    let now = chrono::Utc::now().to_rfc3339();
    let kp = KnowledgePoint {
        id: id.clone(),
        title: req.title.clone(),
        summary: req.summary.clone(),
        content: req.content.clone(),
        tags: req.tags.clone(),
        source_ids: req.source_ids.clone(),
        created_at: now.clone(),
    };
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO knowledge_points (id, title, summary, content, tags, source_ids, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![id, kp.title, kp.summary, kp.content, tags, source_ids, now],
    ).map_err(|e| e.to_string())?;
    Ok(kp)
}

pub fn list_kps(db: &Database) -> Result<Vec<KnowledgePoint>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, title, summary, content, tags, source_ids, created_at FROM knowledge_points ORDER BY created_at DESC")
        .map_err(|e| e.to_string())?;
    let kps: Vec<KnowledgePoint> = stmt
        .query_map([], |row| kp_from_row(row))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(kps)
}

pub fn get_kp(db: &Database, id: &str) -> Result<Option<KnowledgePoint>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, title, summary, content, tags, source_ids, created_at FROM knowledge_points WHERE id = ?1")
        .map_err(|e| e.to_string())?;
    let mut rows = stmt
        .query_map([id], |row| kp_from_row(row))
        .map_err(|e| e.to_string())?;
    Ok(rows.next().and_then(|r| r.ok()))
}

pub fn list_kps_by_ids(db: &Database, ids: &[String]) -> Result<Vec<KnowledgePoint>, String> {
    if ids.is_empty() {
        return Ok(vec![]);
    }
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let placeholders: Vec<String> = ids.iter().enumerate().map(|(i, _)| format!("?{}", i + 1)).collect();
    let sql = format!(
        "SELECT id, title, summary, content, tags, source_ids, created_at FROM knowledge_points WHERE id IN ({})",
        placeholders.join(",")
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let params: Vec<&dyn rusqlite::types::ToSql> = ids.iter().map(|id| id as &dyn rusqlite::types::ToSql).collect();
    let kps: Vec<KnowledgePoint> = stmt
        .query_map(params.as_slice(), |row| kp_from_row(row))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(kps)
}

pub fn search_kps(db: &Database, query: &str) -> Result<Vec<KnowledgePoint>, String> {
    // Escape the user query as an FTS5 phrase so special characters
    // (quotes, AND/OR/NOT operators, prefixes, hyphens, ...) cannot turn
    // into a query syntax error. The external-content FTS table is queried
    // via rowid JOIN against the base table.
    let escaped = format!("\"{}\"", query.replace('"', "\"\""));
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT kp.id, kp.title, kp.summary, kp.content, kp.tags, kp.source_ids, kp.created_at
             FROM knowledge_points kp
             JOIN (SELECT rowid, rank FROM kp_fts WHERE kp_fts MATCH ?1) fts ON kp.rowid = fts.rowid
             ORDER BY fts.rank",
        )
        .map_err(|e| e.to_string())?;
    let kps: Vec<KnowledgePoint> = stmt
        .query_map([&escaped], |row| kp_from_row(row))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(kps)
}

pub fn delete_kp(db: &Database, id: &str) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    // Cascade related rows (schema FKs have no ON DELETE CASCADE)
    tx.execute(
        "DELETE FROM quiz_attempts WHERE question_id IN (SELECT id FROM quiz_questions WHERE kp_id = ?1)",
        [id],
    )
    .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM quiz_questions WHERE kp_id = ?1", [id])
        .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM mastery_records WHERE kp_id = ?1", [id])
        .map_err(|e| e.to_string())?;
    tx.execute(
        "DELETE FROM relations WHERE from_kp_id = ?1 OR to_kp_id = ?1",
        [id],
    )
    .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM knowledge_points WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

fn kp_from_row(row: &rusqlite::Row) -> rusqlite::Result<KnowledgePoint> {
    let tags_str: String = row.get(4)?;
    let source_ids_str: String = row.get(5)?;
    Ok(KnowledgePoint {
        id: row.get(0)?,
        title: row.get(1)?,
        summary: row.get(2)?,
        content: row.get(3)?,
        tags: serde_json::from_str(&tags_str).unwrap_or_default(),
        source_ids: serde_json::from_str(&source_ids_str).unwrap_or_default(),
        created_at: row.get(6)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::CreateKnowledgePointRequest;

    fn test_db() -> Database {
        let db = Database::new(":memory:").expect("in-memory db");
        db.migrate().expect("migrate");
        db
    }

    fn kp_req(title: &str, summary: &str, content: &str) -> CreateKnowledgePointRequest {
        CreateKnowledgePointRequest {
            title: title.into(),
            summary: summary.into(),
            content: content.into(),
            tags: vec![],
            source_ids: vec![],
        }
    }

    #[test]
    fn search_kps_matches_via_fts() {
        let db = test_db();
        create_kp(&db, &kp_req("Rust ownership", "所有权", "Rust 的所有权系统")).unwrap();
        let hits = search_kps(&db, "所有权").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].title, "Rust ownership");
    }

    #[test]
    fn search_kps_escapes_special_chars() {
        let db = test_db();
        create_kp(&db, &kp_req("quotes", "引号", "带引号 \" 的内容")).unwrap();
        // 引号/操作符不应导致 FTS 语法错误
        let hits = search_kps(&db, "\"quote\"").unwrap();
        assert!(hits.len() <= 1);
    }
}
