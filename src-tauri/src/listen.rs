//! Loopback bind for the local API.
//!
//! Port 3001 is also named in the Vite proxy and the desktop CSP
//! `connect-src`, so this helper does not silently roll forward to another
//! port. Occupancy is reported as a readable `PORT_IN_USE` error instead of
//! panicking on `unwrap`.

use std::io::ErrorKind;
use std::net::{Ipv4Addr, SocketAddr, TcpListener as StdTcpListener};

use tokio::net::TcpListener;

/// Preferred (and currently only) local API port.
pub const API_PORT: u16 = 3001;

const BIND_HOST: Ipv4Addr = Ipv4Addr::LOCALHOST;

/// User-facing bind failure. `PORT_IN_USE` is a stable ASCII token for logs
/// and scripts; the rest of the sentence is meant to be read by a person.
pub fn bind_error_message(port: u16, err: &std::io::Error) -> String {
    if err.kind() == ErrorKind::AddrInUse {
        format!(
            "无法启动本地 API：端口 {port} 已被占用（PORT_IN_USE）。请关闭占用该端口的程序后重试。"
        )
    } else {
        format!("无法监听 127.0.0.1:{port}：{err}")
    }
}

/// Bind `127.0.0.1:{port}` with a std listener, already set non-blocking so it
/// can be handed to Tokio. Returns the listener and the port that was bound.
pub fn bind_loopback_std(port: u16) -> Result<(StdTcpListener, u16), String> {
    let listener = match StdTcpListener::bind(SocketAddr::from((BIND_HOST, port))) {
        Ok(listener) => listener,
        Err(err) => return Err(bind_error_message(port, &err)),
    };
    let actual = listener
        .local_addr()
        .map_err(|e| format!("无法读取监听地址：{e}"))?
        .port();
    listener
        .set_nonblocking(true)
        .map_err(|e| format!("无法启动本地 API：{e}"))?;
    Ok((listener, actual))
}

/// Bind `127.0.0.1:{port}` on the current Tokio runtime.
pub async fn bind_loopback(port: u16) -> Result<(TcpListener, u16), String> {
    let (std_listener, actual) = bind_loopback_std(port)?;
    TcpListener::from_std(std_listener)
        .map(|listener| (listener, actual))
        .map_err(|e| format!("无法启动本地 API：{e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn occupied_port_returns_readable_message() {
        let held = StdTcpListener::bind(SocketAddr::from((BIND_HOST, 0))).unwrap();
        let port = held.local_addr().unwrap().port();
        let err = bind_loopback_std(port).unwrap_err();
        assert!(
            err.contains("PORT_IN_USE"),
            "expected PORT_IN_USE marker, got: {err}"
        );
        assert!(
            err.contains(&port.to_string()),
            "expected the occupied port in the message, got: {err}"
        );
        assert!(
            !err.to_lowercase().contains("unwrap"),
            "must not look like a panic: {err}"
        );
    }

    #[test]
    fn free_ephemeral_port_binds() {
        let (listener, port) = bind_loopback_std(0).unwrap();
        assert!(port > 0);
        drop(listener);
    }

    #[test]
    fn other_errors_are_not_tagged_port_in_use() {
        // Port 1 is typically privileged; bind should fail with something
        // other than AddrInUse on developer machines. If it somehow
        // succeeds (running as root), the test is a no-op.
        match bind_loopback_std(1) {
            Ok((listener, _)) => drop(listener),
            Err(msg) => {
                if !msg.contains("PORT_IN_USE") {
                    assert!(
                        msg.contains("127.0.0.1:1"),
                        "other bind failures must still name the address, got: {msg}"
                    );
                }
            }
        }
    }
}
