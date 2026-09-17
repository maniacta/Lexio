use axum::{body::Body, middleware::{self, Next}, response::Response, routing::get, Router};
use http::{header, Method};
use tower_http::cors::CorsLayer;
use crate::api::{
    auth, chat_routes, logs, sources, knowledge, quiz, learning, ai_routes, settings, relation,
};

/// Build the Axum application router.
pub fn app(state: &'static ai_routes::AppState) -> Router {
    Router::new()
        .route("/api/health", get(health_check))
        .route("/api/auth/token", get(auth::bootstrap_token))
        // Sources
        .route("/api/sources", get(sources::list_sources).post(sources::create_source))
        .route("/api/sources/{id}", get(sources::get_source))
        .route("/api/sources/{id}/hide", axum::routing::post(sources::toggle_hidden))
        // Knowledge Points
        .route("/api/knowledge", get(knowledge::list_kps).post(knowledge::create_kp))
        .route("/api/knowledge/{id}", get(knowledge::get_kp).delete(knowledge::delete_kp))
        .route("/api/knowledge/{id}/relations",
            get(relation::list_for_kp).post(relation::create_for_kp))
        .route("/api/relations/{id}", axum::routing::delete(relation::delete_relation))
        // Quiz
        .route("/api/quiz/kp/{kp_id}", get(quiz::get_quiz_by_kp))
        .route("/api/quiz/submit", axum::routing::post(quiz::submit_answer))
        // Learning
        .route("/api/learning/plans", get(learning::list_plans).post(learning::create_plan))
        .route("/api/learning/reviews/due", get(learning::get_due_reviews))
        // AI
        .route("/api/ai/research", axum::routing::post(ai_routes::start_research))
        .route("/api/ai/generate-quiz", axum::routing::post(ai_routes::generate_quiz))
        .route("/api/ai/update-mastery", axum::routing::post(ai_routes::update_mastery))
        .route("/api/ai/chat", axum::routing::post(ai_routes::chat))
        // Chat sessions
        .route("/api/chat/sessions",
            get(chat_routes::list_sessions).post(chat_routes::create_session))
        .route("/api/chat/sessions/{id}", axum::routing::delete(chat_routes::delete_session))
        .route("/api/chat/sessions/{id}/messages", get(chat_routes::get_messages))
        .route("/api/chat/sessions/{id}/plan", axum::routing::post(chat_routes::set_session_plan))
        .route("/api/chat/messages", axum::routing::post(chat_routes::append_message))
        // Settings
        .route("/api/settings", get(settings::get_settings))
        .route("/api/settings/provider-kinds", get(settings::list_provider_kinds))
        .route("/api/settings/providers",
            get(settings::list_providers).post(settings::create_provider))
        .route("/api/settings/providers/{id}",
            axum::routing::put(settings::update_provider).delete(settings::delete_provider))
        .route("/api/settings/providers/{id}/models",
            axum::routing::post(settings::create_model))
        .route("/api/settings/providers/{provider_id}/models/{model_id}",
            axum::routing::put(settings::update_model).delete(settings::delete_model))
        .route(
            "/api/settings/providers/{provider_id}/models/{model_id}/default",
            axum::routing::post(settings::set_model_default),
        )
        .route("/api/settings/tasks", get(settings::get_task_models))
        .route("/api/settings/tasks/{task_name}", axum::routing::put(settings::set_task_model))
        .route("/api/settings/general", axum::routing::put(settings::update_general))
        .route("/api/settings/test-connection", axum::routing::post(settings::test_connection))
        // Logs
        .route("/api/logs/batch", axum::routing::post(logs::ingest_logs))
        .layer(cors())
        .layer(middleware::from_fn_with_state(state, auth::require_token))
        // Audit middleware last → outermost: runs before auth so rejected
        // requests (401) are also recorded.
        .layer(middleware::from_fn(audit_middleware))
        .with_state(state)
}

fn cors() -> CorsLayer {
    CorsLayer::new()
        .allow_origin([
            "http://localhost:14200".parse::<axum::http::HeaderValue>().unwrap(),
            "tauri://localhost".parse::<axum::http::HeaderValue>().unwrap(),
            "https://tauri.localhost".parse::<axum::http::HeaderValue>().unwrap(),
        ])
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([
            header::CONTENT_TYPE,
            header::HeaderName::from_static("x-lexio-token"),
        ])
}

async fn health_check() -> &'static str {
    "OK"
}

async fn audit_middleware(
    request: axum::http::Request<axum::body::Body>,
    next: Next,
) -> Response {
    let start = std::time::Instant::now();
    let method = request.method().clone();
    let uri = request.uri().clone();
    let path = uri.path().to_string();

    let mut response = next.run(request).await;

    let duration_ms = start.elapsed().as_millis() as i64;
    let status_code = response.status().as_u16() as i32;

    // Skip audit for the log ingestion endpoint itself (noise) and for
    // health checks (otherwise every probe writes a row).
    let skip_audit = path == "/api/logs/batch" || path == "/api/health";

    // Collect error bodies so details reach the audit log, and sanitize
    // internal SQL text (e.g. "no such column ... in SELECT") before it
    // is returned to the client.
    let mut error_message: Option<String> = None;
    if status_code >= 400 && path != "/api/logs/batch" {
        let (parts, body) = response.into_parts();
        let bytes = axum::body::to_bytes(body, 8 * 1024).await.unwrap_or_default();
        let text = String::from_utf8_lossy(&bytes).to_string();
        if status_code >= 500 && looks_like_internal_error(&text) {
            response = Response::from_parts(parts, Body::from("服务器内部错误，详情已记录到日志"));
        } else {
            response = Response::from_parts(parts, Body::from(bytes));
        }
        if !text.trim().is_empty() {
            error_message = Some(truncate(&text, 500));
        }
    }

    if !skip_audit {
        if let Some(msg) = &error_message {
            tracing::info!(
                target: "audit",
                source = "backend",
                category = "system",
                action = "http_request",
                method = %method.as_str(),
                path = %path,
                status_code = status_code,
                duration_ms = duration_ms,
                error_message = %msg,
            );
        } else {
            tracing::info!(
                target: "audit",
                source = "backend",
                category = "system",
                action = "http_request",
                method = %method.as_str(),
                path = %path,
                status_code = status_code,
                duration_ms = duration_ms,
            );
        }
    }

    response
}

fn looks_like_internal_error(text: &str) -> bool {
    let t = text.to_lowercase();
    t.contains("sqlite")
        || t.contains("no such column")
        || t.contains("constraint failed")
        || t.contains("foreign key")
        || t.contains("database disk image")
}

fn truncate(text: &str, max_chars: usize) -> String {
    if text.chars().count() > max_chars {
        let mut t: String = text.chars().take(max_chars).collect();
        t.push('…');
        t
    } else {
        text.to_string()
    }
}
