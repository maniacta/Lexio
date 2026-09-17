use axum::{extract::{Path, Query, State}, http::StatusCode, Json};
use serde::Deserialize;
use crate::api::ai_routes::AppState;
use crate::api::blocking;
use crate::models::CreateKnowledgePointRequest;
use crate::repo;

#[derive(Deserialize)]
pub struct ListKpsQuery {
    pub search: Option<String>,
    pub ids: Option<String>, // comma-separated
}

pub async fn create_kp(
    State(state): State<&'static AppState>,
    Json(req): Json<CreateKnowledgePointRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    let audit_title = req.title.clone();
    let start = std::time::Instant::now();
    let kp = blocking::run(move || repo::knowledge::create_kp(state.db, &req)).await?;
    let duration_ms = start.elapsed().as_millis() as i64;
    tracing::info!(
        target: "audit",
        source = "backend",
        category = "knowledge",
        action = "create_kp",
        status_code = 201,
        duration_ms = duration_ms,
        params_summary = %serde_json::json!({"title": audit_title}),
    );
    Ok((StatusCode::CREATED, Json(serde_json::to_value(&kp).unwrap())))
}

pub async fn list_kps(
    State(state): State<&'static AppState>,
    Query(params): Query<ListKpsQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let kps = blocking::run(move || {
        if let Some(ref query) = params.search {
            repo::knowledge::search_kps(state.db, query)
        } else if let Some(ref ids_str) = params.ids {
            let ids: Vec<String> = ids_str.split(',').map(|s| s.trim().to_string()).collect();
            repo::knowledge::list_kps_by_ids(state.db, &ids)
        } else {
            repo::knowledge::list_kps(state.db)
        }
    })
    .await?;
    Ok(Json(serde_json::to_value(&kps).unwrap()))
}

pub async fn get_kp(
    State(state): State<&'static AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let kp = blocking::run(move || repo::knowledge::get_kp(state.db, &id))
        .await?
        .ok_or((StatusCode::NOT_FOUND, "Knowledge point not found".to_string()))?;
    Ok(Json(serde_json::to_value(&kp).unwrap()))
}

pub async fn delete_kp(
    State(state): State<&'static AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let audit_id = id.clone();
    let start = std::time::Instant::now();
    blocking::run(move || repo::knowledge::delete_kp(state.db, &id)).await?;
    let duration_ms = start.elapsed().as_millis() as i64;
    tracing::info!(
        target: "audit",
        source = "backend",
        category = "knowledge",
        action = "delete_kp",
        status_code = 204,
        duration_ms = duration_ms,
        params_summary = %serde_json::json!({"kp_id": audit_id}),
    );
    Ok(StatusCode::NO_CONTENT)
}
