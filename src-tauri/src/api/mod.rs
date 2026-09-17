pub mod sources;
pub mod knowledge;
pub mod quiz;
pub mod learning;
pub mod ai_routes;
pub mod settings;
pub mod blocking;
pub mod auth;
pub mod relation;
pub mod chat_routes;
pub mod logs;
pub mod audit;

use axum::http::StatusCode;

/// Turn a hand-written validation message into a 400 a handler can return with
/// `?`. Used with the checks in [`crate::limits`]; the message is user-facing and
/// is passed through verbatim.
pub fn bad_request(err: String) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, err)
}
