use crate::db::Database;
use crate::models::{
    new_id, CreateKnowledgePointRequest, CreateLearningPlanRequest, LearningPlan, MasteryRecord,
};

pub fn create_plan(db: &Database, req: &CreateLearningPlanRequest) -> Result<LearningPlan, String> {
    let id = new_id();
    let kp_ids = serde_json::to_string(&req.kp_ids).unwrap_or_default();
    let now = chrono::Utc::now().to_rfc3339();
    let plan = LearningPlan {
        id: id.clone(),
        title: req.title.clone(),
        goal: req.goal.clone(),
        kp_ids: req.kp_ids.clone(),
        status: "active".to_string(),
        created_at: now.clone(),
    };
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO learning_plans (id, title, goal, kp_ids, status, created_at) VALUES (?1, ?2, ?3, ?4, 'active', ?5)",
        rusqlite::params![id, plan.title, plan.goal, kp_ids, now],
    ).map_err(|e| e.to_string())?;
    Ok(plan)
}

pub fn list_plans(db: &Database) -> Result<Vec<LearningPlan>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, title, goal, kp_ids, status, created_at FROM learning_plans ORDER BY created_at DESC")
        .map_err(|e| e.to_string())?;
    let plans: Vec<LearningPlan> = stmt
        .query_map([], |row| plan_from_row(row))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(plans)
}

pub fn upsert_mastery(db: &Database, record: &MasteryRecord) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO mastery_records (id, kp_id, ease_factor, interval_days, repetitions, next_review_at, last_reviewed_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) ON CONFLICT(kp_id) DO UPDATE SET ease_factor=?3, interval_days=?4, repetitions=?5, next_review_at=?6, last_reviewed_at=?7",
        rusqlite::params![record.id, record.kp_id, record.ease_factor, record.interval_days, record.repetitions, record.next_review_at, record.last_reviewed_at],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_due_reviews(db: &Database) -> Result<Vec<MasteryRecord>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();
    let mut stmt = conn
        .prepare("SELECT id, kp_id, ease_factor, interval_days, repetitions, next_review_at, last_reviewed_at FROM mastery_records WHERE next_review_at <= ?1 ORDER BY next_review_at ASC")
        .map_err(|e| e.to_string())?;
    let records: Vec<MasteryRecord> = stmt
        .query_map([&now], |row| mastery_from_row(row))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(records)
}

pub fn get_mastery_by_kp(db: &Database, kp_id: &str) -> Result<Option<MasteryRecord>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, kp_id, ease_factor, interval_days, repetitions, next_review_at, last_reviewed_at FROM mastery_records WHERE kp_id = ?1")
        .map_err(|e| e.to_string())?;
    let mut rows = stmt
        .query_map([kp_id], |row| mastery_from_row(row))
        .map_err(|e| e.to_string())?;
    Ok(rows.next().and_then(|r| r.ok()))
}

/// Return due reviews along with the associated KnowledgePoint data.
pub fn get_due_reviews_with_kp(db: &Database) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();
    let mut stmt = conn
        .prepare(
            "SELECT m.id, m.kp_id, m.ease_factor, m.interval_days, m.repetitions,
                    m.next_review_at, m.last_reviewed_at,
                    kp.id, kp.title, kp.summary, kp.content, kp.tags, kp.source_ids, kp.created_at
             FROM mastery_records m
             JOIN knowledge_points kp ON m.kp_id = kp.id
             WHERE m.next_review_at <= ?1
             ORDER BY m.next_review_at ASC"
        )
        .map_err(|e| e.to_string())?;

    let items: Vec<serde_json::Value> = stmt
        .query_map([&now], |row| {
            let tags_raw: String = row.get(11)?;
            let tags: Vec<String> = serde_json::from_str(&tags_raw).unwrap_or_default();
            let source_ids_raw: String = row.get(12)?;
            let source_ids: Vec<String> = serde_json::from_str(&source_ids_raw).unwrap_or_default();
            Ok(serde_json::json!({
                "mastery": {
                    "id": row.get::<_, String>(0)?,
                    "kp_id": row.get::<_, String>(1)?,
                    "ease_factor": row.get::<_, f64>(2)?,
                    "interval_days": row.get::<_, i32>(3)?,
                    "repetitions": row.get::<_, i32>(4)?,
                    "next_review_at": row.get::<_, String>(5)?,
                    "last_reviewed_at": row.get::<_, Option<String>>(6)?
                },
                "knowledge_point": {
                    "id": row.get::<_, String>(7)?,
                    "title": row.get::<_, String>(8)?,
                    "summary": row.get::<_, String>(9)?,
                    "content": row.get::<_, String>(10)?,
                    "tags": tags,
                    "source_ids": source_ids,
                    "created_at": row.get::<_, String>(13)?
                }
            }))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(items)
}

fn plan_from_row(row: &rusqlite::Row) -> rusqlite::Result<LearningPlan> {
    let kp_ids_str: String = row.get(3)?;
    Ok(LearningPlan {
        id: row.get(0)?,
        title: row.get(1)?,
        goal: row.get(2)?,
        kp_ids: serde_json::from_str(&kp_ids_str).unwrap_or_default(),
        status: row.get(4)?,
        created_at: row.get(5)?,
    })
}

fn mastery_from_row(row: &rusqlite::Row) -> rusqlite::Result<MasteryRecord> {
    Ok(MasteryRecord {
        id: row.get(0)?,
        kp_id: row.get(1)?,
        ease_factor: row.get(2)?,
        interval_days: row.get(3)?,
        repetitions: row.get(4)?,
        next_review_at: row.get(5)?,
        last_reviewed_at: row.get(6)?,
    })
}

/// Persist sources + knowledge points + plan atomically after LLM research.
pub fn persist_research_bundle(
    db: &Database,
    source_reqs: &[crate::models::CreateSourceRequest],
    kp_reqs: &[CreateKnowledgePointRequest],
    plan_req: &CreateLearningPlanRequest,
) -> Result<crate::models::AiResearchResult, String> {
    use crate::models::{KnowledgePoint, Source};

    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;

    let mut sources = Vec::new();
    for req in source_reqs {
        let id = new_id();
        let tags = serde_json::to_string(&req.tags).unwrap_or_default();
        let now = chrono::Utc::now().to_rfc3339();
        let source = Source {
            id: id.clone(),
            title: req.title.clone(),
            source_type: req.source_type.clone(),
            content: req.content.clone(),
            tags: req.tags.clone(),
            origin: req.origin.clone(),
            source_url: req.source_url.clone(),
            hidden: false,
            created_at: now.clone(),
        };
        tx.execute(
            "INSERT INTO sources (id, title, type, content, tags, origin, source_url, hidden, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8)",
            rusqlite::params![
                id,
                source.title,
                source.source_type,
                source.content,
                tags,
                source.origin,
                source.source_url,
                now
            ],
        )
        .map_err(|e| e.to_string())?;
        sources.push(source);
    }

    let mut kps = Vec::new();
    for req in kp_reqs {
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
        tx.execute(
            "INSERT INTO knowledge_points (id, title, summary, content, tags, source_ids, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![id, kp.title, kp.summary, kp.content, tags, source_ids, now],
        )
        .map_err(|e| e.to_string())?;
        kps.push(kp);
    }

    let plan_id = new_id();
    let kp_ids: Vec<String> = kps.iter().map(|k| k.id.clone()).collect();
    let kp_ids_json = serde_json::to_string(&kp_ids).unwrap_or_default();
    let now = chrono::Utc::now().to_rfc3339();
    let plan = LearningPlan {
        id: plan_id.clone(),
        title: plan_req.title.clone(),
        goal: plan_req.goal.clone(),
        kp_ids: kp_ids.clone(),
        status: "active".to_string(),
        created_at: now.clone(),
    };
    tx.execute(
        "INSERT INTO learning_plans (id, title, goal, kp_ids, status, created_at)
         VALUES (?1, ?2, ?3, ?4, 'active', ?5)",
        rusqlite::params![plan_id, plan.title, plan.goal, kp_ids_json, now],
    )
    .map_err(|e| e.to_string())?;

    tx.commit().map_err(|e| e.to_string())?;
    Ok(crate::models::AiResearchResult {
        sources,
        knowledge_points: kps,
        plan,
    })
}
