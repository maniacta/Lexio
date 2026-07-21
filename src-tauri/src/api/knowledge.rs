use axum::{extract::{Path, Query, State}, http::StatusCode, Json};
use serde::Deserialize;
use crate::db::Database;
use crate::models::CreateKnowledgePointRequest;
use crate::repo;

#[derive(Deserialize)]
pub struct ListKpsQuery {
    pub search: Option<String>,
    pub ids: Option<String>, // comma-separated
}

pub async fn create_kp(
    State(db): State<&'static Database>,
    Json(req): Json<CreateKnowledgePointRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    let kp = repo::knowledge::create_kp(db, &req)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok((StatusCode::CREATED, Json(serde_json::to_value(&kp).unwrap())))
}

pub async fn list_kps(
    State(db): State<&'static Database>,
    Query(params): Query<ListKpsQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let kps = if let Some(ref query) = params.search {
        repo::knowledge::search_kps(db, query)
    } else if let Some(ref ids_str) = params.ids {
        let ids: Vec<String> = ids_str.split(',').map(|s| s.trim().to_string()).collect();
        repo::knowledge::list_kps_by_ids(db, &ids)
    } else {
        repo::knowledge::list_kps(db)
    }.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(serde_json::to_value(&kps).unwrap()))
}

pub async fn get_kp(
    State(db): State<&'static Database>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let kp = repo::knowledge::get_kp(db, &id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
        .ok_or((StatusCode::NOT_FOUND, "Knowledge point not found".to_string()))?;
    Ok(Json(serde_json::to_value(&kp).unwrap()))
}

pub async fn delete_kp(
    State(db): State<&'static Database>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    repo::knowledge::delete_kp(db, &id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(StatusCode::NO_CONTENT)
}
