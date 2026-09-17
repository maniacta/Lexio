//! At-rest encryption for API keys using AES-256-GCM.
//! The master key is held by the OS credential store (see [`crate::key_store`]);
//! ciphertext lives in SQLite.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const PREFIX: &str = "enc:v1:";
const NONCE_LEN: usize = 12;

static MASTER_KEY: OnceLock<[u8; 32]> = OnceLock::new();

fn fill_random(buf: &mut [u8]) -> Result<(), String> {
    getrandom::fill(buf).map_err(crate::error::internal)
}

/// How the master key was obtained on this run.
///
/// Recorded here and emitted to the audit log by
/// [`log_master_key_provenance`] once the subscriber is installed, because the
/// key is loaded before logging is initialized.
static KEY_OUTCOME: std::sync::OnceLock<crate::key_store::KeyOutcome> = std::sync::OnceLock::new();

/// Load or create the 32-byte master key.
///
/// Held by the OS credential store when one is available; see
/// [`crate::key_store`] for the fallback rules.
pub fn init_master_key(db_path: &str) -> Result<(), String> {
    let key_path = master_key_path(db_path);
    let (key, outcome) = crate::key_store::load_or_create(&key_path)?;
    let _ = KEY_OUTCOME.set(outcome);
    let _ = MASTER_KEY.set(key);
    Ok(())
}

/// Emit how the master key was stored, once the audit subscriber exists.
///
/// Silently falling back to a plaintext file is exactly the condition this
/// change exists to avoid, so it is reported at `warn`; a normal credential
/// store load is reported at `info`.
pub fn log_master_key_provenance() {
    let Some(outcome) = KEY_OUTCOME.get().copied() else {
        return;
    };
    tracing::info!(
        target: "audit",
        source = "backend",
        category = "system",
        action = "master_key_loaded",
        result_summary = ?outcome,
    );
    if outcome.is_plaintext_file() {
        tracing::warn!(
            target: "audit",
            source = "backend",
            category = "system",
            action = "master_key_stored_in_plaintext_file",
            result_summary = ?outcome,
        );
    }
}

fn master_key_path(db_path: &str) -> PathBuf {
    let p = Path::new(db_path);
    match p.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir.join(".lexio-master.key"),
        _ => PathBuf::from(".lexio-master.key"),
    }
}

fn cipher() -> Result<Aes256Gcm, String> {
    let key = MASTER_KEY
        .get()
        .ok_or_else(|| "主密钥未初始化".to_string())?;
    let key = Key::<Aes256Gcm>::from_slice(key);
    Ok(Aes256Gcm::new(key))
}

/// Encrypt a secret for DB storage. Empty string stays empty.
pub fn encrypt_secret(plain: &str) -> Result<String, String> {
    if plain.is_empty() {
        return Ok(String::new());
    }
    if plain.starts_with(PREFIX) {
        // Re-saving an already-valid ciphertext is a no-op. A string that only
        // *looks* like our envelope would otherwise be stored forever and then
        // fail to decrypt on every read.
        return match decrypt_secret(plain) {
            Ok(_) => Ok(plain.to_string()),
            Err(_) => Err("请粘贴明文 API Key，不要粘贴无法解密的 enc:v1: 密文".into()),
        };
    }
    let cipher = cipher()?;
    let mut nonce_bytes = [0u8; NONCE_LEN];
    fill_random(&mut nonce_bytes)?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plain.as_bytes())
        .map_err(crate::error::internal)?;
    let mut packed = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    packed.extend_from_slice(&nonce_bytes);
    packed.extend_from_slice(&ciphertext);
    Ok(format!("{PREFIX}{}", B64.encode(packed)))
}

/// Decrypt a secret from DB. Plaintext legacy values are returned as-is.
pub fn decrypt_secret(stored: &str) -> Result<String, String> {
    if stored.is_empty() {
        return Ok(String::new());
    }
    if !stored.starts_with(PREFIX) {
        return Ok(stored.to_string());
    }
    let cipher = cipher()?;
    let raw = B64
        .decode(stored[PREFIX.len()..].as_bytes())
        .map_err(crate::error::internal)?;
    if raw.len() <= NONCE_LEN {
        return Err("密文过短".into());
    }
    let (nonce_bytes, ciphertext) = raw.split_at(NONCE_LEN);
    let nonce = Nonce::from_slice(nonce_bytes);
    let plain = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| "解密失败：主密钥可能已更换".to_string())?;
    String::from_utf8(plain).map_err(|_| "解密结果不是合法 UTF-8".to_string())
}

pub fn is_encrypted(stored: &str) -> bool {
    stored.starts_with(PREFIX)
}

pub fn generate_api_token() -> Result<String, String> {
    let mut bytes = [0u8; 32];
    fill_random(&mut bytes)?;
    Ok(B64.encode(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_api_token_is_32_random_bytes() {
        let token = generate_api_token().expect("rng");
        let raw = B64.decode(token.as_bytes()).expect("base64");
        assert_eq!(raw.len(), 32);
        assert!(
            raw.iter().any(|&b| b != 0),
            "token must not be the all-zero fallback that used to leak on rng failure"
        );
        let other = generate_api_token().expect("rng");
        assert_ne!(token, other);
    }

    fn ensure_test_master_key() {
        let mut key = [0u8; 32];
        key[0] = 0x5a;
        let _ = MASTER_KEY.set(key);
    }

    #[test]
    fn encrypt_secret_round_trips() {
        ensure_test_master_key();
        let stored = encrypt_secret("sk-test-key").unwrap();
        assert!(stored.starts_with(PREFIX));
        assert_eq!(decrypt_secret(&stored).unwrap(), "sk-test-key");
    }

    #[test]
    fn encrypt_secret_keeps_valid_ciphertext() {
        ensure_test_master_key();
        let stored = encrypt_secret("sk-test-key").unwrap();
        assert_eq!(encrypt_secret(&stored).unwrap(), stored);
    }

    #[test]
    fn encrypt_secret_rejects_forged_ciphertext() {
        ensure_test_master_key();
        let err = encrypt_secret("enc:v1:not-a-real-blob").unwrap_err();
        assert!(
            err.contains("enc:v1:"),
            "forged envelope must be rejected, got: {err}"
        );
    }
}
