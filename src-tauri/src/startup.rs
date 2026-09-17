//! Readable process-start failures.
//!
//! The desktop `run` path and the web `server` binary used to abort with
//! `unwrap`/`expect`. Those panics hide the cause from a user who can fix
//! a missing directory or a bad path; these helpers print a sentence instead.

use std::path::Path;

/// Print a startup failure to stderr.
pub fn report(msg: impl std::fmt::Display) {
    eprintln!("{msg}");
}

/// Print `msg` and return it so Tauri `setup` can fail with that text.
pub fn fail(msg: impl Into<String>) -> String {
    let msg = msg.into();
    report(&msg);
    msg
}

/// Print `msg` and exit the process. Used by the standalone web server.
pub fn exit(msg: impl std::fmt::Display) -> ! {
    report(msg);
    std::process::exit(1);
}

/// SQLite and the master-key path both need a UTF-8 `&str`.
pub fn utf8_path(path: &Path, what: &str) -> Result<String, String> {
    path.to_str()
        .map(str::to_string)
        .ok_or_else(|| format!("{what}包含无法处理的字符：{}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn utf8_path_accepts_ascii() {
        assert_eq!(
            utf8_path(Path::new("lexio.db"), "数据库路径").unwrap(),
            "lexio.db"
        );
    }
}
