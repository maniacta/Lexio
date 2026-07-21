mod models;
mod server;
mod db;
mod repo;

use std::net::SocketAddr;
use std::path::PathBuf;
use db::Database;
use tauri::Manager;
use tokio::net::TcpListener;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn get_api_port(state: tauri::State<'_, ApiState>) -> u16 {
    state.port
}

struct ApiState {
    port: u16,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_handle = app.handle().clone();

            let app_dir: PathBuf = app_handle.path().app_data_dir().unwrap();
            std::fs::create_dir_all(&app_dir).unwrap();
            let db_path = app_dir.join("lexio.db");

            let db = Database::new(db_path.to_str().unwrap()).expect("Failed to open database");
            db.migrate().expect("Failed to run migrations");
            app_handle.manage(db);

            tauri::async_runtime::spawn(async move {
                let addr = SocketAddr::from(([127, 0, 0, 1], 3001));
                let listener = TcpListener::bind(addr).await.unwrap();
                let actual_port = listener.local_addr().unwrap().port();

                app_handle.manage(ApiState { port: actual_port });
                eprintln!("Lexio backend running on http://127.0.0.1:{actual_port}");

                axum::serve(listener, server::app()).await.unwrap();
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet, get_api_port])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
