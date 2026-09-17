use axum::http::StatusCode;

use crate::error;

/// Run blocking SQLite / CPU work off the tokio worker threads.
///
/// A marked internal (raw `rusqlite`, lock or filesystem detail) is returned
/// with its marker intact so the audit middleware can record the detail and
/// replace it with a generic message; hand-written messages keep passing
/// through with the historical 500 status, because callers rely on them —
/// `submit_answer` remaps "not found" to 404 and `map_llm_resolve_err` reads
/// the `MISSING_API_KEY` prefix.
pub async fn run<T, F>(f: F) -> Result<T, (StatusCode, String)>
where
    F: FnOnce() -> Result<T, String> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(join_error)?
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))
}

/// Run blocking work whose error may be shown to the user as a 400.
///
/// Only messages written by hand in the repository layer (validation text and
/// the `*_ERROR:` codes the frontend maps to actionable hints) reach the
/// client as a 400. Anything marked with [`error::internal`] becomes a 500
/// carrying the marker, which the audit middleware turns into a generic
/// message after logging the detail.
pub async fn run_user<T, F>(f: F) -> Result<T, (StatusCode, String)>
where
    F: FnOnce() -> Result<T, String> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(join_error)?
        .map_err(|e| {
            if error::is_internal(&e) {
                (StatusCode::INTERNAL_SERVER_ERROR, e)
            } else {
                (StatusCode::BAD_REQUEST, e)
            }
        })
}

/// A panic in the blocking task is always an internal fault.
fn join_error(e: tokio::task::JoinError) -> (StatusCode, String) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        error::internal(format!("blocking task failed: {e}")),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The marker must survive `run` so the middleware can log the detail.
    #[tokio::test]
    async fn run_keeps_the_marker_for_the_middleware() {
        let err =
            run(|| Err::<(), _>(error::internal("near \"SELECT\": syntax error".to_string())))
                .await
                .unwrap_err();
        assert_eq!(err.0, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(error::is_internal(&err.1), "middleware needs the marker");
        assert_eq!(
            error::internal_detail(&err.1),
            "near \"SELECT\": syntax error"
        );
    }

    /// Callers remap these, so the message and status must survive.
    #[tokio::test]
    async fn run_preserves_hand_written_messages() {
        let err = run(|| Err::<(), _>("MISSING_API_KEY: 请填写 API Key".to_string()))
            .await
            .unwrap_err();
        assert_eq!(err.0, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(err.1, "MISSING_API_KEY: 请填写 API Key");

        let err = run(|| Err::<(), _>("Question not found".to_string()))
            .await
            .unwrap_err();
        assert!(err.1.contains("not found"), "404 remapping depends on this");
    }

    #[tokio::test]
    async fn run_user_passes_hand_written_messages_through_as_400() {
        let err = run_user(|| Err::<(), _>("MISSING_API_KEY: 请填写 API Key".to_string()))
            .await
            .unwrap_err();
        assert_eq!(err.0, StatusCode::BAD_REQUEST);
        assert_eq!(err.1, "MISSING_API_KEY: 请填写 API Key");
    }

    #[tokio::test]
    async fn run_user_marks_internals_as_500() {
        let err = run_user(|| Err::<(), _>(error::internal("database is locked".to_string())))
            .await
            .unwrap_err();
        assert_eq!(err.0, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(error::is_internal(&err.1));
    }
}
