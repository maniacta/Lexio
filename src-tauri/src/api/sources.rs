use axum::{extract::{Path, Query, State}, http::StatusCode, Json};
use serde::Deserialize;
use crate::api::ai_routes::AppState;
use crate::api::blocking;
use crate::models::CreateSourceRequest;
use crate::repo;

#[derive(Deserialize)]
pub struct ListSourcesQuery {
    pub include_hidden: Option<bool>,
    pub search: Option<String>,
}

pub async fn create_source(
    State(state): State<&'static AppState>,
    Json(req): Json<CreateSourceRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    let source = blocking::run(move || repo::source::create_source(state.db, &req)).await?;
    Ok((StatusCode::CREATED, Json(serde_json::to_value(&source).unwrap())))
}

pub async fn list_sources(
    State(state): State<&'static AppState>,
    Query(params): Query<ListSourcesQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let sources = blocking::run(move || {
        if let Some(ref query) = params.search {
            repo::source::search_sources(state.db, query)
        } else {
            repo::source::list_sources(state.db, params.include_hidden.unwrap_or(false))
        }
    })
    .await?;
    Ok(Json(serde_json::to_value(&sources).unwrap()))
}

pub async fn get_source(
    State(state): State<&'static AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let source = blocking::run(move || repo::source::get_source(state.db, &id))
        .await?
        .ok_or((StatusCode::NOT_FOUND, "Source not found".to_string()))?;
    Ok(Json(serde_json::to_value(&source).unwrap()))
}

#[derive(Deserialize)]
pub struct ToggleHiddenRequest {
    pub hidden: bool,
}

pub async fn toggle_hidden(
    State(state): State<&'static AppState>,
    Path(id): Path<String>,
    Json(req): Json<ToggleHiddenRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    blocking::run(move || repo::source::toggle_hidden(state.db, &id, req.hidden)).await?;
    Ok(StatusCode::OK)
}
