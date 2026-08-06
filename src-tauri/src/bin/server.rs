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

    lexio_lib::repo::settings::init_presets(db)
        .expect("Failed to initialize settings presets");
    lexio_lib::repo::settings::migrate_deepseek_models(db)
        .expect("Failed to migrate DeepSeek models");

    let app_state: &'static lexio_lib::api::ai_routes::AppState =
        Box::leak(Box::new(lexio_lib::api::ai_routes::AppState { db }));

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
