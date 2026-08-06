//! At-rest encryption for API keys using AES-256-GCM.
//! Master key lives next to the DB (machine-local); ciphertext in SQLite.

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
    getrandom::getrandom(buf).map_err(|e| format!("随机数生成失败: {e}"))
}

/// Load or create the 32-byte master key beside the database file.
pub fn init_master_key(db_path: &str) -> Result<(), String> {
    let key_path = master_key_path(db_path);
    let key = if key_path.exists() {
        let bytes = std::fs::read(&key_path).map_err(|e| format!("读取主密钥失败: {e}"))?;
        if bytes.len() != 32 {
            return Err("主密钥文件损坏，请删除后重新填写 API Key".into());
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        arr
    } else {
        let mut arr = [0u8; 32];
        fill_random(&mut arr)?;
        if let Some(parent) = key_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&key_path, arr).map_err(|e| format!("写入主密钥失败: {e}"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600));
        }
        arr
    };
    let _ = MASTER_KEY.set(key);
    Ok(())
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
        return Ok(plain.to_string());
    }
    let cipher = cipher()?;
    let mut nonce_bytes = [0u8; NONCE_LEN];
    fill_random(&mut nonce_bytes)?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plain.as_bytes())
        .map_err(|e| format!("加密失败: {e}"))?;
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
        .map_err(|e| format!("密文解码失败: {e}"))?;
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

pub fn generate_api_token() -> String {
    let mut bytes = [0u8; 32];
    let _ = fill_random(&mut bytes);
    B64.encode(bytes)
}
