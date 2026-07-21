use axum::{extract::{Path, State}, http::StatusCode, Json};
use crate::api::ai_routes::AppState;
use crate::models::SubmitQuizAnswerRequest;
use crate::repo;

pub async fn get_quiz_by_kp(
    State(state): State<&'static AppState>,
    Path(kp_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let questions = repo::quiz::get_questions_by_kp(state.db, &kp_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(serde_json::to_value(&questions).unwrap()))
}

pub async fn submit_answer(
    State(state): State<&'static AppState>,
    Json(req): Json<SubmitQuizAnswerRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let questions = repo::quiz::get_questions_by_ids(state.db, &[req.question_id.clone()])
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let question = questions.first()
        .ok_or((StatusCode::NOT_FOUND, "Question not found".to_string()))?;

    let is_correct = req.user_answer.trim().eq_ignore_ascii_case(question.answer.trim());
    let _attempt = repo::quiz::record_attempt(state.db, &req, is_correct)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let result = crate::models::QuizResult {
        question: question.clone(),
        user_answer: req.user_answer,
        is_correct,
        explanation: question.explanation.clone(),
    };

    Ok(Json(serde_json::to_value(&result).unwrap()))
}
