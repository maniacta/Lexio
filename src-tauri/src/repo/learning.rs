use crate::db::Database;
use crate::models::{new_id, CreateLearningPlanRequest, LearningPlan, MasteryRecord};

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
