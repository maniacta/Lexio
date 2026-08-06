use axum::{extract::{Query, State}, http::StatusCode, Json};
use crate::api::ai_routes::AppState;
use crate::api::blocking;
use crate::models::CreateLearningPlanRequest;
use crate::repo;
use std::collections::HashMap;

pub async fn create_plan(
    State(state): State<&'static AppState>,
    Json(req): Json<CreateLearningPlanRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    let plan = blocking::run(move || repo::learning::create_plan(state.db, &req)).await?;
    Ok((StatusCode::CREATED, Json(serde_json::to_value(&plan).unwrap())))
}

pub async fn list_plans(
    State(state): State<&'static AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let plans = blocking::run(move || repo::learning::list_plans(state.db)).await?;
    Ok(Json(serde_json::to_value(&plans).unwrap()))
}

pub async fn get_due_reviews(
    State(state): State<&'static AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let with_kp = params.get("with_kp").map(|v| v == "true").unwrap_or(false);
    if with_kp {
        let items = blocking::run(move || repo::learning::get_due_reviews_with_kp(state.db)).await?;
        Ok(Json(serde_json::json!(items)))
    } else {
        let records = blocking::run(move || repo::learning::get_due_reviews(state.db)).await?;
        Ok(Json(serde_json::to_value(&records).unwrap()))
    }
}
