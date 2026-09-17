use std::fmt::Debug;
use tokio::sync::mpsc;
use tracing::{Event, Subscriber, field::Visit};
use tracing_subscriber::{Layer, layer::Context};
use crate::repo::audit::{self, AuditRecord};
use crate::db::Database;
use crate::models::new_id;

/// Maximum backlog of audit events. Older events are dropped if the channel is full.
const MAX_BACKLOG: usize = 1000;

/// Batch size — flush to DB when this many records accumulate.
const BATCH_SIZE: usize = 50;

/// Flush interval — flush to DB at least this often (in milliseconds).
const FLUSH_INTERVAL_MS: u64 = 500;

/// A tracing Layer that writes audit events to SQLite via a non-blocking channel.
pub struct AuditDbLayer {
    tx: mpsc::Sender<AuditRecord>,
}

impl AuditDbLayer {
    /// Create a new AuditDbLayer, spawning a background task that writes to `db`.
    pub fn new(db: &'static Database) -> Self {
        let (tx, mut rx) = mpsc::channel::<AuditRecord>(MAX_BACKLOG);

        tokio::spawn(async move {
            let mut buffer: Vec<AuditRecord> = Vec::with_capacity(BATCH_SIZE);
            eprintln!("AuditDbLayer bg task: started, db={:p}", db as *const _);
            loop {
                // Wait for first event or flush interval
                let deadline = tokio::time::Instant::now()
                    + std::time::Duration::from_millis(FLUSH_INTERVAL_MS);

                let flush_reason = loop {
                    match tokio::time::timeout_at(deadline, rx.recv()).await {
                        Ok(Some(record)) => {
                            buffer.push(record);
                            // Drain any additional queued events without blocking
                            while let Ok(record) = rx.try_recv() {
                                buffer.push(record);
                                if buffer.len() >= BATCH_SIZE {
                                    break;
                                }
                            }
                            if buffer.len() >= BATCH_SIZE {
                                break "batch_full";
                            }
                        }
                        Ok(None) => break "channel_closed",
                        Err(_) => break "interval_elapsed",
                    }
                };

                if !buffer.is_empty() {
                    let _ = audit::batch_insert(db, &buffer);
                    buffer.clear();
                }

                if flush_reason == "channel_closed" {
                    break;
                }
            }
        });

        AuditDbLayer { tx }
    }
}

impl<S: Subscriber> Layer<S> for AuditDbLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        // Only process events targeting "audit"
        if event.metadata().target() != "audit" {
            return;
        }

        let mut visitor = AuditVisitor::default();
        event.record(&mut visitor);

        // Require at minimum category and action
        let category = match &visitor.category {
            Some(c) => c.clone(),
            None => return,
        };
        let action = match &visitor.action {
            Some(a) => a.clone(),
            None => return,
        };

        let record = AuditRecord {
            id: new_id(),
            timestamp: chrono::Utc::now()
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            source: visitor.source.unwrap_or_else(|| "backend".to_string()),
            level: event.metadata().level().as_str().to_lowercase(),
            category,
            action,
            user_action: visitor.user_action,
            method: visitor.method,
            path: visitor.path,
            status_code: visitor.status_code,
            duration_ms: visitor.duration_ms,
            params_summary: visitor.params_summary,
            result_summary: visitor.result_summary,
            error_message: visitor.error_message,
        };

        // Non-blocking send; if channel is full, drop the event
        let _ = self.tx.try_send(record);
    }
}

/// Field visitor that extracts audit fields from a tracing event.
#[derive(Default)]
struct AuditVisitor {
    source: Option<String>,
    category: Option<String>,
    action: Option<String>,
    user_action: Option<String>,
    method: Option<String>,
    path: Option<String>,
    status_code: Option<i32>,
    duration_ms: Option<i64>,
    params_summary: Option<String>,
    result_summary: Option<String>,
    error_message: Option<String>,
}

impl Visit for AuditVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn Debug) {
        let s = format!("{:?}", value);
        // Fields that use %-format or ?-format sigils
        match field.name() {
            "method" => self.method = Some(s),
            "path" => self.path = Some(s),
            "params_summary" => self.params_summary = Some(s),
            "result_summary" => self.result_summary = Some(s),
            "error_message" => self.error_message = Some(s),
            _ => {}
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        match field.name() {
            "source" => self.source = Some(value.to_string()),
            "category" => self.category = Some(value.to_string()),
            "action" => self.action = Some(value.to_string()),
            "user_action" => self.user_action = Some(value.to_string()),
            "method" => self.method = Some(value.to_string()),
            "path" => self.path = Some(value.to_string()),
            "params_summary" => self.params_summary = Some(value.to_string()),
            "result_summary" => self.result_summary = Some(value.to_string()),
            "error_message" => self.error_message = Some(value.to_string()),
            _ => {}
        }
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        match field.name() {
            "status_code" => self.status_code = Some(value as i32),
            "duration_ms" => self.duration_ms = Some(value),
            _ => {}
        }
    }
}
