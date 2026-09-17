pub mod api;
pub mod models;
pub mod server;
pub mod db;
pub mod repo;
pub mod learning;
pub mod ai;
pub mod crypto;
pub mod error;
pub mod key_store;
pub mod limits;
pub mod listen;
pub mod tracing_layer;

use std::net::SocketAddr;
use std::path::PathBuf;
use db::Database;
use tauri::Manager;
use tokio::net::TcpListener;
use tracing_subscriber::prelude::*;

/// Retention window (days) for rotated log files. tracing-appender rotates
/// files daily but never deletes them, so cleanup is done at startup.
pub const FILE_LOG_RETENTION_DAYS: i64 = 7;
/// Retention window (days) for audit_logs rows.
pub const AUDIT_LOG_RETENTION_DAYS: i64 = 90;

/// Delete rotated log files older than `retention_days`.
pub fn cleanup_old_log_files(logs_dir: &std::path::Path, retention_days: i64) {
    if retention_days <= 0 {
        return;
    }
    let cutoff = std::time::SystemTime::now()
        - std::time::Duration::from_secs((retention_days * 86_400) as u64);
    if let Ok(entries) = std::fs::read_dir(logs_dir) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if let Ok(modified) = meta.modified() {
                    if modified < cutoff {
                        let _ = std::fs::remove_file(entry.path());
                    }
                }
            }
        }
    }
}

/// Initialize the tracing subscriber (file logs + audit DB layer) and apply
/// retention cleanup. Idempotent for process lifetime; call once at startup.
pub fn init_logging(db: &'static Database, logs_dir: &std::path::Path) {
    std::fs::create_dir_all(logs_dir).unwrap();
    cleanup_old_log_files(logs_dir, FILE_LOG_RETENTION_DAYS);

    let file_appender =
        tracing_appender::rolling::daily(logs_dir, "lexio.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    // Keep the guard alive for the process lifetime.
    Box::leak(Box::new(guard));

    let log_level =
        std::env::var("LEXIO_LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
    let env_filter = tracing_subscriber::EnvFilter::try_new(&log_level)
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    let audit_layer = crate::tracing_layer::AuditDbLayer::new(db);
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_target(true);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(audit_layer)
        .with(file_layer)
        .init();
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn get_api_port(state: tauri::State<'_, ApiState>) -> u16 {
    state.port
}

#[tauri::command]
fn get_api_token(state: tauri::State<'_, ApiState>) -> String {
    state.token.clone()
}

struct ApiState {
    port: u16,
    token: String,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.handle().clone();

            let app_dir: PathBuf = app_handle.path().app_data_dir().unwrap();
            std::fs::create_dir_all(&app_dir).unwrap();
            let db_path = app_dir.join("lexio.db");
            let db_path_str = db_path.to_str().unwrap().to_string();

            crypto::init_master_key(&db_path_str).expect("Failed to init master key");

            let db = Database::new(&db_path_str).expect("Failed to open database");
            db.migrate().expect("Failed to run migrations");
            let db: &'static db::Database = Box::leak(Box::new(db));
            app_handle.manage(db);

            // Bounded retention for audit rows (timestamps are RFC3339).
            repo::audit::prune(db, AUDIT_LOG_RETENTION_DAYS);

            // Initialize settings presets (idempotent)
            repo::settings::init_presets(db)
                .expect("Failed to initialize settings presets");
            repo::settings::migrate_deepseek_models(db)
                .expect("Failed to migrate DeepSeek models");
            repo::settings::migrate_encrypt_api_keys(db)
                .expect("Failed to encrypt API keys");

            // ── Initialize tracing subscriber ──
            let logs_dir = app_dir.join("logs");
            crate::init_logging(db, &logs_dir);
            // The key was loaded before logging existed; report its provenance now.
            crypto::log_master_key_provenance();
            tracing::info!(target: "audit", source = "backend", category = "system", action = "startup", user_action = "应用启动");

            let api_token = crypto::generate_api_token();
            let (std_listener, port) = crate::listen::bind_loopback_std(crate::listen::API_PORT)
                .map_err(|msg| {
                    eprintln!("{msg}");
                    msg
                })?;
            app_handle.manage(ApiState {
                port,
                token: api_token.clone(),
            });

            let app_state: &'static api::ai_routes::AppState =
                Box::leak(Box::new(api::ai_routes::AppState {
                    db,
                    api_token,
                }));

            tauri::async_runtime::spawn(async move {
                let listener = match TcpListener::from_std(std_listener) {
                    Ok(listener) => listener,
                    Err(e) => {
                        eprintln!("无法启动本地 API：{e}");
                        return;
                    }
                };
                eprintln!("Lexio backend running on http://127.0.0.1:{port}");

                if let Err(e) = axum::serve(
                    listener,
                    server::app(app_state).into_make_service_with_connect_info::<SocketAddr>(),
                )
                .await
                {
                    eprintln!("Lexio 本地 API 服务异常退出：{e}");
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet, get_api_port, get_api_token])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
