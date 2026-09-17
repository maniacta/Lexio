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
    let conn = db.conn.lock().map_err(crate::error::internal)?;
    conn.execute(
        "INSERT INTO learning_plans (id, title, goal, kp_ids, status, created_at) VALUES (?1, ?2, ?3, ?4, 'active', ?5)",
        rusqlite::params![id, plan.title, plan.goal, kp_ids, now],
    ).map_err(crate::error::internal)?;
    Ok(plan)
}

pub fn list_plans(db: &Database) -> Result<Vec<LearningPlan>, String> {
    let conn = db.conn.lock().map_err(crate::error::internal)?;
    let mut stmt = conn
        .prepare("SELECT id, title, goal, kp_ids, status, created_at FROM learning_plans ORDER BY created_at DESC")
        .map_err(crate::error::internal)?;
    let plans = crate::repo::rows(
        stmt.query_map([], |row| plan_from_row(row)),
    )?;
    Ok(plans)
}

pub fn get_plan(db: &Database, id: &str) -> Result<Option<LearningPlan>, String> {
    let conn = db.conn.lock().map_err(crate::error::internal)?;
    let mut stmt = conn
        .prepare("SELECT id, title, goal, kp_ids, status, created_at FROM learning_plans WHERE id = ?1")
        .map_err(crate::error::internal)?;
    let rows = stmt
        .query_map([id], |row| plan_from_row(row))
        .map_err(crate::error::internal)?;
    crate::repo::one(rows)
}

pub fn upsert_mastery(db: &Database, record: &MasteryRecord) -> Result<(), String> {
    let conn = db.conn.lock().map_err(crate::error::internal)?;
    conn.execute(
        "INSERT INTO mastery_records (id, kp_id, ease_factor, interval_days, repetitions, next_review_at, last_reviewed_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) ON CONFLICT(kp_id) DO UPDATE SET ease_factor=?3, interval_days=?4, repetitions=?5, next_review_at=?6, last_reviewed_at=?7",
        rusqlite::params![record.id, record.kp_id, record.ease_factor, record.interval_days, record.repetitions, record.next_review_at, record.last_reviewed_at],
    ).map_err(crate::error::internal)?;
    Ok(())
}

pub fn get_due_reviews(db: &Database) -> Result<Vec<MasteryRecord>, String> {
    let conn = db.conn.lock().map_err(crate::error::internal)?;
    let now = chrono::Utc::now().to_rfc3339();
    let mut stmt = conn
        .prepare("SELECT id, kp_id, ease_factor, interval_days, repetitions, next_review_at, last_reviewed_at FROM mastery_records WHERE next_review_at <= ?1 ORDER BY next_review_at ASC")
        .map_err(crate::error::internal)?;
    let records = crate::repo::rows(
        stmt.query_map([&now], |row| mastery_from_row(row)),
    )?;
    Ok(records)
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
    fn get_plan_returns_existing_plan() {
        let db = test_db();
        let req = CreateLearningPlanRequest {
            title: "title".into(),
            goal: "goal".into(),
            kp_ids: vec![],
        };
        let plan = create_plan(&db, &req).unwrap();
        let fetched = get_plan(&db, &plan.id).unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().title, "title");
    }

    #[test]
    fn get_plan_returns_none_for_missing() {
        let db = test_db();
        assert!(get_plan(&db, "nope").unwrap().is_none());
    }

    fn src(title: &str, url: Option<&str>) -> crate::models::CreateSourceRequest {
        crate::models::CreateSourceRequest {
            title: title.into(),
            source_type: "text".into(),
            content: format!("content of {title}"),
            tags: vec![],
            origin: "ai_search".into(),
            source_url: url.map(|s| s.into()),
        }
    }

    fn kp(title: &str) -> CreateKnowledgePointRequest {
        CreateKnowledgePointRequest {
            title: title.into(),
            summary: "s".into(),
            content: "c".into(),
            tags: vec![],
            source_ids: vec![],
        }
    }

    fn plan(title: &str) -> CreateLearningPlanRequest {
        CreateLearningPlanRequest {
            title: title.into(),
            goal: "g".into(),
            kp_ids: vec![],
        }
    }

    fn counts(db: &Database) -> (i64, i64, i64) {
        let conn = db.conn.lock().unwrap();
        let sources: i64 = conn
            .query_row("SELECT COUNT(*) FROM sources", [], |r| r.get(0))
            .unwrap();
        let kps: i64 = conn
            .query_row("SELECT COUNT(*) FROM knowledge_points", [], |r| r.get(0))
            .unwrap();
        let plans: i64 = conn
            .query_row("SELECT COUNT(*) FROM learning_plans", [], |r| r.get(0))
            .unwrap();
        (sources, kps, plans)
    }

    #[test]
    fn persist_research_bundle_reuses_matching_rows() {
        let db = test_db();
        let sources = vec![src("HTTP", None)];
        let kps = vec![kp("Caching")];
        let first = persist_research_bundle(&db, &sources, &kps, &plan("HTTP")).unwrap();
        let second = persist_research_bundle(&db, &sources, &kps, &plan("http")).unwrap();

        assert_eq!(counts(&db), (1, 1, 1), "a repeat must not insert duplicates");
        assert_eq!(first.sources[0].id, second.sources[0].id);
        assert_eq!(first.knowledge_points[0].id, second.knowledge_points[0].id);
        assert_eq!(first.plan.id, second.plan.id);
    }

    #[test]
    fn persist_research_bundle_merges_new_kps_into_the_existing_plan() {
        let db = test_db();
        persist_research_bundle(&db, &[src("HTTP", None)], &[kp("Caching")], &plan("HTTP"))
            .unwrap();
        let second = persist_research_bundle(
            &db,
            &[src("HTTP", None)],
            &[kp("Caching"), kp("ETag")],
            &plan("HTTP"),
        )
        .unwrap();

        assert_eq!(counts(&db), (1, 2, 1));
        assert_eq!(second.plan.kp_ids.len(), 2);
    }

    #[test]
    fn persist_research_bundle_matches_sources_by_url() {
        let db = test_db();
        let url = Some("https://example.com/http");
        let first = persist_research_bundle(
            &db,
            &[src("Old title", url)],
            &[kp("A")],
            &plan("T"),
        )
        .unwrap();
        let second = persist_research_bundle(
            &db,
            &[src("New title", url)],
            &[kp("A")],
            &plan("T"),
        )
        .unwrap();
        assert_eq!(counts(&db), (1, 1, 1));
        assert_eq!(first.sources[0].id, second.sources[0].id);
    }
}

pub fn get_mastery_by_kp(db: &Database, kp_id: &str) -> Result<Option<MasteryRecord>, String> {
    let conn = db.conn.lock().map_err(crate::error::internal)?;
    let mut stmt = conn
        .prepare("SELECT id, kp_id, ease_factor, interval_days, repetitions, next_review_at, last_reviewed_at FROM mastery_records WHERE kp_id = ?1")
        .map_err(crate::error::internal)?;
    let rows = stmt
        .query_map([kp_id], |row| mastery_from_row(row))
        .map_err(crate::error::internal)?;
    crate::repo::one(rows)
}

/// Return due reviews along with the associated KnowledgePoint data.
pub fn get_due_reviews_with_kp(db: &Database) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(crate::error::internal)?;
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
        .map_err(crate::error::internal)?;

    let items = crate::repo::rows(
        stmt.query_map([&now], |row| {
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
        }),
    )?;

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

    let conn = db.conn.lock().map_err(crate::error::internal)?;
    let tx = conn.unchecked_transaction().map_err(crate::error::internal)?;

    let mut sources = Vec::new();
    for req in source_reqs {
        if let Some(existing) = reuse_source(&tx, req)? {
            sources.push(existing);
            continue;
        }
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
        .map_err(crate::error::internal)?;
        sources.push(source);
    }

    let mut kps = Vec::new();
    for req in kp_reqs {
        if let Some(existing) = reuse_kp(&tx, &req.title)? {
            kps.push(existing);
            continue;
        }
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
        .map_err(crate::error::internal)?;
        kps.push(kp);
    }

    let kp_ids: Vec<String> = kps.iter().map(|k| k.id.clone()).collect();
    let plan = if let Some(mut existing) = reuse_plan(&tx, &plan_req.title)? {
        let mut merged = existing.kp_ids.clone();
        for id in &kp_ids {
            if !merged.contains(id) {
                merged.push(id.clone());
            }
        }
        if merged != existing.kp_ids {
            let json = serde_json::to_string(&merged).unwrap_or_default();
            tx.execute(
                "UPDATE learning_plans SET kp_ids = ?1 WHERE id = ?2",
                rusqlite::params![json, existing.id],
            )
            .map_err(crate::error::internal)?;
            existing.kp_ids = merged;
        }
        existing
    } else {
        let plan_id = new_id();
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
        .map_err(crate::error::internal)?;
        plan
    };

    tx.commit().map_err(crate::error::internal)?;
    Ok(crate::models::AiResearchResult {
        sources,
        knowledge_points: kps,
        plan,
    })
}

const SOURCE_COLUMNS: &str =
    "id, title, type, content, tags, origin, source_url, hidden, created_at";
const KP_COLUMNS: &str = "id, title, summary, content, tags, source_ids, created_at";
const PLAN_COLUMNS: &str = "id, title, goal, kp_ids, status, created_at";

fn reuse_source(
    tx: &rusqlite::Transaction<'_>,
    req: &crate::models::CreateSourceRequest,
) -> Result<Option<crate::models::Source>, String> {
    if let Some(url) = req
        .source_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let mut stmt = tx
            .prepare(&format!(
                "SELECT {SOURCE_COLUMNS} FROM sources WHERE source_url = ?1 ORDER BY created_at ASC LIMIT 1"
            ))
            .map_err(crate::error::internal)?;
        let rows = stmt
            .query_map([url], crate::repo::source::source_from_row)
            .map_err(crate::error::internal)?;
        return crate::repo::one(rows);
    }
    let title = req.title.trim();
    if title.is_empty() {
        return Ok(None);
    }
    let mut stmt = tx
        .prepare(&format!(
            "SELECT {SOURCE_COLUMNS} FROM sources
             WHERE origin = ?1 AND lower(title) = lower(?2)
             ORDER BY created_at ASC LIMIT 1"
        ))
        .map_err(crate::error::internal)?;
    let rows = stmt
        .query_map(
            rusqlite::params![req.origin, title],
            crate::repo::source::source_from_row,
        )
        .map_err(crate::error::internal)?;
    crate::repo::one(rows)
}

fn reuse_kp(
    tx: &rusqlite::Transaction<'_>,
    title: &str,
) -> Result<Option<crate::models::KnowledgePoint>, String> {
    let title = title.trim();
    if title.is_empty() {
        return Ok(None);
    }
    let mut stmt = tx
        .prepare(&format!(
            "SELECT {KP_COLUMNS} FROM knowledge_points
             WHERE lower(title) = lower(?1)
             ORDER BY created_at ASC LIMIT 1"
        ))
        .map_err(crate::error::internal)?;
    let rows = stmt
        .query_map([title], crate::repo::knowledge::kp_from_row)
        .map_err(crate::error::internal)?;
    crate::repo::one(rows)
}

fn reuse_plan(
    tx: &rusqlite::Transaction<'_>,
    title: &str,
) -> Result<Option<LearningPlan>, String> {
    let title = title.trim();
    if title.is_empty() {
        return Ok(None);
    }
    let mut stmt = tx
        .prepare(&format!(
            "SELECT {PLAN_COLUMNS} FROM learning_plans
             WHERE status = 'active' AND lower(title) = lower(?1)
             ORDER BY created_at ASC LIMIT 1"
        ))
        .map_err(crate::error::internal)?;
    let rows = stmt
        .query_map([title], plan_from_row)
        .map_err(crate::error::internal)?;
    crate::repo::one(rows)
}
