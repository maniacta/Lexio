// Standalone Axum server for web-only mode (no Tauri/GTK required)
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::net::TcpListener;

fn main() {
    // Use current directory for DB in web mode
    let db_path = PathBuf::from("lexio.db");
    let db_path_str = db_path.to_str().unwrap().to_string();
    println!("Lexio web mode - DB at {}", db_path.display());

    lexio_lib::crypto::init_master_key(&db_path_str).expect("Failed to init master key");

    let db = lexio_lib::db::Database::new(&db_path_str).expect("Failed to open database");
    db.migrate().expect("Failed to run migrations");
    let db: &'static lexio_lib::db::Database = Box::leak(Box::new(db));

    lexio_lib::repo::settings::init_presets(db).expect("Failed to initialize settings presets");
    lexio_lib::repo::settings::migrate_deepseek_models(db)
        .expect("Failed to migrate DeepSeek models");
    lexio_lib::repo::settings::migrate_encrypt_api_keys(db).expect("Failed to encrypt API keys");

    let api_token = lexio_lib::crypto::generate_api_token();
    let app_state: &'static lexio_lib::api::ai_routes::AppState =
        Box::leak(Box::new(lexio_lib::api::ai_routes::AppState {
            db,
            api_token: api_token.clone(),
        }));

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let addr = SocketAddr::from(([127, 0, 0, 1], 3001));
        let listener = TcpListener::bind(addr).await.unwrap();
        println!("Lexio backend running on http://127.0.0.1:3001");
        println!("Local API token ready (fetch /api/auth/token from loopback)");
        axum::serve(
            listener,
            lexio_lib::server::app(app_state).into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });
}
