use axum::{extract::State, http::StatusCode, Json};
use crate::api::ai_routes::AppState;
use crate::models::CreateLearningPlanRequest;
use crate::repo;

pub async fn create_plan(
    State(state): State<&'static AppState>,
    Json(req): Json<CreateLearningPlanRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    let plan = repo::learning::create_plan(state.db, &req)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok((StatusCode::CREATED, Json(serde_json::to_value(&plan).unwrap())))
}

pub async fn list_plans(
    State(state): State<&'static AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let plans = repo::learning::list_plans(state.db)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(serde_json::to_value(&plans).unwrap()))
}

pub async fn get_due_reviews(
    State(state): State<&'static AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let records = repo::learning::get_due_reviews(state.db)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(serde_json::to_value(&records).unwrap()))
}
