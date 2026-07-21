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
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO sources (id, title, type, content, tags, origin, source_url, hidden, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8)",
        rusqlite::params![id, source.title, source.source_type, source.content, tags, source.origin, source.source_url, source.created_at],
    ).map_err(|e| e.to_string())?;
    Ok(source)
}

pub fn list_sources(db: &Database, include_hidden: bool) -> Result<Vec<Source>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let query = if include_hidden {
        "SELECT id, title, type, content, tags, origin, source_url, hidden, created_at FROM sources ORDER BY created_at DESC"
    } else {
        "SELECT id, title, type, content, tags, origin, source_url, hidden, created_at FROM sources WHERE hidden = 0 ORDER BY created_at DESC"
    };
    let mut stmt = conn.prepare(query).map_err(|e| e.to_string())?;
    let sources: Vec<Source> = stmt
        .query_map([], |row| source_from_row(row))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(sources)
}

pub fn get_source(db: &Database, id: &str) -> Result<Option<Source>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, title, type, content, tags, origin, source_url, hidden, created_at FROM sources WHERE id = ?1")
        .map_err(|e| e.to_string())?;
    let mut rows = stmt
        .query_map([id], |row| source_from_row(row))
        .map_err(|e| e.to_string())?;
    Ok(rows.next().and_then(|r| r.ok()))
}

pub fn toggle_hidden(db: &Database, id: &str, hidden: bool) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE sources SET hidden = ?1 WHERE id = ?2",
        rusqlite::params![hidden as i32, id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn search_sources(db: &Database, query: &str) -> Result<Vec<Source>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, title, type, content, tags, origin, source_url, hidden, created_at FROM sources WHERE sources_fts MATCH ?1 ORDER BY rank")
        .map_err(|e| e.to_string())?;
    let sources: Vec<Source> = stmt
        .query_map([query], |row| source_from_row(row))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
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
