use crate::db::Database;
use crate::models::{new_id, CreateSourceRequest, Source};

pub fn create_source(db: &Database, req: &CreateSourceRequest) -> Result<Source, String> {
    let id = new_id();
    let tags = serde_json::to_string(&req.tags).unwrap_or_default();
    let source = Source {
        id: id.clone(),
        title: req.title.clone(),
        source_type: req.source_type.clone(),
        content: req.content.clone(),
        tags: req.tags.clone(),
        origin: req.origin.clone(),
        source_url: req.source_url.clone(),
        hidden: false,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    let conn = db.conn.lock().map_err(crate::error::internal)?;
    conn.execute(
        "INSERT INTO sources (id, title, type, content, tags, origin, source_url, hidden, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8)",
        rusqlite::params![id, source.title, source.source_type, source.content, tags, source.origin, source.source_url, source.created_at],
    ).map_err(crate::error::internal)?;
    Ok(source)
}

pub fn list_sources(db: &Database, include_hidden: bool) -> Result<Vec<Source>, String> {
    let conn = db.conn.lock().map_err(crate::error::internal)?;
    let query = if include_hidden {
        "SELECT id, title, type, content, tags, origin, source_url, hidden, created_at FROM sources ORDER BY created_at DESC"
    } else {
        "SELECT id, title, type, content, tags, origin, source_url, hidden, created_at FROM sources WHERE hidden = 0 ORDER BY created_at DESC"
    };
    let mut stmt = conn.prepare(query).map_err(crate::error::internal)?;
    let sources = crate::repo::rows(
        stmt.query_map([], |row| source_from_row(row)),
    )?;
    Ok(sources)
}

pub fn get_source(db: &Database, id: &str) -> Result<Option<Source>, String> {
    let conn = db.conn.lock().map_err(crate::error::internal)?;
    let mut stmt = conn
        .prepare("SELECT id, title, type, content, tags, origin, source_url, hidden, created_at FROM sources WHERE id = ?1")
        .map_err(crate::error::internal)?;
    let rows = stmt
        .query_map([id], |row| source_from_row(row))
        .map_err(crate::error::internal)?;
    crate::repo::one(rows)
}

pub fn toggle_hidden(db: &Database, id: &str, hidden: bool) -> Result<(), String> {
    let conn = db.conn.lock().map_err(crate::error::internal)?;
    conn.execute(
        "UPDATE sources SET hidden = ?1 WHERE id = ?2",
        rusqlite::params![hidden as i32, id],
    ).map_err(crate::error::internal)?;
    Ok(())
}

pub fn search_sources(db: &Database, query: &str) -> Result<Vec<Source>, String> {
    // Escape as an FTS5 phrase; external-content table queried via rowid JOIN.
    let escaped = format!("\"{}\"", query.replace('"', "\"\""));
    let conn = db.conn.lock().map_err(crate::error::internal)?;
    let mut stmt = conn
        .prepare(
            "SELECT s.id, s.title, s.type, s.content, s.tags, s.origin, s.source_url, s.hidden, s.created_at
             FROM sources s
             JOIN (SELECT rowid, rank FROM sources_fts WHERE sources_fts MATCH ?1) fts ON s.rowid = fts.rowid
             ORDER BY fts.rank",
        )
        .map_err(crate::error::internal)?;
    let sources = crate::repo::rows(
        stmt.query_map([&escaped], |row| source_from_row(row)),
    )?;
    Ok(sources)
}

fn source_from_row(row: &rusqlite::Row) -> rusqlite::Result<Source> {
    let tags_str: String = row.get(4)?;
    Ok(Source {
        id: row.get(0)?,
        title: row.get(1)?,
        source_type: row.get(2)?,
        content: row.get(3)?,
        tags: serde_json::from_str(&tags_str).unwrap_or_default(),
        origin: row.get(5)?,
        source_url: row.get(6)?,
        hidden: row.get::<_, i32>(7)? != 0,
        created_at: row.get(8)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::CreateSourceRequest;

    fn test_db() -> Database {
        let db = Database::new(":memory:").expect("in-memory db");
        db.migrate().expect("migrate");
        db
    }

    fn src_req(title: &str, content: &str) -> CreateSourceRequest {
        CreateSourceRequest {
            title: title.into(),
            source_type: "text".into(),
            content: content.into(),
            tags: vec![],
            origin: "user".into(),
            source_url: None,
        }
    }

    #[test]
    fn search_sources_matches_via_fts() {
        let db = test_db();
        create_source(&db, &src_req("HTTP 缓存", "Cache-Control 与 ETag")).unwrap();
        let hits = search_sources(&db, "缓存").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].title, "HTTP 缓存");
    }

    #[test]
    fn search_sources_escapes_special_chars() {
        let db = test_db();
        create_source(&db, &src_req("quotes", "带引号 \" 的内容")).unwrap();
        let hits = search_sources(&db, "\"quote\"").unwrap();
        assert!(hits.len() <= 1);
    }

    /// A type-mismatched column used to vanish: `filter_map(|r| r.ok())` skipped
    /// the row and `list_sources` returned only the well-formed ones. The same
    /// row through `get_source` used to look like "not found".
    #[test]
    fn a_corrupt_row_is_an_error_not_a_shorter_list() {
        let db = test_db();
        create_source(&db, &src_req("ok", "content")).unwrap();
        {
            let conn = db.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO sources (id, title, type, content, tags, origin, hidden)
                 VALUES ('bad', 't', 'text', 'c', '[]', 'user', 'not-an-int')",
                [],
            )
            .unwrap();
        }

        let err = list_sources(&db, true).unwrap_err();
        assert!(
            crate::error::is_internal(&err),
            "mapping failure must not look like a user-facing 400: {err}"
        );

        let get_err = get_source(&db, "bad").unwrap_err();
        assert!(
            crate::error::is_internal(&get_err),
            "a corrupt row must not look missing: {get_err}"
        );
    }
}
