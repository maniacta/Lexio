// Standalone Axum server for web-only mode (no Tauri/GTK required)
use std::net::SocketAddr;
use std::path::PathBuf;

fn main() {
    // Use current directory for DB in web mode
    let db_path = PathBuf::from("lexio.db");
    let db_path_str = match lexio_lib::startup::utf8_path(&db_path, "数据库路径") {
        Ok(s) => s,
        Err(msg) => lexio_lib::startup::exit(msg),
    };
    println!("Lexio web mode - DB at {}", db_path.display());

    if let Err(e) = lexio_lib::crypto::init_master_key(&db_path_str) {
        lexio_lib::startup::exit(format!("无法初始化主密钥：{e}"));
    }

    let db = match lexio_lib::db::Database::new(&db_path_str) {
        Ok(db) => db,
        Err(e) => lexio_lib::startup::exit(format!("无法打开数据库：{e}")),
    };
    if let Err(e) = db.migrate() {
        lexio_lib::startup::exit(format!("无法运行数据库迁移：{e}"));
    }
    let db: &'static lexio_lib::db::Database = Box::leak(Box::new(db));

    if let Err(e) = lexio_lib::repo::settings::init_presets(db) {
        lexio_lib::startup::exit(format!("无法初始化设置：{e}"));
    }
    if let Err(e) = lexio_lib::repo::settings::migrate_deepseek_models(db) {
        lexio_lib::startup::exit(format!("无法迁移模型设置：{e}"));
    }
    if let Err(e) = lexio_lib::repo::settings::migrate_encrypt_api_keys(db) {
        lexio_lib::startup::exit(format!("无法加密已保存的 API Key：{e}"));
    }

    let api_token = match lexio_lib::crypto::generate_api_token() {
        Ok(token) => token,
        Err(e) => lexio_lib::startup::exit(format!("无法生成 API Token：{e}")),
    };
    let app_state: &'static lexio_lib::api::ai_routes::AppState =
        Box::leak(Box::new(lexio_lib::api::ai_routes::AppState {
            db,
            api_token: api_token.clone(),
        }));

    let rt = tokio::runtime::Runtime::new()
        .unwrap_or_else(|e| lexio_lib::startup::exit(format!("无法启动运行时：{e}")));
    rt.block_on(async {
        // Logging (file logs + audit DB layer) — same as Tauri mode. Must run
        // inside the tokio runtime: AuditDbLayer spawns a background task.
        let logs_dir = PathBuf::from("logs");
        if let Err(msg) = lexio_lib::init_logging(db, &logs_dir) {
            lexio_lib::startup::exit(msg);
        }
        // The key was loaded before logging existed; report its provenance now.
        lexio_lib::crypto::log_master_key_provenance();
        lexio_lib::repo::audit::prune(db, lexio_lib::AUDIT_LOG_RETENTION_DAYS);

        let (listener, port) =
            match lexio_lib::listen::bind_loopback(lexio_lib::listen::API_PORT).await {
                Ok(bound) => bound,
                Err(msg) => lexio_lib::startup::exit(msg),
            };
        println!("Lexio backend running on http://127.0.0.1:{port}");
        println!("Local API token ready (fetch /api/auth/token from loopback)");
        if let Err(e) = axum::serve(
            listener,
            lexio_lib::server::app(app_state).into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        {
            lexio_lib::startup::exit(format!("Lexio 本地 API 服务异常退出：{e}"));
        }
    });
}
