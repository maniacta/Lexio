use crate::db::Database;
use crate::models::{new_id, Relation};

pub fn create_relation(db: &Database, from_kp_id: &str, to_kp_id: &str, relation_type: &str) -> Result<Relation, String> {
    let id = new_id();
    let now = chrono::Utc::now().to_rfc3339();
    let rel = Relation {
        id: id.clone(),
        from_kp_id: from_kp_id.to_string(),
        to_kp_id: to_kp_id.to_string(),
        relation_type: relation_type.to_string(),
        created_at: now.clone(),
    };
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO relations (id, from_kp_id, to_kp_id, relation_type, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![id, rel.from_kp_id, rel.to_kp_id, rel.relation_type, now],
    ).map_err(|e| e.to_string())?;
    Ok(rel)
}

pub fn get_relations_for_kp(db: &Database, kp_id: &str) -> Result<Vec<Relation>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, from_kp_id, to_kp_id, relation_type, created_at FROM relations WHERE from_kp_id = ?1 OR to_kp_id = ?1")
        .map_err(|e| e.to_string())?;
    let relations: Vec<Relation> = stmt
        .query_map([kp_id], |row| relation_from_row(row))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(relations)
}

fn relation_from_row(row: &rusqlite::Row) -> rusqlite::Result<Relation> {
    Ok(Relation {
        id: row.get(0)?,
        from_kp_id: row.get(1)?,
        to_kp_id: row.get(2)?,
        relation_type: row.get(3)?,
        created_at: row.get(4)?,
    })
}
