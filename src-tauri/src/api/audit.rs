//! Read access to the audit trail.
//!
//! The trail was write-only: `repo::audit::list` had no callers and no route
//! exposed it, so a user could not see what the application had recorded about
//! them — which is most of the point of keeping an audit log, and the part that
//! makes the record trustworthy to the person it describes.
//!
//! Reads are deliberately narrow. They are only reachable with the API token,
//! the page size is clamped, and every filter is bound as a query parameter
//! rather than interpolated, so a search term cannot alter the statement. The
//! returned rows carry the server-side timestamp; the client's own clock, when
//! present, is returned separately and labelled as untrusted.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::api::ai_routes::AppState;
use crate::api::blocking;
use crate::repo::audit::{self, AuditFilter, AuditRecord};

/// Rows per page when the client does not ask for a specific size.
const DEFAULT_PAGE_SIZE: i64 = 50;

#[derive(Debug, Deserialize)]
pub struct AuditQuery {
    pub level: Option<String>,
    pub source: Option<String>,
    pub category: Option<String>,
    /// Free-text match over action, user_action, path and error_message.
    pub search: Option<String>,
    /// RFC3339 inclusive lower bound on the server timestamp.
    pub since: Option<String>,
    /// RFC3339 inclusive upper bound on the server timestamp.
    pub until: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl AuditQuery {
    /// Turn the request into a repository filter, rejecting unusable bounds.
    ///
    /// `limit`/`offset` are clamped rather than rejected — a caller asking for
    /// more than the maximum gets the maximum — but a malformed time bound is a
    /// real mistake and is reported, because silently ignoring it would return a
    /// wider range than was asked for.
    fn into_filter(self) -> Result<AuditFilter, String> {
        // A blank bound is "not supplied", normalised here so the repository
        // never has to decide whether whitespace means a filter.
        let clean = |v: Option<String>| v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        let since = clean(self.since);
        let until = clean(self.until);
        for (name, value) in [("since", &since), ("until", &until)] {
            if let Some(raw) = value.as_deref() {
                if chrono::DateTime::parse_from_rfc3339(raw).is_err() {
                    return Err(format!(
                        "{name} 必须是 RFC3339 时间，例如 2026-09-17T00:00:00Z"
                    ));
                }
            }
        }
        Ok(AuditFilter {
            level: clean(self.level),
            source: clean(self.source),
            category: clean(self.category),
            search: clean(self.search),
            since,
            until,
            offset: self.offset.unwrap_or(0).max(0),
            limit: self
                .limit
                .unwrap_or(DEFAULT_PAGE_SIZE)
                .clamp(1, audit::MAX_PAGE_SIZE),
        })
    }
}

/// One page of the audit trail, newest first.
#[derive(Debug, Serialize)]
pub struct AuditPage {
    pub logs: Vec<AuditRecord>,
    /// Total rows matching the filter, ignoring paging.
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
    /// True when `timestamp` is the server's clock and `client_timestamp` is
    /// diagnostic only. Stated in the payload so a reader of the API does not
    /// have to infer the trust model from documentation.
    pub timestamp_authority: &'static str,
}

pub async fn list_logs(
    State(state): State<&'static AppState>,
    Query(query): Query<AuditQuery>,
) -> Result<Json<AuditPage>, (StatusCode, String)> {
    let filter = query
        .into_filter()
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let limit = filter.limit;
    let offset = filter.offset;
    let (logs, total) = blocking::run(move || {
        let rows = audit::query(state.db, &filter)?;
        let total = audit::count(state.db, &filter)?;
        Ok((rows, total))
    })
    .await?;

    Ok(Json(AuditPage {
        logs,
        total,
        limit,
        offset,
        timestamp_authority: "server",
    }))
}

/// The distinct levels, sources and categories actually present.
///
/// The filter UI is populated from this rather than from a hard-coded list, so
/// it cannot offer a value nothing has ever written.
pub async fn get_facets(
    State(state): State<&'static AppState>,
) -> Result<Json<audit::AuditFacets>, (StatusCode, String)> {
    let facets = blocking::run(move || audit::facets(state.db)).await?;
    Ok(Json(facets))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query() -> AuditQuery {
        AuditQuery {
            level: None,
            source: None,
            category: None,
            search: None,
            since: None,
            until: None,
            limit: None,
            offset: None,
        }
    }

    #[test]
    fn defaults_to_a_bounded_page() {
        let f = query().into_filter().unwrap();
        assert_eq!(f.limit, DEFAULT_PAGE_SIZE);
        assert_eq!(f.offset, 0);
    }

    /// A caller cannot raise the page size past the repository ceiling, and a
    /// nonsense value cannot get through as zero rows.
    #[test]
    fn limit_is_clamped_into_range() {
        let mut q = query();
        q.limit = Some(i64::MAX);
        assert_eq!(q.into_filter().unwrap().limit, audit::MAX_PAGE_SIZE);

        let mut q = query();
        q.limit = Some(0);
        assert_eq!(q.into_filter().unwrap().limit, 1);

        let mut q = query();
        q.limit = Some(-5);
        assert_eq!(q.into_filter().unwrap().limit, 1);
    }

    #[test]
    fn negative_offset_becomes_zero() {
        let mut q = query();
        q.offset = Some(-100);
        assert_eq!(q.into_filter().unwrap().offset, 0);
    }

    #[test]
    fn accepts_rfc3339_bounds() {
        let mut q = query();
        q.since = Some("2026-09-01T00:00:00Z".into());
        q.until = Some("2026-09-17T23:59:59.999Z".into());
        assert!(q.into_filter().is_ok());
    }

    /// A bad time bound is an error, not a silently ignored filter.
    #[test]
    fn rejects_unparseable_bounds() {
        let mut q = query();
        q.since = Some("yesterday".into());
        let err = q.into_filter().unwrap_err();
        assert!(
            err.contains("since"),
            "the message names the bad field: {err}"
        );

        let mut q = query();
        q.until = Some("2026-13-45".into());
        assert!(q.into_filter().unwrap_err().contains("until"));
    }

    /// A blank bound means "not supplied", not "a filter containing spaces".
    #[test]
    fn blank_bounds_are_normalised_away() {
        let mut q = query();
        q.since = Some("   ".into());
        q.until = Some(String::new());
        q.level = Some("".into());
        q.search = Some("  ".into());
        let f = q.into_filter().unwrap();
        assert_eq!(f.since, None);
        assert_eq!(f.until, None);
        assert_eq!(f.level, None);
        assert_eq!(f.search, None);
    }

    /// Filter values are trimmed so a stray space does not turn an exact match
    /// into a miss.
    #[test]
    fn filter_values_are_trimmed() {
        let mut q = query();
        q.level = Some(" error ".into());
        q.source = Some(" frontend".into());
        q.category = Some("ui  ".into());
        let f = q.into_filter().unwrap();
        assert_eq!(f.level.as_deref(), Some("error"));
        assert_eq!(f.source.as_deref(), Some("frontend"));
        assert_eq!(f.category.as_deref(), Some("ui"));
    }

    #[test]
    fn filters_are_carried_through_untouched() {
        let mut q = query();
        q.level = Some("error".into());
        q.source = Some("frontend".into());
        q.category = Some("ui".into());
        q.search = Some("100%".into());
        let f = q.into_filter().unwrap();
        assert_eq!(f.level.as_deref(), Some("error"));
        assert_eq!(f.source.as_deref(), Some("frontend"));
        assert_eq!(f.category.as_deref(), Some("ui"));
        assert_eq!(f.search.as_deref(), Some("100%"));
    }
}
