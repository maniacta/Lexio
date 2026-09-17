use axum::{
    extract::{ConnectInfo, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::net::SocketAddr;

use crate::api::ai_routes::AppState;

/// Require `X-Lexio-Token` on all routes except health + localhost token bootstrap.
///
/// Also enforces a loopback/Tauri `Host` on every request, including the ones
/// exempt from the token check (see `ALLOWED_HOSTS`).
pub async fn require_token(
    State(state): State<&'static AppState>,
    req: Request,
    next: Next,
) -> Response {
    let host = req
        .headers()
        .get(axum::http::header::HOST)
        .and_then(|v| v.to_str().ok());

    // Defence in depth: gate every route on the Host so a DNS-rebinding page or
    // a direct foreign-origin request cannot reach the token bootstrap.
    if !host_is_allowed(host) {
        return (StatusCode::FORBIDDEN, "FORBIDDEN: 仅允许本机访问本地 API").into_response();
    }

    let path = req.uri().path();
    if path == "/api/health" || path == "/api/auth/token" {
        return next.run(req).await;
    }

    // CORS preflight carries no custom headers and no token by design; the
    // CorsLayer just inside this middleware answers it. Letting it through here
    // is required because this layer runs *outside* CorsLayer — otherwise the
    // preflight would be rejected with 401 and the real request never sent.
    // The Host check above has already run, so this cannot be reached by a
    // foreign origin.
    if is_preflight(req.method()) {
        return next.run(req).await;
    }

    let provided = req
        .headers()
        .get("x-lexio-token")
        .and_then(|v| v.to_str().ok());

    if provided == Some(state.api_token.as_str()) {
        next.run(req).await
    } else {
        (
            StatusCode::UNAUTHORIZED,
            "UNAUTHORIZED: 缺少或无效的 API Token",
        )
            .into_response()
    }
}

/// Hosts accepted on the local API. Requests carrying any other `Host` are
/// rejected before routing.
///
/// This blocks DNS-rebinding (an attacker page whose hostname resolves to
/// 127.0.0.1 arrives with `Host: evil.com`) and any direct access to the
/// backend from a foreign origin.
///
/// Note: it does **not** guard the Vite dev-server path. Vite's proxy rewrites
/// `Host` to the target (`127.0.0.1:3001`), so requests arriving through the
/// proxy always look local. That path is protected by binding the dev server to
/// loopback instead (see `vite.config.ts`); the two controls are complementary.
const ALLOWED_HOSTS: &[&str] = &[
    "127.0.0.1",
    "localhost",
    "[::1]",
    // Tauri webview origin: http://tauri.localhost on Windows,
    // tauri://localhost elsewhere.
    "tauri.localhost",
];

/// Strip an optional scheme from a Host header value. Tauri's webview can send
/// a scheme-qualified origin (`tauri://localhost`) instead of a bare authority.
fn strip_scheme(value: &str) -> &str {
    match value.split_once("://") {
        Some((scheme, rest)) if !scheme.is_empty() => rest,
        _ => value,
    }
}

/// Split a `Host` header value into (host_without_port, port).
fn split_host(value: &str) -> (&str, Option<&str>) {
    let value = strip_scheme(value.trim());
    // Ignore any path/query that may follow a scheme-qualified origin.
    let value = value.split(['/', '?', '#']).next().unwrap_or(value);

    if let Some(rest) = value.strip_prefix('[') {
        // IPv6 literal, e.g. "[::1]:3001"
        return match rest.split_once(']') {
            Some((host, tail)) => (
                // Compare against the bracketed form so "[::1]" matches.
                &value[..host.len() + 2],
                tail.strip_prefix(':'),
            ),
            None => (value, None),
        };
    }
    match value.rsplit_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (value, None),
    }
}

/// True for a CORS preflight. Such a request never carries the token, so the
/// token check must not apply to it.
fn is_preflight(method: &axum::http::Method) -> bool {
    method == axum::http::Method::OPTIONS
}

/// True when the `Host` header names a loopback/Tauri origin.
/// A missing `Host` is rejected: HTTP/1.1 requires it, and accepting its
/// absence would create a bypass.
fn host_is_allowed(host_header: Option<&str>) -> bool {
    let Some(raw) = host_header else {
        return false;
    };
    let (host, port) = split_host(raw);
    if !ALLOWED_HOSTS.contains(&host) {
        return false;
    }
    match port {
        // Port is optional; when present it must be a valid u16.
        Some(p) => p.parse::<u16>().is_ok(),
        None => true,
    }
}

/// Loopback-only bootstrap so the web UI can obtain the session token.
/// Reached only when the Host check above has already passed.
pub async fn bootstrap_token(
    State(state): State<&'static AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if !addr.ip().is_loopback() {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(Json(json!({ "token": state.api_token })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_loopback_and_tauri_hosts() {
        for host in [
            "127.0.0.1",
            "127.0.0.1:3001",
            "localhost",
            "localhost:3001",
            "[::1]",
            "[::1]:3001",
            "tauri.localhost",
            "tauri.localhost:3001",
            // Tauri webview can send a scheme-qualified origin.
            "tauri://localhost",
            "https://tauri.localhost",
            "http://localhost:14200",
        ] {
            assert!(host_is_allowed(Some(host)), "{host} should be allowed");
        }
    }

    #[test]
    fn rejects_foreign_and_missing_hosts() {
        for host in [
            "evil.com",
            "evil.com:3001",
            "192.168.1.10:14200",
            "example.com:3001",
            "127.0.0.1.evil.com",
            "localhost.evil.com",
            "127.0.0.1:notaport",
            // Scheme must not be used to smuggle a foreign host through.
            "tauri://evil.com",
        ] {
            assert!(!host_is_allowed(Some(host)), "{host} must be rejected");
        }
        assert!(!host_is_allowed(None), "missing Host must be rejected");
    }

    /// The regression this whole check exists for: a LAN client reaching the
    /// backend through the dev-server proxy sends its own Host header.
    #[test]
    fn rejects_lan_client_through_proxy() {
        assert!(!host_is_allowed(Some("192.168.0.42:14200")));
        assert!(!host_is_allowed(Some("my-laptop.local:14200")));
    }

    #[test]
    fn split_host_handles_ipv6_and_ports() {
        assert_eq!(split_host("[::1]:3001"), ("[::1]", Some("3001")));
        assert_eq!(split_host("[::1]"), ("[::1]", None));
        assert_eq!(split_host("localhost:3001"), ("localhost", Some("3001")));
        assert_eq!(split_host("localhost"), ("localhost", None));
    }

    /// Desktop reaches the backend cross-origin, so its preflight must reach
    /// CorsLayer instead of being stopped by the token check.
    #[test]
    fn preflight_bypasses_the_token_check() {
        assert!(is_preflight(&axum::http::Method::OPTIONS));
        for m in [
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::DELETE,
        ] {
            assert!(!is_preflight(&m), "{m} must still require a token");
        }
    }
}
