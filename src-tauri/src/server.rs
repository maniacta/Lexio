use axum::{routing::get, Router};
use tower_http::cors::CorsLayer;
use crate::api::{sources, knowledge, quiz, learning, ai_routes};

/// Build the Axum application router.
pub fn app(state: &'static ai_routes::AppState) -> Router {
    Router::new()
        .route("/api/health", get(health_check))
        // Sources
        .route("/api/sources", get(sources::list_sources).post(sources::create_source))
        .route("/api/sources/{id}", get(sources::get_source))
        .route("/api/sources/{id}/hide", axum::routing::post(sources::toggle_hidden))
        // Knowledge Points
        .route("/api/knowledge", get(knowledge::list_kps).post(knowledge::create_kp))
        .route("/api/knowledge/{id}", get(knowledge::get_kp).delete(knowledge::delete_kp))
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
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn health_check() -> &'static str {
    "OK"
}
