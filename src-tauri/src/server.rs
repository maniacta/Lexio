use axum::{
    middleware,
    routing::get,
    Router,
};
use http::{header, Method};
use tower_http::cors::CorsLayer;
use crate::api::{
    auth, chat_routes, sources, knowledge, quiz, learning, ai_routes, settings, relation,
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
        .layer(middleware::from_fn_with_state(state, auth::require_token))
        .layer(cors())
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
