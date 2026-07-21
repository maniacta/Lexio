use axum::{routing::get, Router};
use tower_http::cors::CorsLayer;
use crate::api::{sources, knowledge};

/// Build the Axum application router.
pub fn app(db: &'static crate::db::Database) -> Router {
    Router::new()
        .route("/api/health", get(health_check))
        .route("/api/sources", get(sources::list_sources).post(sources::create_source))
        .route("/api/sources/{id}", get(sources::get_source))
        .route("/api/sources/{id}/hide", axum::routing::post(sources::toggle_hidden))
        .route("/api/knowledge", get(knowledge::list_kps).post(knowledge::create_kp))
        .route("/api/knowledge/{id}", get(knowledge::get_kp).delete(knowledge::delete_kp))
        .layer(CorsLayer::permissive())
        .with_state(db)
}

async fn health_check() -> &'static str {
    "OK"
}
