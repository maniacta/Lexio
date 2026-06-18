mod server;

use std::net::SocketAddr;
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
