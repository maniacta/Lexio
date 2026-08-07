use crate::db::Database;
use crate::models::new_id;
use chrono::Utc;

pub struct ChatSession {
    pub id: String,
    pub title: String,
    pub plan_id: Option<String>,
    pub message_count: i64,
    pub updated_at: String,
}

pub struct ChatMessage {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub actions: Option<String>,
    pub context: Option<String>,
    pub created_at: String,
}

pub fn list_sessions(db: &Database) -> Result<Vec<ChatSession>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT s.id, s.title, s.plan_id, s.updated_at, COUNT(m.id) as message_count
             FROM chat_sessions s
             LEFT JOIN chat_messages m ON m.session_id = s.id
             GROUP BY s.id
             ORDER BY s.updated_at DESC",
        )
        .map_err(|e| e.to_string())?;
    let sessions: Vec<ChatSession> = stmt
        .query_map([], |row| {
            Ok(ChatSession {
                id: row.get(0)?,
                title: row.get(1)?,
                plan_id: row.get(2)?,
                updated_at: row.get(3)?,
                message_count: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(sessions)
}

pub fn create_session(db: &Database, title: &str) -> Result<ChatSession, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let id = new_id();
    conn.execute(
        "INSERT INTO chat_sessions (id, title, plan_id, created_at, updated_at) VALUES (?1, ?2, NULL, ?3, ?3)",
        rusqlite::params![id, title, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(ChatSession {
        id,
        title: title.to_string(),
        plan_id: None,
        message_count: 0,
        updated_at: now,
    })
}

pub fn get_messages(db: &Database, session_id: &str) -> Result<Vec<ChatMessage>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, session_id, role, content, actions, context, created_at
             FROM chat_messages WHERE session_id = ?1 ORDER BY created_at ASC, rowid ASC",
        )
        .map_err(|e| e.to_string())?;
    let messages: Vec<ChatMessage> = stmt
        .query_map([session_id], |row| {
            Ok(ChatMessage {
                id: row.get(0)?,
                session_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                actions: row.get(4)?,
                context: row.get(5)?,
                created_at: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(messages)
}

pub fn append_message(
    db: &Database,
    session_id: &str,
    role: &str,
    content: &str,
    actions: Option<&str>,
    context: Option<&str>,
) -> Result<ChatMessage, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let id = new_id();
    conn.execute(
        "INSERT INTO chat_messages (id, session_id, role, content, actions, context, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![id, session_id, role, content, actions, context, now],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE chat_sessions SET updated_at = ?1 WHERE id = ?2",
        rusqlite::params![now, session_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(ChatMessage {
        id,
        session_id: session_id.to_string(),
        role: role.to_string(),
        content: content.to_string(),
        actions: actions.map(|s| s.to_string()),
        context: context.map(|s| s.to_string()),
        created_at: now,
    })
}

pub fn delete_session(db: &Database, session_id: &str) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "DELETE FROM chat_messages WHERE session_id = ?1",
        rusqlite::params![session_id],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "DELETE FROM chat_sessions WHERE id = ?1",
        rusqlite::params![session_id],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn set_session_title(db: &Database, session_id: &str, title: &str) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE chat_sessions SET title = ?1 WHERE id = ?2",
        rusqlite::params![title, session_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn set_session_plan(db: &Database, session_id: &str, plan_id: &str) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE chat_sessions SET plan_id = ?1 WHERE id = ?2",
        rusqlite::params![plan_id, session_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
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
    fn session_crud_flow() {
        let db = test_db();
        let s = create_session(&db, "新对话").unwrap();
        assert_eq!(s.title, "新对话");
        assert_eq!(s.message_count, 0);

        let m = append_message(&db, &s.id, "user", "你好", None, None).unwrap();
        assert_eq!(m.role, "user");
        let msgs = get_messages(&db, &s.id).unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].content, "你好");

        set_session_title(&db, &s.id, "你好，世界").unwrap();
        set_session_plan(&db, &s.id, "plan-1").unwrap();
        let list = list_sessions(&db).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].title, "你好，世界");
        assert_eq!(list[0].plan_id.as_deref(), Some("plan-1"));
        assert_eq!(list[0].message_count, 1);

        delete_session(&db, &s.id).unwrap();
        assert!(list_sessions(&db).unwrap().is_empty());
        assert!(get_messages(&db, &s.id).unwrap().is_empty());
    }

    #[test]
    fn sessions_ordered_by_updated_at_desc() {
        let db = test_db();
        let s1 = create_session(&db, "first").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        let s2 = create_session(&db, "second").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        // 给 s1 追加消息 → updated_at 更新，s1 应排到最前
        append_message(&db, &s1.id, "user", "hi", None, None).unwrap();
        let list = list_sessions(&db).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].id, s1.id);
        assert_eq!(list[0].message_count, 1);
        assert_eq!(list[1].id, s2.id);
        assert_eq!(list[1].message_count, 0);
    }

    #[test]
    fn append_message_persists_actions_and_context() {
        let db = test_db();
        let s = create_session(&db, "new").unwrap();
        let m = append_message(
            &db,
            &s.id,
            "assistant",
            "回复",
            Some(r#"[{"type":"navigate_learning","label":"进入"}]"#),
            Some(r#"{"plan":{"id":"p1"}}"#),
        )
        .unwrap();
        let msgs = get_messages(&db, &s.id).unwrap();
        assert_eq!(msgs[0].actions.as_deref(), Some(r#"[{"type":"navigate_learning","label":"进入"}]"#));
        assert_eq!(msgs[0].context.as_deref(), Some(r#"{"plan":{"id":"p1"}}"#));
        assert_eq!(m.role, "assistant");
    }
}
