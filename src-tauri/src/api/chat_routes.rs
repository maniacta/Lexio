use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use crate::api::ai_routes::AppState;
use crate::api::blocking;
use crate::repo;

#[derive(Deserialize)]
pub struct CreateSessionRequest {
    pub title: Option<String>,
}

#[derive(Deserialize)]
pub struct AppendMessageRequest {
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub actions: Option<String>,
    pub context: Option<String>,
}

#[derive(Deserialize)]
pub struct SetPlanRequest {
    pub plan_id: String,
}

pub async fn list_sessions(
    State(state): State<&'static AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let sessions = blocking::run(move || repo::chat::list_sessions(state.db)).await?;
    Ok(Json(serde_json::to_value(&sessions).unwrap()))
}

pub async fn create_session(
    State(state): State<&'static AppState>,
    Json(req): Json<CreateSessionRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    let title = req.title.unwrap_or_else(|| "新对话".to_string());
    let session = blocking::run(move || repo::chat::create_session(state.db, &title)).await?;
    Ok((StatusCode::CREATED, Json(serde_json::to_value(&session).unwrap())))
}

pub async fn get_messages(
    State(state): State<&'static AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let messages = blocking::run(move || repo::chat::get_messages(state.db, &id)).await?;
    Ok(Json(serde_json::to_value(&messages).unwrap()))
}

pub async fn append_message(
    State(state): State<&'static AppState>,
    Json(req): Json<AppendMessageRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    let message = blocking::run(move || {
        repo::chat::append_message(
            state.db,
            &req.session_id,
            &req.role,
            &req.content,
            req.actions.as_deref(),
            req.context.as_deref(),
        )
    })
    .await?;
    Ok((StatusCode::CREATED, Json(serde_json::to_value(&message).unwrap())))
}

pub async fn delete_session(
    State(state): State<&'static AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    blocking::run(move || repo::chat::delete_session(state.db, &id)).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn set_session_plan(
    State(state): State<&'static AppState>,
    Path(id): Path<String>,
    Json(req): Json<SetPlanRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    blocking::run(move || repo::chat::set_session_plan(state.db, &id, &req.plan_id)).await?;
    Ok(StatusCode::OK)
}
