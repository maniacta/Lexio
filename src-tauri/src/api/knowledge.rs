use axum::{extract::{Path, Query, State}, http::StatusCode, Json};
use serde::Deserialize;
use crate::api::ai_routes::AppState;
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
    let kp = repo::knowledge::create_kp(state.db, &req)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok((StatusCode::CREATED, Json(serde_json::to_value(&kp).unwrap())))
}

pub async fn list_kps(
    State(state): State<&'static AppState>,
    Query(params): Query<ListKpsQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let kps = if let Some(ref query) = params.search {
        repo::knowledge::search_kps(state.db, query)
    } else if let Some(ref ids_str) = params.ids {
        let ids: Vec<String> = ids_str.split(',').map(|s| s.trim().to_string()).collect();
        repo::knowledge::list_kps_by_ids(state.db, &ids)
    } else {
        repo::knowledge::list_kps(state.db)
    }.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(serde_json::to_value(&kps).unwrap()))
}

pub async fn get_kp(
    State(state): State<&'static AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let kp = repo::knowledge::get_kp(state.db, &id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
        .ok_or((StatusCode::NOT_FOUND, "Knowledge point not found".to_string()))?;
    Ok(Json(serde_json::to_value(&kp).unwrap()))
}

pub async fn delete_kp(
    State(state): State<&'static AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    repo::knowledge::delete_kp(state.db, &id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(StatusCode::NO_CONTENT)
}
