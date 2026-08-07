use axum::{extract::State, http::StatusCode, Json};
use crate::ai::truncate_chars;
use crate::api::blocking;
use crate::db::Database;
use crate::models::{AiStartResearchRequest, CreateLearningPlanRequest};
use crate::repo;

const MAX_TOPIC_CHARS: usize = 500;
const MAX_PROMPT_CHARS: usize = 24_000;
const MAX_QUIZ_COUNT: usize = 10;

// AppState holding db + local API token
pub struct AppState {
    pub db: &'static Database,
    pub api_token: String,
}

fn map_llm_resolve_err(e: String) -> (StatusCode, String) {
    let code = if e.contains("MISSING_API_KEY") {
        StatusCode::BAD_REQUEST
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (code, e)
}

pub async fn start_research(
    State(state): State<&'static AppState>,
    Json(req): Json<AiStartResearchRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    let topic = truncate_chars(req.topic.trim(), MAX_TOPIC_CHARS);
    if topic.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "请输入要学习的主题".into()));
    }

    let llm_config = match blocking::run(move || repo::settings::resolve_llm_config(state.db, "chat")).await
    {
        Ok(cfg) => cfg,
        Err((_, e)) => return Err(map_llm_resolve_err(e)),
    };

    // Clone config for the second LLM call (provider holds the first)
    let extract_config = crate::ai::LlmConfig {
        kind: llm_config.kind,
        base_url: llm_config.base_url.clone(),
        api_key: llm_config.api_key.clone(),
        model: llm_config.model.clone(),
        temperature: llm_config.temperature,
        max_tokens: llm_config.max_tokens,
    };

    let drafts = crate::ai::extract::propose_sources(llm_config, &topic)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let source_reqs: Vec<crate::models::CreateSourceRequest> = drafts
        .into_iter()
        .map(|r| crate::models::CreateSourceRequest {
            title: r.title,
            source_type: "text".to_string(),
            content: r.description,
            tags: vec![],
            origin: "ai_search".to_string(),
            source_url: None,
        })
        .collect();

    let all_content: String = source_reqs
        .iter()
        .map(|s| format!("{}\n{}", s.title, s.content))
        .collect::<Vec<_>>()
        .join("\n---\n");

    let kp_reqs = crate::ai::extract::extract_knowledge_points(extract_config, &topic, &all_content)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let plan_req = CreateLearningPlanRequest {
        title: topic.clone(),
        goal: format!("Master the core concepts of {}", topic),
        kp_ids: vec![],
    };

    let result = blocking::run(move || {
        repo::learning::persist_research_bundle(state.db, &source_reqs, &kp_reqs, &plan_req)
    })
    .await?;

    Ok((StatusCode::OK, Json(serde_json::to_value(&result).unwrap())))
}

/// Generate quiz questions for given knowledge points
#[derive(serde::Deserialize)]
pub struct GenerateQuizRequest {
    pub kp_id: String,
    pub count: usize,
}

pub async fn generate_quiz(
    State(state): State<&'static AppState>,
    Json(req): Json<GenerateQuizRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    let count = req.count.clamp(1, MAX_QUIZ_COUNT);
    let kp_id = req.kp_id.clone();

    let (kp, llm_config) = blocking::run(move || {
        let kp = repo::knowledge::get_kp(state.db, &kp_id)?
            .ok_or_else(|| "KP not found".to_string())?;
        let llm_config = repo::settings::resolve_llm_config(state.db, "quiz_gen")?;
        Ok((kp, llm_config))
    })
    .await
    .map_err(|(code, e)| {
        if e.contains("not found") {
            (StatusCode::NOT_FOUND, e)
        } else if e.contains("MISSING_API_KEY") {
            (StatusCode::BAD_REQUEST, e)
        } else if e.contains("PROVIDER") || e.contains("未知厂商") {
            (StatusCode::SERVICE_UNAVAILABLE, e)
        } else {
            (code, e)
        }
    })?;

    let mut questions = crate::ai::quiz_gen::generate_quizzes(
        llm_config,
        &kp.title,
        &truncate_chars(&kp.content, MAX_PROMPT_CHARS),
        count,
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let public = blocking::run(move || {
        for q in &mut questions {
            q.kp_id = req.kp_id.clone();
            repo::quiz::create_question(state.db, q)?;
        }
        Ok(questions.iter().map(|q| q.to_public()).collect::<Vec<_>>())
    })
    .await?;

    Ok((StatusCode::CREATED, Json(serde_json::to_value(&public).unwrap())))
}

/// Update mastery record after quiz attempt
#[derive(serde::Deserialize)]
pub struct UpdateMasteryRequest {
    pub kp_id: String,
    pub is_correct: bool,
}

pub async fn update_mastery(
    State(state): State<&'static AppState>,
    Json(req): Json<UpdateMasteryRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let record = blocking::run(move || {
        let existing = repo::learning::get_mastery_by_kp(state.db, &req.kp_id)?;

        // SM-2 advances at most once per day per KP. A quiz session answers
        // several questions in a row and each answer calls update_mastery;
        // without this guard a single session would count as several reviews
        // and inflate the interval (0->1->6->16 days).
        if let Some(rec) = &existing {
            if !should_advance_sm2(rec.last_reviewed_at.as_deref(), chrono::Utc::now()) {
                return Ok(rec.clone());
            }
        }

        let record_id = existing
            .as_ref()
            .map(|r| r.id.clone())
            .unwrap_or_else(crate::models::new_id);

        let input = match existing {
            Some(rec) => crate::learning::sm2::Sm2Input {
                ease_factor: rec.ease_factor,
                interval_days: rec.interval_days,
                repetitions: rec.repetitions,
                is_correct: req.is_correct,
                response_quality: if req.is_correct { 4 } else { 1 },
            },
            None => crate::learning::sm2::Sm2Input {
                ease_factor: 2.5,
                interval_days: 0,
                repetitions: 0,
                is_correct: req.is_correct,
                response_quality: if req.is_correct { 4 } else { 1 },
            },
        };

        let output = crate::learning::sm2::calculate(input);
        let record = crate::models::MasteryRecord {
            id: record_id,
            kp_id: req.kp_id,
            ease_factor: output.ease_factor,
            interval_days: output.interval_days,
            repetitions: output.repetitions,
            next_review_at: output.next_review_at,
            last_reviewed_at: Some(chrono::Utc::now().to_rfc3339()),
        };
        repo::learning::upsert_mastery(state.db, &record)?;
        Ok(record)
    })
    .await?;

    Ok(Json(serde_json::to_value(&record).unwrap()))
}

/// SM-2 advances at most once per day per KP.
fn should_advance_sm2(
    last_reviewed_at: Option<&str>,
    now: chrono::DateTime<chrono::Utc>,
) -> bool {
    let Some(last) = last_reviewed_at
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
    else {
        return true;
    };
    let last = last.with_timezone(&chrono::Utc);
    last.date_naive() != now.date_naive()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sm2_advances_when_no_previous_review() {
        assert!(should_advance_sm2(None, chrono::Utc::now()));
    }

    #[test]
    fn sm2_does_not_advance_twice_same_day() {
        let now = chrono::Utc::now();
        let last = now.to_rfc3339();
        assert!(!should_advance_sm2(Some(&last), now));
    }

    #[test]
    fn sm2_advances_next_day() {
        let now = chrono::Utc::now();
        let yesterday = (now - chrono::Duration::days(1)).to_rfc3339();
        assert!(should_advance_sm2(Some(&yesterday), now));
    }
}
