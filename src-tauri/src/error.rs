//! Error classification for the local HTTP API.
//!
//! Repository code returns `Result<T, String>`, which makes it easy to leak
//! internals: a raw `rusqlite` message (`near "SELECT": syntax error`,
//! `database is locked`, `Query returned no rows`, `UNIQUE constraint failed`)
//! or an OS error with a filesystem path would otherwise reach the client and
//! help an attacker map the schema and implementation.
//!
//! The convention is therefore explicit at the source: messages that are meant
//! for the user are written by hand (validation text and the `*_ERROR:` codes
//! the frontend maps to actionable hints), while anything forwarded from a
//! lower layer is wrapped with [`internal`]. The HTTP boundary then decides
//! what may be shown, and records the detail in the audit log instead.
//!
//! This keeps the existing `Result<T, String>` signatures — the marker travels
//! inside the string — so the classification is enforced in exactly one place
//! ([`crate::api::blocking`]) rather than at ~200 call sites.

/// Delimiters are control characters so a hand-written message can never
/// collide with the marker by accident.
const INTERNAL_PREFIX: &str = "\u{1}lexio-internal\u{1}";

/// Message returned to the client in place of any internal detail.
pub const GENERIC_INTERNAL_MESSAGE: &str =
    "服务器内部错误，详情已记录到日志。请稍后重试，或重启应用。";

/// Wrap a lower-layer error so the HTTP boundary hides its detail.
///
/// Use this for anything forwarded from `rusqlite`, a mutex lock, the
/// filesystem, or an HTTP client. Do **not** wrap hand-written, user-facing
/// validation messages.
///
/// Idempotent: re-wrapping an already marked message is a no-op, so an error
/// can safely pass through several layers that each call `internal`.
pub fn internal<E: std::fmt::Display>(err: E) -> String {
    let text = err.to_string();
    if is_internal(&text) {
        return text;
    }
    format!("{INTERNAL_PREFIX}{text}")
}

/// True when the message was marked by [`internal`].
pub fn is_internal(err: &str) -> bool {
    err.starts_with(INTERNAL_PREFIX)
}

/// Strip the marker, returning the detail that must only go to the log.
pub fn internal_detail(err: &str) -> &str {
    err.strip_prefix(INTERNAL_PREFIX).unwrap_or(err)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_round_trips_the_detail() {
        let marked = internal("near \"SELECT\": syntax error");
        assert!(is_internal(&marked));
        assert_eq!(internal_detail(&marked), "near \"SELECT\": syntax error");
    }

    #[test]
    fn internal_is_idempotent() {
        let once = internal("boom");
        assert_eq!(
            internal(once.clone()),
            once,
            "re-wrapping must not double-mark"
        );
        assert_eq!(internal_detail(&internal(once.clone())), "boom");
    }

    #[test]
    fn user_facing_messages_are_not_marked() {
        for msg in [
            "请输入要学习的主题",
            "MISSING_API_KEY: 请填写 API Key",
            "模型不存在",
            "AUTH_ERROR: API Key 无效或权限不足",
        ] {
            assert!(!is_internal(msg), "{msg} must stay user-facing");
            assert_eq!(internal_detail(msg), msg, "unmarked text is returned as-is");
        }
    }

    /// A user-facing message must not be able to masquerade as internal, and
    /// stripping the marker must leave no control characters behind.
    #[test]
    fn marker_is_stripped_cleanly() {
        let marked = internal("boom");
        assert!(
            marked.starts_with('\u{1}'),
            "marker must be delimited by a control char"
        );
        assert!(
            marked.chars().any(|c| c == '\u{1}'),
            "no plain text error can contain U+0001, so this cannot be forged"
        );
        let detail = internal_detail(&marked);
        assert_eq!(detail, "boom");
        assert!(
            !detail.contains('\u{1}'),
            "stripped detail must be safe to display"
        );
    }

    #[test]
    fn works_with_any_display_error() {
        let io = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        assert_eq!(internal_detail(&internal(io)), "denied");
    }
}
