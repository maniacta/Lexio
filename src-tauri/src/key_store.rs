//! Master key storage.
//!
//! The key that encrypts API keys at rest was kept in a plaintext file next to
//! the database, so the key and the ciphertext shared one location and anything
//! able to read the app data directory — a backup, a synced folder, another
//! user account, malware without admin rights — could decrypt every stored API
//! key. The key is now held by the OS credential store (Windows Credential
//! Manager, macOS Keychain, Linux Secret Service), which gates access on the
//! logged-in user account rather than on filesystem permissions.
//!
//! The decision logic is separated from the credential backend so every branch
//! can be unit-tested without touching a real OS credential store.

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use std::path::Path;

/// Credential store coordinates. The service is the app identifier so the entry
/// is attributable to Lexio in the OS UI.
pub const SERVICE: &str = "com.maniact.lexio";
pub const ACCOUNT: &str = "master-key";

/// Set `LEXIO_MASTER_KEY_STORE=file` to force file storage, e.g. on a headless
/// server where no credential store exists but the app must still boot.
const STORE_ENV: &str = "LEXIO_MASTER_KEY_STORE";

pub const KEY_LEN: usize = 32;

/// Where the master key came from, for logging and tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyOutcome {
    /// Read from the OS credential store; no plaintext file involved.
    CredentialStore,
    /// Adopted from the plaintext file, stored in the credential store, and
    /// the file removed.
    MigratedFromFile,
    /// Newly generated and saved to the credential store.
    CreatedInCredentialStore,
    /// Credential store forced off; read from the file.
    FileForced,
    /// Credential store unreachable; read from the file.
    FileFallback,
    /// Both held a key and they matched, so the redundant file was removed.
    RedundantFileRemoved,
    /// Both held a key but they differed: the store wins and the file is left
    /// alone, because deleting it could destroy the only copy of a key needed
    /// to decrypt existing rows.
    StorePreferredOverConflictingFile,
}

impl KeyOutcome {
    /// True when the key is only protected by file permissions.
    pub fn is_plaintext_file(&self) -> bool {
        matches!(self, Self::FileForced | Self::FileFallback)
    }
}

/// The credential backend, abstracted so the logic below is testable.
pub trait CredentialStore {
    /// `Ok(None)` means "no entry yet", `Err` means unusable.
    fn get(&self) -> Result<Option<[u8; KEY_LEN]>, String>;
    fn set(&self, key: &[u8; KEY_LEN]) -> Result<(), String>;
}

/// OS-backed implementation.
///
/// The service/account are parameters so tests can use their own namespace and
/// never touch the entry the app itself reads.
pub struct OsCredentialStore {
    service: String,
    account: String,
}

impl OsCredentialStore {
    pub fn new(service: &str, account: &str) -> Self {
        Self {
            service: service.to_string(),
            account: account.to_string(),
        }
    }

    /// The store the application uses.
    pub fn app() -> Self {
        Self::new(SERVICE, ACCOUNT)
    }
}

/// Remove the app's credential. Only used by tests and explicit cleanup.
impl OsCredentialStore {
    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    pub fn delete(&self) -> Result<(), String> {
        let entry =
            keyring::Entry::new(&self.service, &self.account).map_err(crate::error::internal)?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(crate::error::internal(e)),
        }
    }
}

/// Platforms with a backend wired up in the dependency table.
const OS_STORE_SUPPORTED: bool = cfg!(any(
    target_os = "windows",
    target_os = "macos",
    target_os = "linux"
));

impl CredentialStore for OsCredentialStore {
    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    fn get(&self) -> Result<Option<[u8; KEY_LEN]>, String> {
        let entry =
            keyring::Entry::new(&self.service, &self.account).map_err(crate::error::internal)?;
        match entry.get_password() {
            Ok(secret) => key_from_b64(&secret).map(Some),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(crate::error::internal(e)),
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    fn get(&self) -> Result<Option<[u8; KEY_LEN]>, String> {
        Err("当前平台没有可用的系统凭据库".to_string())
    }

    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    fn set(&self, key: &[u8; KEY_LEN]) -> Result<(), String> {
        let entry =
            keyring::Entry::new(&self.service, &self.account).map_err(crate::error::internal)?;
        entry
            .set_password(&b64_key(key))
            .map_err(crate::error::internal)
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    fn set(&self, _key: &[u8; KEY_LEN]) -> Result<(), String> {
        Err("当前平台没有可用的系统凭据库".to_string())
    }
}

fn env_forces_file() -> bool {
    matches!(
        std::env::var(STORE_ENV).as_deref(),
        Ok("file") | Ok("FILE") | Ok("File")
    )
}

fn b64_key(key: &[u8; KEY_LEN]) -> String {
    B64.encode(key)
}

/// Decode a base64 key, rejecting anything not exactly 32 bytes.
fn key_from_b64(raw: &str) -> Result<[u8; KEY_LEN], String> {
    let bytes = B64
        .decode(raw.trim().as_bytes())
        .map_err(|_| "主密钥编码无效".to_string())?;
    if bytes.len() != KEY_LEN {
        return Err("主密钥长度无效".to_string());
    }
    let mut arr = [0u8; KEY_LEN];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}

fn random_key() -> Result<[u8; KEY_LEN], String> {
    let mut arr = [0u8; KEY_LEN];
    getrandom::fill(&mut arr).map_err(crate::error::internal)?;
    Ok(arr)
}

/// Read the legacy plaintext key file, if present and well-formed.
fn read_file_key(path: &Path) -> Result<Option<[u8; KEY_LEN]>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let bytes = std::fs::read(path).map_err(crate::error::internal)?;
    if bytes.len() != KEY_LEN {
        // Do not silently replace a corrupt key: that would make existing
        // ciphertext undecryptable and look like data loss.
        return Err("主密钥文件损坏，请删除后重新填写 API Key".into());
    }
    let mut arr = [0u8; KEY_LEN];
    arr.copy_from_slice(&bytes);
    Ok(Some(arr))
}

fn write_file_key(path: &Path, key: &[u8; KEY_LEN]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(crate::error::internal)?;
        }
    }
    std::fs::write(path, key).map_err(crate::error::internal)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

fn remove_file_quietly(path: &Path) {
    if let Err(e) = std::fs::remove_file(path) {
        tracing::warn!(
            target: "audit",
            source = "backend",
            category = "system",
            action = "master_key_file_remove_failed",
            error_message = %e,
        );
    }
}

/// Decide the master key given a credential store and the legacy file path.
///
/// Preference order: credential store, then an existing plaintext file (which
/// is migrated out of), then a freshly generated key. The file is never written
/// as a second copy once the store holds the key.
pub fn resolve_with(
    store: &dyn CredentialStore,
    key_path: &Path,
    force_file: bool,
) -> Result<([u8; KEY_LEN], KeyOutcome), String> {
    if force_file {
        return match read_file_key(key_path)? {
            Some(k) => Ok((k, KeyOutcome::FileForced)),
            None => {
                let k = random_key()?;
                write_file_key(key_path, &k)?;
                Ok((k, KeyOutcome::FileForced))
            }
        };
    }

    if !OS_STORE_SUPPORTED {
        return match read_file_key(key_path)? {
            Some(k) => Ok((k, KeyOutcome::FileFallback)),
            None => {
                let k = random_key()?;
                write_file_key(key_path, &k)?;
                Ok((k, KeyOutcome::FileFallback))
            }
        };
    }

    let file_key = read_file_key(key_path)?;

    match store.get() {
        Ok(Some(stored)) => {
            return match file_key {
                // Redundant plaintext copy: safe to delete, it is the same key.
                Some(f) if f == stored => {
                    remove_file_quietly(key_path);
                    Ok((stored, KeyOutcome::RedundantFileRemoved))
                }
                // Conflicting keys: the store wins, but the file is kept. It may
                // be the only copy of a key that encrypted existing rows.
                Some(_) => Ok((stored, KeyOutcome::StorePreferredOverConflictingFile)),
                None => Ok((stored, KeyOutcome::CredentialStore)),
            };
        }
        Ok(None) => {}
        Err(e) => {
            // Store unusable right now, but a file may still hold the real key
            // — fall back to it rather than generating a new key, which would
            // make existing ciphertext undecryptable.
            tracing::warn!(
                target: "audit",
                source = "backend",
                category = "system",
                action = "keyring_unavailable",
                error_message = %crate::error::internal_detail(&e),
            );
            return match file_key {
                Some(k) => Ok((k, KeyOutcome::FileFallback)),
                None => {
                    let k = random_key()?;
                    write_file_key(key_path, &k)?;
                    Ok((k, KeyOutcome::FileFallback))
                }
            };
        }
    }

    match file_key {
        // Migrate: the file holds the key that encrypted existing rows, so
        // adopt it. Only remove the file once the store has accepted it.
        Some(k) => {
            store.set(&k)?;
            remove_file_quietly(key_path);
            Ok((k, KeyOutcome::MigratedFromFile))
        }
        None => {
            let k = random_key()?;
            store.set(&k)?;
            Ok((k, KeyOutcome::CreatedInCredentialStore))
        }
    }
}

/// Load the master key using the real OS credential store.
pub fn load_or_create(key_path: &Path) -> Result<([u8; KEY_LEN], KeyOutcome), String> {
    resolve_with(&OsCredentialStore::app(), key_path, env_forces_file())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// In-memory stand-in for the OS store.
    struct MockStore {
        value: RefCell<Option<[u8; KEY_LEN]>>,
        fails: bool,
        sets: RefCell<usize>,
    }

    impl MockStore {
        fn empty() -> Self {
            Self {
                value: RefCell::new(None),
                fails: false,
                sets: RefCell::new(0),
            }
        }
        fn with(key: [u8; KEY_LEN]) -> Self {
            Self {
                value: RefCell::new(Some(key)),
                fails: false,
                sets: RefCell::new(0),
            }
        }
        fn broken() -> Self {
            Self {
                value: RefCell::new(None),
                fails: true,
                sets: RefCell::new(0),
            }
        }
    }

    impl CredentialStore for MockStore {
        fn get(&self) -> Result<Option<[u8; KEY_LEN]>, String> {
            if self.fails {
                return Err("store unavailable".to_string());
            }
            Ok(*self.value.borrow())
        }
        fn set(&self, key: &[u8; KEY_LEN]) -> Result<(), String> {
            if self.fails {
                return Err("store unavailable".to_string());
            }
            *self.sets.borrow_mut() += 1;
            *self.value.borrow_mut() = Some(*key);
            Ok(())
        }
    }

    fn tmp_path(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("lexio-key-store-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join(name);
        std::fs::remove_file(&p).ok();
        p
    }

    #[test]
    fn base64_key_round_trips() {
        let key = [7u8; KEY_LEN];
        assert_eq!(key_from_b64(&b64_key(&key)).unwrap(), key);
    }

    #[test]
    fn rejects_wrong_length_and_garbage() {
        assert!(key_from_b64(&B64.encode([0u8; 16])).is_err(), "short key");
        assert!(key_from_b64(&B64.encode([0u8; 33])).is_err(), "long key");
        assert!(key_from_b64("not base64!!").is_err(), "garbage");
    }

    #[test]
    fn file_key_round_trips() {
        let path = tmp_path("roundtrip.key");
        let key = [3u8; KEY_LEN];
        write_file_key(&path, &key).unwrap();
        assert_eq!(read_file_key(&path).unwrap(), Some(key));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn absent_file_is_not_an_error() {
        let path = tmp_path("absent.key");
        assert_eq!(read_file_key(&path).unwrap(), None);
    }

    /// A truncated key must surface as an error, not as a fresh key: replacing
    /// it would make every previously encrypted secret undecryptable.
    #[test]
    fn corrupt_file_is_rejected() {
        let path = tmp_path("corrupt.key");
        std::fs::write(&path, [1u8; 5]).unwrap();
        assert!(read_file_key(&path).is_err());
        std::fs::remove_file(&path).ok();
    }

    // ── The migration state machine ──

    /// The upgrade path: an old install has the file, the store is empty.
    #[test]
    fn migrates_file_key_into_store_and_deletes_file() {
        let path = tmp_path("migrate.key");
        let legacy = [5u8; KEY_LEN];
        write_file_key(&path, &legacy).unwrap();
        let store = MockStore::empty();

        let (key, outcome) = resolve_with(&store, &path, false).unwrap();

        assert_eq!(key, legacy, "must adopt the existing key, not generate one");
        assert_eq!(outcome, KeyOutcome::MigratedFromFile);
        assert_eq!(
            *store.value.borrow(),
            Some(legacy),
            "key landed in the store"
        );
        assert!(!path.exists(), "plaintext copy must be gone");
    }

    #[test]
    fn first_run_creates_key_in_store_without_writing_a_file() {
        let path = tmp_path("fresh.key");
        let store = MockStore::empty();

        let (key, outcome) = resolve_with(&store, &path, false).unwrap();

        assert_eq!(outcome, KeyOutcome::CreatedInCredentialStore);
        assert_eq!(*store.value.borrow(), Some(key));
        assert!(!path.exists(), "no plaintext file may be written");
    }

    #[test]
    fn reads_existing_store_key() {
        let path = tmp_path("existing.key");
        let stored = [9u8; KEY_LEN];
        let store = MockStore::with(stored);

        let (key, outcome) = resolve_with(&store, &path, false).unwrap();

        assert_eq!(key, stored);
        assert_eq!(outcome, KeyOutcome::CredentialStore);
        assert_eq!(*store.sets.borrow(), 0, "must not rewrite an existing key");
    }

    /// Same key in both places after an interrupted migration: the file is
    /// redundant and must be removed.
    #[test]
    fn removes_redundant_file_when_keys_match() {
        let path = tmp_path("redundant.key");
        let key = [4u8; KEY_LEN];
        write_file_key(&path, &key).unwrap();
        let store = MockStore::with(key);

        let (got, outcome) = resolve_with(&store, &path, false).unwrap();

        assert_eq!(got, key);
        assert_eq!(outcome, KeyOutcome::RedundantFileRemoved);
        assert!(!path.exists(), "matching plaintext copy should be removed");
    }

    /// Conflicting keys: the store wins, but the file must survive because it
    /// may be the only copy of a key that encrypted existing rows.
    #[test]
    fn keeps_conflicting_file_and_prefers_store() {
        let path = tmp_path("conflict.key");
        write_file_key(&path, &[1u8; KEY_LEN]).unwrap();
        let store = MockStore::with([2u8; KEY_LEN]);

        let (key, outcome) = resolve_with(&store, &path, false).unwrap();

        assert_eq!(key, [2u8; KEY_LEN], "store is authoritative");
        assert_eq!(outcome, KeyOutcome::StorePreferredOverConflictingFile);
        assert!(path.exists(), "a differing key must not be deleted");
    }

    /// A broken store must not cause a new key to be generated when the file
    /// still holds the real one — that would orphan existing ciphertext.
    #[test]
    fn falls_back_to_file_when_store_is_unusable() {
        let path = tmp_path("fallback.key");
        let legacy = [6u8; KEY_LEN];
        write_file_key(&path, &legacy).unwrap();

        let (key, outcome) = resolve_with(&MockStore::broken(), &path, false).unwrap();

        assert_eq!(key, legacy, "must reuse the file key, not generate one");
        assert_eq!(outcome, KeyOutcome::FileFallback);
        assert!(
            outcome.is_plaintext_file(),
            "must be reported as file-backed"
        );
    }

    #[test]
    fn forced_file_mode_creates_and_reads_file() {
        let path = tmp_path("forced.key");
        let store = MockStore::empty();

        let (key, outcome) = resolve_with(&store, &path, true).unwrap();
        assert_eq!(outcome, KeyOutcome::FileForced);
        assert!(path.exists(), "forced mode writes the file");
        assert!(
            store.value.borrow().is_none(),
            "forced mode must not touch the store"
        );

        let (again, _) = resolve_with(&store, &path, true).unwrap();
        assert_eq!(again, key, "second run reuses the same key");
        std::fs::remove_file(&path).ok();
    }

    /// Only the two file-backed outcomes should be flagged as insecure.
    #[test]
    fn only_file_outcomes_are_flagged_plaintext() {
        assert!(KeyOutcome::FileFallback.is_plaintext_file());
        assert!(KeyOutcome::FileForced.is_plaintext_file());
        for o in [
            KeyOutcome::CredentialStore,
            KeyOutcome::MigratedFromFile,
            KeyOutcome::CreatedInCredentialStore,
            KeyOutcome::RedundantFileRemoved,
            KeyOutcome::StorePreferredOverConflictingFile,
        ] {
            assert!(!o.is_plaintext_file(), "{o:?} uses the credential store");
        }
    }

    // ── Real OS credential store ──
    //
    // Uses a dedicated service/account namespace and always cleans up, so it
    // never reads or removes the entry the application itself depends on.

    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    mod os_store {
        use super::*;

        fn test_store() -> OsCredentialStore {
            OsCredentialStore::new("com.maniact.lexio.test", "master-key-test")
        }

        /// Round-trips a key through the real credential store, and proves the
        /// full migration path lands the key there and removes the file.
        #[test]
        fn real_store_round_trip_and_file_migration() {
            let store = test_store();
            store.delete().ok();

            // Start clean: no entry, no file -> key is created in the store.
            let path = tmp_path("os-fresh.key");
            let (created, outcome) = resolve_with(&store, &path, false).unwrap();
            if outcome == KeyOutcome::FileFallback {
                // No usable store in this environment (e.g. headless Linux):
                // the fallback behaviour is covered by the mock tests.
                eprintln!("skipping: no OS credential store available here");
                std::fs::remove_file(&path).ok();
                return;
            }
            assert_eq!(outcome, KeyOutcome::CreatedInCredentialStore);
            assert!(!path.exists(), "no plaintext file may be written");

            // The value must actually be readable back from the OS store.
            assert_eq!(
                store.get().unwrap(),
                Some(created),
                "key must round-trip through the OS credential store"
            );
            assert_eq!(store.get().unwrap(), Some(created), "and be stable");

            // A restart reads the same key from the store.
            let (again, outcome2) = resolve_with(&store, &path, false).unwrap();
            assert_eq!(again, created);
            assert_eq!(outcome2, KeyOutcome::CredentialStore);

            // Migration: drop the store entry and seed a file instead.
            store.delete().unwrap();
            let legacy = [8u8; KEY_LEN];
            write_file_key(&path, &legacy).unwrap();
            let (migrated, outcome3) = resolve_with(&store, &path, false).unwrap();
            assert_eq!(migrated, legacy, "must adopt the legacy file key");
            assert_eq!(outcome3, KeyOutcome::MigratedFromFile);
            assert!(
                !path.exists(),
                "plaintext file must be removed after migration"
            );
            assert_eq!(
                store.get().unwrap(),
                Some(legacy),
                "migrated key must be in the OS store"
            );

            store.delete().ok();
        }

        /// The application's own credential must not be disturbed by tests.
        #[test]
        fn app_entry_is_namespaced_separately() {
            let app = OsCredentialStore::app();
            let test = test_store();
            assert_ne!(
                app.service, test.service,
                "tests must not touch the app entry"
            );
            assert_ne!(app.account, test.account);
        }
    }
}
