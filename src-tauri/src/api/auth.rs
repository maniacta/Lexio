use axum::{
    extract::{ConnectInfo, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::net::SocketAddr;

use crate::api::ai_routes::AppState;

/// Require `X-Lexio-Token` on all routes except health + localhost token bootstrap.
pub async fn require_token(
    State(state): State<&'static AppState>,
    req: Request,
    next: Next,
) -> Response {
    let path = req.uri().path();
    if path == "/api/health" || path == "/api/auth/token" {
        return next.run(req).await;
    }

    let provided = req
        .headers()
        .get("x-lexio-token")
        .and_then(|v| v.to_str().ok());

    if provided == Some(state.api_token.as_str()) {
        next.run(req).await
    } else {
        (StatusCode::UNAUTHORIZED, "UNAUTHORIZED: 缺少或无效的 API Token").into_response()
    }
}

/// Loopback-only bootstrap so the web UI can obtain the session token.
pub async fn bootstrap_token(
    State(state): State<&'static AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if !addr.ip().is_loopback() {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(Json(json!({ "token": state.api_token })))
}
