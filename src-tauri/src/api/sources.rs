use axum::{extract::{Path, Query, State}, http::StatusCode, Json};
use serde::Deserialize;
use crate::db::Database;
use crate::models::CreateSourceRequest;
use crate::repo;

#[derive(Deserialize)]
pub struct ListSourcesQuery {
    pub include_hidden: Option<bool>,
    pub search: Option<String>,
}

pub async fn create_source(
    State(db): State<&'static Database>,
    Json(req): Json<CreateSourceRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    let source = repo::source::create_source(db, &req)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok((StatusCode::CREATED, Json(serde_json::to_value(&source).unwrap())))
}

pub async fn list_sources(
    State(db): State<&'static Database>,
    Query(params): Query<ListSourcesQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let sources = if let Some(ref query) = params.search {
        repo::source::search_sources(db, query)
    } else {
        repo::source::list_sources(db, params.include_hidden.unwrap_or(false))
    }.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(serde_json::to_value(&sources).unwrap()))
}

pub async fn get_source(
    State(db): State<&'static Database>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let source = repo::source::get_source(db, &id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
        .ok_or((StatusCode::NOT_FOUND, "Source not found".to_string()))?;
    Ok(Json(serde_json::to_value(&source).unwrap()))
}

#[derive(Deserialize)]
pub struct ToggleHiddenRequest {
    pub hidden: bool,
}

pub async fn toggle_hidden(
    State(db): State<&'static Database>,
    Path(id): Path<String>,
    Json(req): Json<ToggleHiddenRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    repo::source::toggle_hidden(db, &id, req.hidden)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(StatusCode::OK)
}
