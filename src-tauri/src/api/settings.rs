use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use crate::api::ai_routes::AppState;
use crate::api::blocking;
use crate::models::{
    CreateProviderRequest, UpdateProviderRequest,
    CreateModelRequest, UpdateModelRequest,
    SetTaskModelRequest, GeneralSettings,
    TestConnectionRequest, TestConnectionResponse,
};
use crate::repo;
use serde_json::json;

pub async fn get_settings(
    State(state): State<&'static AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let value = blocking::run(move || {
        let general_entries = repo::settings::get_all_settings(state.db)?;
        let general: serde_json::Map<_, _> = general_entries
            .into_iter()
            .map(|(k, v)| (k, serde_json::Value::String(v)))
            .collect();

        let providers = repo::settings::list_providers(state.db)?;
        let tasks = repo::settings::get_task_models(state.db)?;

        let mut task_map = serde_json::Map::new();
        for t in &["chat", "quiz_gen"] {
            let mapping = tasks.iter().find(|m| m.task_name == *t);
            let resolved = if mapping.as_ref().and_then(|m| m.model_id.as_ref()).is_some() {
                let model_id = mapping.as_ref().unwrap().model_id.as_ref().unwrap();
                repo::settings::get_model_full(state.db, model_id)
                    .ok()
                    .flatten()
                    .map(|(p, m)| format!("{} / {}", p.name, m.model_name))
            } else {
                resolve_default_display(state.db).ok()
            };
            task_map.insert(
                t.to_string(),
                json!({
                    "model_id": mapping.and_then(|m| m.model_id.clone()),
                    "resolved": resolved,
                }),
            );
        }

        Ok(json!({
            "general": general,
            "providers": providers,
            "task_models": task_map,
        }))
    })
    .await?;

    Ok(Json(value))
}

fn resolve_default_display(db: &crate::db::Database) -> Result<String, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let (p_name, m_name): (String, String) = conn
        .query_row(
            "SELECT mp.name, pm.model_name FROM model_providers mp
         JOIN provider_models pm ON pm.provider_id = mp.id
         WHERE mp.is_default = 1 AND pm.is_default = 1
         LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| e.to_string())?;
    Ok(format!("{} / {}", p_name, m_name))
}

// ── Providers ──

pub async fn list_providers(
    State(state): State<&'static AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let providers = blocking::run(move || repo::settings::list_providers(state.db)).await?;
    Ok(Json(serde_json::to_value(&providers).unwrap()))
}

pub async fn create_provider(
    State(state): State<&'static AppState>,
    Json(req): Json<CreateProviderRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    let start = std::time::Instant::now();
    let p = blocking::run_user(move || repo::settings::create_provider(state.db, &req)).await?;
    let duration_ms = start.elapsed().as_millis() as i64;
    tracing::info!(
        target: "audit",
        source = "backend",
        category = "settings",
        action = "create_provider",
        status_code = 201,
        duration_ms = duration_ms,
    );
    Ok((StatusCode::CREATED, Json(serde_json::to_value(&p).unwrap())))
}

pub async fn update_provider(
    State(state): State<&'static AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateProviderRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let audit_id = id.clone();
    let start = std::time::Instant::now();
    blocking::run_user(move || repo::settings::update_provider(state.db, &id, &req)).await?;
    let duration_ms = start.elapsed().as_millis() as i64;
    tracing::info!(
        target: "audit",
        source = "backend",
        category = "settings",
        action = "update_provider",
        status_code = 200,
        duration_ms = duration_ms,
        params_summary = %serde_json::json!({"provider_id": audit_id}),
    );
    Ok(StatusCode::OK)
}

pub async fn delete_provider(
    State(state): State<&'static AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let audit_id = id.clone();
    let start = std::time::Instant::now();
    blocking::run_user(move || repo::settings::delete_provider(state.db, &id)).await?;
    let duration_ms = start.elapsed().as_millis() as i64;
    tracing::info!(
        target: "audit",
        source = "backend",
        category = "settings",
        action = "delete_provider",
        status_code = 204,
        duration_ms = duration_ms,
        params_summary = %serde_json::json!({"provider_id": audit_id}),
    );
    Ok(StatusCode::NO_CONTENT)
}

// ── Models ──

pub async fn create_model(
    State(state): State<&'static AppState>,
    Path(provider_id): Path<String>,
    Json(req): Json<CreateModelRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    let audit_provider = provider_id.clone();
    let start = std::time::Instant::now();
    let m = blocking::run_user(move || repo::settings::create_model(state.db, &provider_id, &req)).await?;
    let duration_ms = start.elapsed().as_millis() as i64;
    tracing::info!(
        target: "audit",
        source = "backend",
        category = "settings",
        action = "create_model",
        status_code = 201,
        duration_ms = duration_ms,
        params_summary = %serde_json::json!({"provider_id": audit_provider}),
    );
    Ok((StatusCode::CREATED, Json(serde_json::to_value(&m).unwrap())))
}

pub async fn update_model(
    State(state): State<&'static AppState>,
    Path((provider_id, model_id)): Path<(String, String)>,
    Json(req): Json<UpdateModelRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let audit_provider = provider_id.clone();
    let audit_model = model_id.clone();
    let start = std::time::Instant::now();
    blocking::run_user(move || repo::settings::update_model(state.db, &provider_id, &model_id, &req))
        .await?;
    let duration_ms = start.elapsed().as_millis() as i64;
    tracing::info!(
        target: "audit",
        source = "backend",
        category = "settings",
        action = "update_model",
        status_code = 200,
        duration_ms = duration_ms,
        params_summary = %serde_json::json!({"provider_id": audit_provider, "model_id": audit_model}),
    );
    Ok(StatusCode::OK)
}

pub async fn delete_model(
    State(state): State<&'static AppState>,
    Path((provider_id, model_id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let audit_provider = provider_id.clone();
    let audit_model = model_id.clone();
    let start = std::time::Instant::now();
    blocking::run_user(move || repo::settings::delete_model(state.db, &provider_id, &model_id))
        .await?;
    let duration_ms = start.elapsed().as_millis() as i64;
    tracing::info!(
        target: "audit",
        source = "backend",
        category = "settings",
        action = "delete_model",
        status_code = 204,
        duration_ms = duration_ms,
        params_summary = %serde_json::json!({"provider_id": audit_provider, "model_id": audit_model}),
    );
    Ok(StatusCode::NO_CONTENT)
}

pub async fn set_model_default(
    State(state): State<&'static AppState>,
    Path((provider_id, model_id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let audit_provider = provider_id.clone();
    let audit_model = model_id.clone();
    let start = std::time::Instant::now();
    blocking::run_user(move || {
        repo::settings::set_model_default(state.db, &provider_id, &model_id)
    })
    .await?;
    let duration_ms = start.elapsed().as_millis() as i64;
    tracing::info!(
        target: "audit",
        source = "backend",
        category = "settings",
        action = "set_model_default",
        status_code = 200,
        duration_ms = duration_ms,
        params_summary = %serde_json::json!({"provider_id": audit_provider, "model_id": audit_model}),
    );
    Ok(StatusCode::OK)
}

// ── Task Models ──

pub async fn get_task_models(
    State(state): State<&'static AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let tasks = blocking::run(move || repo::settings::get_task_models(state.db)).await?;
    Ok(Json(serde_json::to_value(&tasks).unwrap()))
}

pub async fn set_task_model(
    State(state): State<&'static AppState>,
    Path(task_name): Path<String>,
    Json(req): Json<SetTaskModelRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let valid_tasks = ["chat", "quiz_gen"];
    if !valid_tasks.contains(&task_name.as_str()) {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("Invalid task name: {}", task_name),
        ));
    }
    let audit_task = task_name.clone();
    let start = std::time::Instant::now();
    blocking::run_user(move || repo::settings::set_task_model(state.db, &task_name, &req)).await?;
    let duration_ms = start.elapsed().as_millis() as i64;
    tracing::info!(
        target: "audit",
        source = "backend",
        category = "settings",
        action = "set_task_model",
        status_code = 200,
        duration_ms = duration_ms,
        params_summary = %serde_json::json!({"task": audit_task}),
    );
    Ok(StatusCode::OK)
}

// ── General Settings ──

pub async fn update_general(
    State(state): State<&'static AppState>,
    Json(req): Json<GeneralSettings>,
) -> Result<StatusCode, (StatusCode, String)> {
    let start = std::time::Instant::now();
    blocking::run(move || {
        let mut entries = Vec::new();
        if let Some(v) = &req.theme {
            entries.push(("theme".to_string(), v.clone()));
        }
        if let Some(v) = &req.language {
            entries.push(("language".to_string(), v.clone()));
        }
        if let Some(v) = &req.data_path {
            entries.push(("data_path".to_string(), v.clone()));
        }
        if let Some(v) = req.search_enabled {
            entries.push(("search_enabled".to_string(), v.to_string()));
        }
        repo::settings::set_settings(state.db, &entries)
    })
    .await?;
    let duration_ms = start.elapsed().as_millis() as i64;
    tracing::info!(
        target: "audit",
        source = "backend",
        category = "settings",
        action = "update_general",
        status_code = 200,
        duration_ms = duration_ms,
    );
    Ok(StatusCode::OK)
}

// ── Test Connection ──

pub async fn test_connection(
    State(state): State<&'static AppState>,
    Json(req): Json<TestConnectionRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let config = blocking::run_user(move || {
        repo::settings::resolve_for_test(state.db, &req.provider_id, &req.model_name)
    })
    .await?;
    let client = crate::ai::create_provider(config).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    match client
        .chat("You are a helpful assistant.", "Say 'hello' in one word.")
        .await
    {
        Ok(resp) => Ok(Json(
            serde_json::to_value(&TestConnectionResponse {
                ok: true,
                message: format!("连接成功。回复: {}", resp),
            })
            .unwrap(),
        )),
        Err(e) => Ok(Json(
            serde_json::to_value(&TestConnectionResponse {
                ok: false,
                message: e,
            })
            .unwrap(),
        )),
    }
}

pub async fn list_provider_kinds() -> Json<serde_json::Value> {
    Json(serde_json::to_value(crate::ai::list_provider_kinds()).unwrap())
}
