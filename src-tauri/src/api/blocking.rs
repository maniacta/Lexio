use axum::http::StatusCode;

/// Run blocking SQLite / CPU work off the tokio worker threads.
pub async fn run<T, F>(f: F) -> Result<T, (StatusCode, String)>
where
    F: FnOnce() -> Result<T, String> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("任务失败: {e}")))?
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))
}

pub async fn run_user<T, F>(f: F) -> Result<T, (StatusCode, String)>
where
    F: FnOnce() -> Result<T, String> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("任务失败: {e}")))?
        .map_err(|e| (StatusCode::BAD_REQUEST, e))
}
