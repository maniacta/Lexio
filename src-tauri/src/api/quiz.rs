use axum::{extract::{Path, State}, http::StatusCode, Json};
use crate::api::ai_routes::AppState;
use crate::api::blocking;
use crate::models::SubmitQuizAnswerRequest;
use crate::repo;

pub async fn get_quiz_by_kp(
    State(state): State<&'static AppState>,
    Path(kp_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let public = blocking::run(move || {
        let questions = repo::quiz::get_questions_by_kp(state.db, &kp_id)?;
        Ok(questions.iter().map(|q| q.to_public()).collect::<Vec<_>>())
    })
    .await?;
    Ok(Json(serde_json::to_value(&public).unwrap()))
}

pub async fn submit_answer(
    State(state): State<&'static AppState>,
    Json(req): Json<SubmitQuizAnswerRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let start = std::time::Instant::now();
    let result = blocking::run(move || {
        let questions = repo::quiz::get_questions_by_ids(state.db, &[req.question_id.clone()])?;
        let question = questions
            .first()
            .ok_or_else(|| "Question not found".to_string())?;
        let is_correct = repo::quiz::answers_match(&req.user_answer, &question.answer);
        let _attempt = repo::quiz::record_attempt(state.db, &req, is_correct)?;
        Ok(crate::models::QuizResult {
            question: question.to_public(),
            user_answer: req.user_answer,
            is_correct,
            explanation: question.explanation.clone(),
            correct_answer: question.answer.clone(),
        })
    })
    .await
    .map_err(|(code, e)| {
        if e.contains("not found") {
            (StatusCode::NOT_FOUND, e)
        } else {
            (code, e)
        }
    })?;
    let duration_ms = start.elapsed().as_millis() as i64;
    tracing::info!(
        target: "audit",
        source = "backend",
        category = "quiz",
        action = "submit_answer",
        status_code = 200,
        duration_ms = duration_ms,
        params_summary = %serde_json::json!({"is_correct": result.is_correct}),
    );

    Ok(Json(serde_json::to_value(&result).unwrap()))
}
