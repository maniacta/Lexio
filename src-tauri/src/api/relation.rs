use axum::{extract::{Path, State}, http::StatusCode, Json};
use crate::api::ai_routes::AppState;
use crate::api::blocking;
use crate::models::CreateRelationRequest;
use crate::repo;

const VALID_TYPES: &[&str] = &["prerequisite", "related", "extension"];

pub async fn list_for_kp(
    State(state): State<&'static AppState>,
    Path(kp_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let rels = blocking::run(move || repo::relation::get_relations_for_kp(state.db, &kp_id)).await?;
    Ok(Json(serde_json::to_value(&rels).unwrap()))
}

pub async fn create_for_kp(
    State(state): State<&'static AppState>,
    Path(from_kp_id): Path<String>,
    Json(req): Json<CreateRelationRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    if !VALID_TYPES.contains(&req.relation_type.as_str()) {
        return Err((
            StatusCode::BAD_REQUEST,
            "relation_type 须为 prerequisite / related / extension".into(),
        ));
    }
    if req.to_kp_id.trim().is_empty() || req.to_kp_id == from_kp_id {
        return Err((StatusCode::BAD_REQUEST, "无效的关联知识点".into()));
    }

    let rel = blocking::run_user(move || {
        // Ensure both KPs exist
        if repo::knowledge::get_kp(state.db, &from_kp_id)?.is_none() {
            return Err("源知识点不存在".into());
        }
        if repo::knowledge::get_kp(state.db, &req.to_kp_id)?.is_none() {
            return Err("目标知识点不存在".into());
        }
        repo::relation::create_relation(
            state.db,
            &from_kp_id,
            &req.to_kp_id,
            &req.relation_type,
        )
    })
    .await?;

    Ok((StatusCode::CREATED, Json(serde_json::to_value(&rel).unwrap())))
}

pub async fn delete_relation(
    State(state): State<&'static AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    blocking::run_user(move || repo::relation::delete_relation(state.db, &id)).await?;
    Ok(StatusCode::NO_CONTENT)
}
