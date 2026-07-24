// Standalone Axum server for web-only mode (no Tauri/GTK required)
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::net::TcpListener;

fn main() {
    // Use current directory for DB in web mode
    let db_path = PathBuf::from("lexio.db");
    println!("Lexio web mode - DB at {}", db_path.display());

    let db = lexio_lib::db::Database::new(db_path.to_str().unwrap())
        .expect("Failed to open database");
    db.migrate().expect("Failed to run migrations");
    let db: &'static lexio_lib::db::Database = Box::leak(Box::new(db));

    let llm_config = lexio_lib::ai::llm::LlmConfig {
        base_url: std::env::var("LLM_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1".into()),
        api_key: std::env::var("LLM_API_KEY").unwrap_or_default(),
        model: std::env::var("LLM_MODEL")
            .unwrap_or_else(|_| "gpt-4o-mini".into()),
        temperature: 0.7,
        max_tokens: 4096,
        api_format: "openai_compatible".to_string(),
    };
    let llm = lexio_lib::ai::llm::LlmClient::new(llm_config);

    let app_state: &'static lexio_lib::api::ai_routes::AppState =
        Box::leak(Box::new(lexio_lib::api::ai_routes::AppState { db, llm }));

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let addr = SocketAddr::from(([127, 0, 0, 1], 3001));
        let listener = TcpListener::bind(addr).await.unwrap();
        println!("Lexio backend running on http://127.0.0.1:3001");
        axum::serve(listener, lexio_lib::server::app(app_state))
            .await
            .unwrap();
    });
}
