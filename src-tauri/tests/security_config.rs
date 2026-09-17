//! Guards the Tauri security config.
//!
//! The webview CSP silently degrades to "no policy at all" when the field is
//! missing or null, and a `connect-src` without Tauri's IPC origins breaks every
//! `invoke()` call with no error beyond a console violation. Both mistakes are
//! invisible until the desktop app is exercised by hand, so they are asserted
//! here instead.

const CONFIG: &str = include_str!("../tauri.conf.json");

fn security() -> serde_json::Value {
    let config: serde_json::Value = serde_json::from_str(CONFIG).expect("tauri.conf.json is valid JSON");
    config["app"]["security"].clone()
}

#[test]
fn csp_is_not_disabled() {
    let csp = &security()["csp"];
    assert!(!csp.is_null(), "app.security.csp must not be null: that disables CSP entirely");
    let text = csp.as_str().expect("csp should be a string");
    assert!(!text.trim().is_empty(), "csp must not be empty");
}

#[test]
fn production_csp_allows_the_tauri_ipc_bridge() {
    let csp = security()["csp"].as_str().unwrap().to_string();
    let connect = csp
        .split(';')
        .map(str::trim)
        .find(|d| d.starts_with("connect-src"))
        .expect("csp must declare connect-src");

    // Without both of these, every invoke() to a Rust command is blocked.
    assert!(connect.contains("ipc:"), "connect-src must list `ipc:`");
    assert!(
        connect.contains("http://ipc.localhost"),
        "connect-src must list `http://ipc.localhost`"
    );
}

#[test]
fn production_csp_keeps_script_src_strict() {
    let csp = security()["csp"].as_str().unwrap().to_string();
    let script = csp
        .split(';')
        .map(str::trim)
        .find(|d| d.starts_with("script-src"))
        .expect("csp must declare script-src");

    // Tauri hashes the bundled scripts at build time, so production does not
    // need 'unsafe-inline'. Allowing it would defeat the point of the CSP.
    assert!(
        !script.contains("unsafe-inline"),
        "production script-src must not allow 'unsafe-inline'"
    );
    assert!(
        !script.contains("unsafe-eval"),
        "production script-src must not allow 'unsafe-eval'"
    );
}

#[test]
fn csp_locks_down_privileged_directives() {
    let csp = security()["csp"].as_str().unwrap().to_string();
    assert!(csp.contains("object-src 'none'"), "object-src should be 'none'");
    assert!(csp.contains("base-uri 'self'"), "base-uri should be 'self'");
    assert!(csp.contains("frame-ancestors 'none'"), "frame-ancestors should be 'none'");
}

/// Dev loads unhashed HMR scripts from the Vite server, so it needs its own,
/// looser policy — but it still must not lose the IPC origins.
#[test]
fn dev_csp_covers_the_dev_server_and_ipc() {
    let dev = &security()["devCsp"];
    assert!(!dev.is_null(), "devCsp should be set so dev keeps working");
    let dev = dev.as_str().unwrap();
    assert!(dev.contains("ipc:"), "dev connect-src must list `ipc:`");
    assert!(
        dev.contains("http://ipc.localhost"),
        "dev connect-src must list `http://ipc.localhost`"
    );
    assert!(dev.contains("http://localhost"), "dev must reach the Vite server");
}

/// Every granted capability widens what an injected script can reach, so the
/// ones the frontend does not use must not be present.
#[test]
fn unused_capabilities_are_not_granted() {
    const CAPABILITY: &str = include_str!("../capabilities/default.json");
    let cap: serde_json::Value = serde_json::from_str(CAPABILITY).expect("valid JSON");
    let permissions: Vec<String> = cap["permissions"]
        .as_array()
        .expect("permissions list")
        .iter()
        .map(|p| p.as_str().expect("permission is a string").to_string())
        .collect();

    // The frontend never calls the opener plugin, so granting it would only add
    // an escape hatch (open arbitrary URLs/paths) that nothing needs.
    assert!(
        !permissions.iter().any(|p| p.starts_with("opener:")),
        "opener permissions must not be granted: {permissions:?}"
    );
    assert!(
        permissions.iter().any(|p| p == "core:default"),
        "core:default is required for the app to run"
    );
}
