use axum::{extract::State, http::StatusCode, Json};
use serde::Deserialize;
use crate::api::ai_routes::AppState;
use crate::repo::audit::{self, AuditRecord};
use crate::models::new_id;

/// Maximum number of entries accepted in a single batch.
const MAX_BATCH_SIZE: usize = 500;
/// Maximum length (in chars) for summary text fields, per the logging spec.
const MAX_SUMMARY_LEN: usize = 1000;
/// Maximum length (in chars) for category/action identifiers.
const MAX_NAME_LEN: usize = 200;

#[derive(Deserialize)]
pub struct BatchLogRequest {
    pub logs: Vec<FrontendLogEntry>,
}

#[derive(Deserialize)]
pub struct FrontendLogEntry {
    pub level: String,
    pub category: String,
    pub action: String,
    pub user_action: Option<String>,
    pub timestamp: Option<String>,
    pub params_summary: Option<serde_json::Value>,
    pub result_summary: Option<serde_json::Value>,
    pub duration_ms: Option<i64>,
    pub error_message: Option<String>,
}

/// Validate a single frontend log entry. Rejects levels outside the DB's
/// CHECK constraint and missing/oversized identifiers, so one bad entry can
/// never silently kill the whole batch (previously it failed the INSERT and
/// the error was swallowed while returning 200).
fn validate_entry(entry: &FrontendLogEntry) -> Result<(), String> {
    if !matches!(entry.level.as_str(), "info" | "warn" | "error") {
        return Err(format!(
            "invalid level '{}' (must be info/warn/error)",
            entry.level
        ));
    }
    if entry.category.trim().is_empty() || entry.action.trim().is_empty() {
        return Err("category and action are required".to_string());
    }
    if entry.category.chars().count() > MAX_NAME_LEN {
        return Err(format!("category too long (max {} chars)", MAX_NAME_LEN));
    }
    if entry.action.chars().count() > MAX_NAME_LEN {
        return Err(format!("action too long (max {} chars)", MAX_NAME_LEN));
    }
    Ok(())
}

/// Truncate summary text to MAX_SUMMARY_LEN chars (per the logging spec).
fn truncate(value: String) -> String {
    if value.chars().count() > MAX_SUMMARY_LEN {
        let mut t: String = value.chars().take(MAX_SUMMARY_LEN).collect();
        t.push('…');
        t
    } else {
        value
    }
}

/// Replace sensitive fields in JSON summaries with "***" (per the logging
/// spec: params_summary must never contain raw api keys/tokens).
fn mask_sensitive(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let masked: serde_json::Map<String, serde_json::Value> = map
                .iter()
                .map(|(k, v)| {
                    let key = k.to_lowercase();
                    if matches!(
                        key.as_str(),
                        "api_key" | "apikey" | "authorization" | "token" | "secret" | "password"
                    ) {
                        (k.clone(), serde_json::Value::String("***".to_string()))
                    } else {
                        (k.clone(), mask_sensitive(v))
                    }
                })
                .collect();
            serde_json::Value::Object(masked)
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(mask_sensitive).collect())
        }
        other => other.clone(),
    }
}

pub async fn ingest_logs(
    State(state): State<&'static AppState>,
    Json(req): Json<BatchLogRequest>,
) -> (StatusCode, String) {
    if req.logs.is_empty() {
        return (StatusCode::OK, "ok".to_string());
    }
    if req.logs.len() > MAX_BATCH_SIZE {
        return (
            StatusCode::BAD_REQUEST,
            format!("batch too large (max {} entries)", MAX_BATCH_SIZE),
        );
    }
    if let Some(bad) = req.logs.iter().find(|e| validate_entry(e).is_err()) {
        let msg = validate_entry(bad).unwrap_err();
        return (
            StatusCode::BAD_REQUEST,
            format!("invalid log entry: {}", msg),
        );
    }

    let fallback = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let records: Vec<AuditRecord> = req.logs.iter().map(|entry| AuditRecord {
        id: new_id(),
        // Per-entry timestamp from the client when present and parseable.
        timestamp: entry
            .timestamp
            .clone()
            .filter(|t| chrono::DateTime::parse_from_rfc3339(t).is_ok())
            .unwrap_or_else(|| fallback.clone()),
        source: "frontend".to_string(),
        level: entry.level.clone(),
        category: entry.category.clone(),
        action: entry.action.clone(),
        user_action: entry.user_action.clone(),
        method: None,
        path: None,
        status_code: None,
        duration_ms: entry.duration_ms,
        params_summary: entry
            .params_summary
            .as_ref()
            .map(|v| truncate(mask_sensitive(v).to_string())),
        result_summary: entry
            .result_summary
            .as_ref()
            .map(|v| truncate(mask_sensitive(v).to_string())),
        error_message: entry.error_message.as_ref().map(|s| truncate(s.clone())),
    }).collect();

    match audit::batch_insert(state.db, &records) {
        Ok(()) => (StatusCode::OK, "ok".to_string()),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to store logs: {}", e),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(level: &str) -> FrontendLogEntry {
        FrontendLogEntry {
            level: level.to_string(),
            category: "ui".to_string(),
            action: "click".to_string(),
            user_action: None,
            timestamp: None,
            params_summary: None,
            result_summary: None,
            duration_ms: None,
            error_message: None,
        }
    }

    #[test]
    fn accepts_valid_levels() {
        for lvl in ["info", "warn", "error"] {
            assert!(validate_entry(&entry(lvl)).is_ok(), "level {lvl} should be valid");
        }
    }

    #[test]
    fn rejects_unknown_levels() {
        for lvl in ["debug", "trace", "fatal", "INFO", ""] {
            assert!(validate_entry(&entry(lvl)).is_err(), "level {lvl} should be rejected");
        }
    }

    #[test]
    fn rejects_missing_category_or_action() {
        let mut e = entry("info");
        e.category = "  ".to_string();
        assert!(validate_entry(&e).is_err());
        let mut e = entry("info");
        e.action = String::new();
        assert!(validate_entry(&e).is_err());
    }

    #[test]
    fn truncates_long_summaries() {
        let long = truncate("x".repeat(3000));
        assert_eq!(long.chars().count(), MAX_SUMMARY_LEN + 1); // + ellipsis
        assert_eq!(truncate("short".to_string()), "short");
    }

    #[test]
    fn masks_sensitive_fields_recursively() {
        let v = serde_json::json!({
            "api_key": "sk-1234",
            "model": {"name": "gpt", "token": "abc"},
            "list": [{"password": "p"}, 1],
            "safe": "keep"
        });
        let m = mask_sensitive(&v);
        assert_eq!(m["api_key"], "***");
        assert_eq!(m["model"]["token"], "***");
        assert_eq!(m["list"][0]["password"], "***");
        assert_eq!(m["list"][1], 1);
        assert_eq!(m["safe"], "keep");
    }

    #[test]
    fn entry_accepts_parseable_timestamp() {
        let mut e = entry("info");
        e.timestamp = Some("2026-08-06T10:00:00Z".into());
        assert!(validate_entry(&e).is_ok());
    }
}
