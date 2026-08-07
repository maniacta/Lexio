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

// ── Chat ──

#[derive(serde::Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessageItem>,
    pub context: Option<ChatContext>,
}

#[derive(serde::Deserialize)]
pub struct ChatMessageItem {
    pub role: String,
    pub content: String,
}

#[derive(serde::Deserialize)]
pub struct ChatContext {
    pub plan_id: Option<String>,
    pub current_kp_id: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ChatAction {
    #[serde(rename = "type")]
    pub action_type: String,
    pub label: String,
    pub payload: serde_json::Value,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ChatResponse {
    pub content: String,
    pub actions: Vec<ChatAction>,
}

pub async fn chat(
    State(state): State<&'static AppState>,
    Json(req): Json<ChatRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    use crate::repo::{knowledge, learning};

    let llm_config = match blocking::run(move || repo::settings::resolve_llm_config(state.db, "chat")).await
    {
        Ok(v) => v,
        Err((_, e)) => return Err(map_llm_resolve_err(e)),
    };
    let llm = crate::ai::create_provider(llm_config)
        .map_err(|e| (StatusCode::SERVICE_UNAVAILABLE, e))?;

    // Build system prompt with context
    let mut system_prompt = String::from(
        "你是 Lexio 学习教练。用中文回复。"
    );

    if let Some(ref ctx) = req.context {
        if let Some(ref plan_id) = ctx.plan_id {
            let plan_id = plan_id.clone();
            if let Ok(Some(plan)) = blocking::run(move || learning::get_plan(state.db, &plan_id)).await {
                system_prompt.push_str(&format!("\n当前学习计划：{}（{}）", plan.title, plan.goal));
                if !plan.kp_ids.is_empty() {
                    let kp_ids = plan.kp_ids.clone();
                    if let Ok(kps) = blocking::run(move || knowledge::list_kps_by_ids(state.db, &kp_ids)).await {
                        system_prompt.push_str("\n知识点列表：");
                        for kp in &kps {
                            system_prompt.push_str(&format!("\n- {} (id={}): {}", kp.title, kp.id, kp.summary));
                        }
                    }
                }
            }
        }
        if let Some(ref kp_id) = ctx.current_kp_id {
            let kp_id = kp_id.clone();
            if let Ok(Some(kp)) = blocking::run(move || knowledge::get_kp(state.db, &kp_id)).await {
                system_prompt.push_str(&format!("\n正在学习：{}\n{}", kp.title, kp.content));
            }
        }
    }

    system_prompt.push_str(&concat!(
        "\n\n行为指南：",
        "\n- 用户说\"开始学习/学XX/教我\"时，引导选择知识点（返回 navigate_learning action，kpId 用上面列出的 id）",
        "\n- 用户说\"出题/测验\"时，建议进入测验模式（返回 start_quiz action）",
        "\n- 用户问概念问题时，用已有知识点内容回答",
        "\n- 如果没有对应知识点，建议用户\"先研究这个主题\"",
        "\n- 用户表达想学新主题（如\"我想学X\"、\"帮我研究X\"）时，返回 start_research action，payload 带 topic（用户想研究的内容原文）",
        "\n- 始终返回 JSON，不要包含 markdown 代码块标记",
        "\n\n返回格式（严格 JSON，一行写完不要换行）：",
        "\n{\"content\":\"你的markdown回复\",\"actions\":[{\"type\":\"navigate_learning\",\"label\":\"进入学习\",\"payload\":{\"kpId\":\"知识点id\",\"kpTitle\":\"知识点名称\"}}]}",
        "\naction type 只能是 navigate_learning / start_quiz / view_source / start_research，不需要 action 时 actions 为空数组 []",
    ));

    // Build user prompt from message history
    let user_prompt = req.messages.iter()
        .map(|m| format!("{}: {}", m.role, m.content))
        .collect::<Vec<_>>()
        .join("\n");

    let response = llm.chat(&system_prompt, &user_prompt).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    // Parse JSON — fallback to raw text if parsing fails
    let json_str = crate::ai::extract_json_payload(&response);
    let chat_resp: ChatResponse = serde_json::from_str(json_str)
        .unwrap_or_else(|_| ChatResponse {
            content: response,
            actions: vec![],
        });

    Ok((StatusCode::OK, Json(serde_json::to_value(&chat_resp).unwrap())))
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
