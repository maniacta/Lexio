use axum::{routing::get, Router};
use tower_http::cors::CorsLayer;

/// Build the Axum application router.
pub fn app() -> Router {
    Router::new()
        .route("/api/health", get(health_check))
        .layer(CorsLayer::permissive())
}

async fn health_check() -> &'static str {
    "OK"
}
