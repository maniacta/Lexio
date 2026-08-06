use axum::{extract::State, http::StatusCode, Json};
use crate::ai::{create_provider, extract_json_payload, truncate_chars};
use crate::api::blocking;
use crate::db::Database;
use crate::models::{AiStartResearchRequest, CreateKnowledgePointRequest, CreateLearningPlanRequest};
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
    let llm = create_provider(llm_config).map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    let search_prompt = truncate_chars(
        &format!(
            "用户想学习主题：{}。\n\
请列出 3 份高质量学习资料。\n\
返回 JSON 对象，格式严格为：{{\"items\":[{{\"title\":\"...\",\"description\":\"...\"}}]}}",
            topic
        ),
        MAX_PROMPT_CHARS,
    );
    let response = llm
        .chat_json(
            "You are a research assistant. Always reply with a valid JSON object only.",
            &search_prompt,
        )
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    #[derive(serde::Deserialize)]
    struct SearchResult {
        title: String,
        description: String,
    }
    #[derive(serde::Deserialize)]
    struct SearchEnvelope {
        items: Vec<SearchResult>,
    }

    let json_str = extract_json_payload(&response);
    let results: Vec<SearchResult> =
        if let Ok(env) = serde_json::from_str::<SearchEnvelope>(json_str) {
            env.items
        } else {
            serde_json::from_str(json_str).map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!(
                        "Parse error: {} | {}",
                        e,
                        json_str.chars().take(200).collect::<String>()
                    ),
                )
            })?
        };

    let source_reqs: Vec<crate::models::CreateSourceRequest> = results
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

    let kp_prompt = truncate_chars(
        &format!(
            "从以下关于「{}」的内容中提取主要知识点。\n\n{}\n\n\
返回 JSON 对象，格式严格为：\
{{\"items\":[{{\"title\":\"...\",\"summary\":\"一句话\",\"content\":\"2-3段详细说明\",\"tags\":[\"...\"]}}]}}",
            topic, all_content
        ),
        MAX_PROMPT_CHARS,
    );
    let kp_response = llm
        .chat_json(
            "You are a knowledge extraction assistant. Always reply with a valid JSON object only.",
            &kp_prompt,
        )
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    #[derive(serde::Deserialize)]
    struct KpEnvelope {
        items: Vec<CreateKnowledgePointRequest>,
    }

    let kp_json = extract_json_payload(&kp_response);
    let kp_reqs: Vec<CreateKnowledgePointRequest> =
        if let Ok(env) = serde_json::from_str::<KpEnvelope>(kp_json) {
            env.items
        } else {
            serde_json::from_str(kp_json).map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!(
                        "Parse KPs: {} | {}",
                        e,
                        kp_json.chars().take(200).collect::<String>()
                    ),
                )
            })?
        };

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
