use crate::db::Database;
use crate::models::{
    new_id, ModelProvider, CreateProviderRequest, UpdateProviderRequest,
    ProviderModel, CreateModelRequest, UpdateModelRequest,
    TaskModelMapping, SetTaskModelRequest, ProviderWithModels,
};

// ── General Settings ──

pub fn get_all_settings(db: &Database) -> Result<Vec<(String, String)>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT key, value FROM settings")
        .map_err(|e| e.to_string())?;
    let rows: Vec<(String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}

pub fn get_setting(db: &Database, key: &str) -> Option<String> {
    let conn = db.conn.lock().ok()?;
    conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| row.get(0)).ok()
}

pub fn set_settings(db: &Database, entries: &[(String, String)]) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    for (key, value) in entries {
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value=?2",
            rusqlite::params![key, value],
        ).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ── Providers ──

pub fn list_providers(db: &Database) -> Result<Vec<ProviderWithModels>, String> {
    let providers: Vec<ModelProvider> = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare(
            "SELECT id, name, base_url, api_key, api_format, is_preset, is_default, created_at FROM model_providers ORDER BY created_at ASC"
        ).map_err(|e| e.to_string())?;
        let result: Vec<ModelProvider> = stmt
            .query_map([], |row| {
                Ok(ModelProvider {
                    id: row.get(0)?, name: row.get(1)?, base_url: row.get(2)?,
                    api_key: row.get(3)?, api_format: row.get(4)?,
                    is_preset: row.get::<_, i32>(5)? != 0,
                    is_default: row.get::<_, i32>(6)? != 0,
                    created_at: row.get(7)?,
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        result
    };

    let mut result = Vec::new();
    for p in providers {
        let models = list_models_by_provider(db, &p.id)?;
        let plain = crate::crypto::decrypt_secret(&p.api_key)?;
        result.push(ProviderWithModels {
            id: p.id,
            name: p.name,
            base_url: p.base_url,
            api_key: mask_api_key(&plain),
            api_format: p.api_format,
            is_preset: p.is_preset,
            is_default: p.is_default,
            created_at: p.created_at,
            models,
        });
    }
    Ok(result)
}

fn mask_api_key(api_key: &str) -> String {
    if api_key.is_empty() {
        String::new()
    } else if api_key.len() > 4 {
        format!("sk-****{}", &api_key[api_key.len() - 4..])
    } else {
        "****".to_string()
    }
}

pub fn get_provider(db: &Database, id: &str) -> Result<Option<ModelProvider>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, name, base_url, api_key, api_format, is_preset, is_default, created_at FROM model_providers WHERE id = ?1"
    ).map_err(|e| e.to_string())?;
    let mut rows = stmt.query_map([id], |row| {
        Ok(ModelProvider {
            id: row.get(0)?, name: row.get(1)?, base_url: row.get(2)?,
            api_key: row.get(3)?, api_format: row.get(4)?,
            is_preset: row.get::<_, i32>(5)? != 0,
            is_default: row.get::<_, i32>(6)? != 0,
            created_at: row.get(7)?,
        })
    }).map_err(|e| e.to_string())?;
    Ok(rows.next().and_then(|r| r.ok()))
}

pub fn create_provider(db: &Database, req: &CreateProviderRequest) -> Result<ModelProvider, String> {
    let kind = crate::ai::ProviderKind::parse(&req.kind)
        .ok_or_else(|| "请选择已支持的厂商类型：deepseek / openai / anthropic".to_string())?;
    let base_url = kind.normalize_base_url(req.base_url.as_deref().unwrap_or(""))?;
    create_provider_by_kind(
        db,
        kind,
        &req.api_key,
        &base_url,
        req.set_default.unwrap_or(false),
    )
}

/// Create a provider from a known kind, seeding its catalog models.
pub fn create_provider_by_kind(
    db: &Database,
    kind: crate::ai::ProviderKind,
    api_key: &str,
    base_url: &str,
    set_default: bool,
) -> Result<ModelProvider, String> {
    let url = kind.normalize_base_url(base_url)?;
    let api_key = api_key.trim();
    if api_key.is_empty() {
        return Err("MISSING_API_KEY: 请填写 API Key".into());
    }
    if set_default && !kind.is_implemented() {
        return Err(format!(
            "「{}」调用尚未接入，不能设为默认厂商",
            kind.display_name()
        ));
    }
    // Encrypt before taking the DB lock — avoids holding the mutex across crypto I/O.
    let stored_key = crate::crypto::encrypt_secret(api_key)?;

    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let exists: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM model_providers WHERE api_format = ?1",
            [kind.as_str()],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if exists > 0 {
        return Err(format!("「{}」已存在，请直接编辑现有配置", kind.display_name()));
    }

    let id = new_id();
    let now = chrono::Utc::now().to_rfc3339();
    let count: i32 = conn
        .query_row("SELECT COUNT(*) FROM model_providers", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let is_default = set_default || count == 0;
    if is_default {
        conn.execute("UPDATE model_providers SET is_default = 0", [])
            .map_err(|e| e.to_string())?;
    }

    let name = kind.display_name();
    let api_format = kind.as_str();

    conn.execute(
        "INSERT INTO model_providers (id, name, base_url, api_key, api_format, is_preset, is_default, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?7)",
        rusqlite::params![id, name, url, stored_key, api_format, is_default as i32, now],
    )
    .map_err(|e| e.to_string())?;

    for m in kind.default_models() {
        let mid = new_id();
        conn.execute(
            "INSERT INTO provider_models (id, provider_id, model_name, temperature, max_tokens, is_default)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                mid,
                id,
                m.model_name,
                m.temperature,
                m.max_tokens,
                m.is_default as i32
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(ModelProvider {
        id,
        name: name.to_string(),
        base_url: url,
        api_key: mask_api_key(api_key),
        api_format: api_format.to_string(),
        is_preset: false,
        is_default,
        created_at: now,
    })
}

pub fn update_provider(db: &Database, id: &str, req: &UpdateProviderRequest) -> Result<(), String> {
    let existing = get_provider(db, id)?.ok_or_else(|| "厂商不存在".to_string())?;
    let kind = crate::ai::ProviderKind::parse(&existing.api_format)
        .ok_or_else(|| format!("未知厂商类型: {}", existing.api_format))?;
    let url = kind.normalize_base_url(&req.base_url)?;

    if req.is_default == Some(true) && !kind.is_implemented() {
        return Err(format!(
            "「{}」调用尚未接入，不能设为默认厂商",
            kind.display_name()
        ));
    }

    // Encrypt before lock so a crypto failure doesn't hold the mutex.
    let new_key = match req.api_key.as_deref().map(str::trim) {
        Some(key) if !key.is_empty() => Some(crate::crypto::encrypt_secret(key)?),
        _ => None,
    };

    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    if req.is_default == Some(true) {
        conn.execute("UPDATE model_providers SET is_default = 0", [])
            .map_err(|e| e.to_string())?;
    }

    // Kind (api_format) is immutable — only name/url/key/default may change
    if let Some(stored_key) = new_key {
        conn.execute(
            "UPDATE model_providers SET base_url=?1, api_key=?2 WHERE id=?3",
            rusqlite::params![url, stored_key, id],
        )
        .map_err(|e| e.to_string())?;
    } else {
        conn.execute(
            "UPDATE model_providers SET base_url=?1 WHERE id=?2",
            rusqlite::params![url, id],
        )
        .map_err(|e| e.to_string())?;
    }

    if let Some(ref name) = req.name {
        if !name.trim().is_empty() {
            conn.execute(
                "UPDATE model_providers SET name=?1 WHERE id=?2",
                rusqlite::params![name, id],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    if let Some(is_def) = req.is_default {
        conn.execute(
            "UPDATE model_providers SET is_default = ?1 WHERE id = ?2",
            rusqlite::params![is_def as i32, id],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn delete_provider(db: &Database, id: &str) -> Result<(), String> {
    let p = get_provider(db, id)?.ok_or("Provider not found")?;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Clean up task_models referencing models of this provider
    conn.execute(
        "DELETE FROM task_models WHERE model_id IN (SELECT id FROM provider_models WHERE provider_id = ?1)",
        [id],
    ).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM provider_models WHERE provider_id = ?1", [id])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM model_providers WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;

    // If we removed the default provider, promote another one when any remain
    if p.is_default {
        let next: Result<String, _> = conn.query_row(
            "SELECT id FROM model_providers ORDER BY created_at ASC LIMIT 1",
            [],
            |r| r.get(0),
        );
        if let Ok(new_default_id) = next {
            conn.execute(
                "UPDATE model_providers SET is_default = 1 WHERE id = ?1",
                [&new_default_id],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

// ── Provider Models ──

fn list_models_by_provider(db: &Database, provider_id: &str) -> Result<Vec<ProviderModel>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, provider_id, model_name, temperature, max_tokens, is_default FROM provider_models WHERE provider_id = ?1 ORDER BY model_name ASC"
    ).map_err(|e| e.to_string())?;
    let models: Vec<ProviderModel> = stmt
        .query_map([provider_id], |row| {
            Ok(ProviderModel {
                id: row.get(0)?, provider_id: row.get(1)?, model_name: row.get(2)?,
                temperature: row.get(3)?, max_tokens: row.get(4)?,
                is_default: row.get::<_, i32>(5)? != 0,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(models)
}

pub fn create_model(db: &Database, provider_id: &str, req: &CreateModelRequest) -> Result<ProviderModel, String> {
    let provider = get_provider(db, provider_id)?.ok_or_else(|| "厂商不存在".to_string())?;
    let kind = crate::ai::ProviderKind::parse(&provider.api_format)
        .ok_or_else(|| format!("未知厂商类型: {}", provider.api_format))?;
    let model_name = req.model_name.trim();
    if model_name.is_empty() {
        return Err("请选择模型".into());
    }
    let preset = kind
        .default_models()
        .iter()
        .find(|m| m.model_name == model_name)
        .ok_or_else(|| {
            format!(
                "「{}」不是 {} 官方支持的模型。可选：{}",
                model_name,
                kind.display_name(),
                kind.default_models()
                    .iter()
                    .map(|m| m.model_name)
                    .collect::<Vec<_>>()
                    .join("、")
            )
        })?;
    // Temperature / max_tokens are no longer user-configurable; keep DB columns for
    // schema compat and seed from the vendor catalog defaults.
    let temp = preset.temperature;
    let tokens = preset.max_tokens;

    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let dup: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM provider_models WHERE provider_id = ?1 AND model_name = ?2",
            rusqlite::params![provider_id, model_name],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if dup > 0 {
        return Err(format!("模型「{}」已添加", model_name));
    }

    let id = new_id();
    let is_default = if req.is_default == Some(true) {
        conn.execute("UPDATE provider_models SET is_default = 0 WHERE provider_id = ?1", [provider_id])
            .map_err(|e| e.to_string())?;
        true
    } else {
        let count: i32 = conn.query_row("SELECT COUNT(*) FROM provider_models WHERE provider_id = ?1", [provider_id], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        count == 0
    };
    conn.execute(
        "INSERT INTO provider_models (id, provider_id, model_name, temperature, max_tokens, is_default) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![id, provider_id, model_name, temp, tokens, is_default as i32],
    ).map_err(|e| e.to_string())?;
    Ok(ProviderModel {
        id,
        provider_id: provider_id.to_string(),
        model_name: model_name.to_string(),
        temperature: temp,
        max_tokens: tokens,
        is_default,
    })
}

/// Mark a model as the provider default without touching other fields.
pub fn set_model_default(db: &Database, provider_id: &str, model_id: &str) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let exists: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM provider_models WHERE id = ?1 AND provider_id = ?2",
            rusqlite::params![model_id, provider_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if exists == 0 {
        return Err("模型不存在".into());
    }
    conn.execute(
        "UPDATE provider_models SET is_default = 0 WHERE provider_id = ?1",
        [provider_id],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE provider_models SET is_default = 1 WHERE id = ?1 AND provider_id = ?2",
        rusqlite::params![model_id, provider_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn update_model(db: &Database, provider_id: &str, model_id: &str, req: &UpdateModelRequest) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let temp = req.temperature.unwrap_or(0.7);
    let tokens = req.max_tokens.unwrap_or(4096);
    if req.is_default == Some(true) {
        conn.execute("UPDATE provider_models SET is_default = 0 WHERE provider_id = ?1", [provider_id])
            .map_err(|e| e.to_string())?;
    }
    conn.execute(
        "UPDATE provider_models SET model_name=?1, temperature=?2, max_tokens=?3 WHERE id=?4 AND provider_id=?5",
        rusqlite::params![req.model_name, temp, tokens, model_id, provider_id],
    ).map_err(|e| e.to_string())?;
    if let Some(is_def) = req.is_default {
        conn.execute(
            "UPDATE provider_models SET is_default = ?1 WHERE id = ?2 AND provider_id = ?3",
            rusqlite::params![is_def as i32, model_id, provider_id],
        ).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn delete_model(db: &Database, provider_id: &str, model_id: &str) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    // Clear task assignments that pointed at this model
    conn.execute("DELETE FROM task_models WHERE model_id = ?1", [model_id])
        .map_err(|e| e.to_string())?;

    let is_default: i32 = conn
        .query_row(
            "SELECT is_default FROM provider_models WHERE id = ?1 AND provider_id = ?2",
            [model_id, provider_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;

    let deleted = conn
        .execute(
            "DELETE FROM provider_models WHERE id = ?1 AND provider_id = ?2",
            [model_id, provider_id],
        )
        .map_err(|e| e.to_string())?;
    if deleted == 0 {
        return Err("Model not found".to_string());
    }

    if is_default != 0 {
        let next: Result<String, _> = conn.query_row(
            "SELECT id FROM provider_models WHERE provider_id = ?1 LIMIT 1",
            [provider_id],
            |r| r.get(0),
        );
        if let Ok(next_id) = next {
            conn.execute(
                "UPDATE provider_models SET is_default = 1 WHERE id = ?1 AND provider_id = ?2",
                rusqlite::params![next_id, provider_id],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

pub fn get_model_full(db: &Database, model_id: &str) -> Result<Option<(ModelProvider, ProviderModel)>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT pm.id, pm.provider_id, pm.model_name, pm.temperature, pm.max_tokens, pm.is_default,
                mp.id, mp.name, mp.base_url, mp.api_key, mp.api_format, mp.is_preset, mp.is_default, mp.created_at
         FROM provider_models pm JOIN model_providers mp ON pm.provider_id = mp.id
         WHERE pm.id = ?1"
    ).map_err(|e| e.to_string())?;
    let mut rows = stmt.query_map([model_id], |row| {
        Ok((
            ModelProvider {
                id: row.get(6)?, name: row.get(7)?, base_url: row.get(8)?,
                api_key: row.get(9)?, api_format: row.get(10)?,
                is_preset: row.get::<_, i32>(11)? != 0,
                is_default: row.get::<_, i32>(12)? != 0,
                created_at: row.get(13)?,
            },
            ProviderModel {
                id: row.get(0)?, provider_id: row.get(1)?, model_name: row.get(2)?,
                temperature: row.get(3)?, max_tokens: row.get(4)?,
                is_default: row.get::<_, i32>(5)? != 0,
            },
        ))
    }).map_err(|e| e.to_string())?;
    Ok(rows.next().and_then(|r| r.ok()))
}

// ── Task Models ──

pub fn get_task_models(db: &Database) -> Result<Vec<TaskModelMapping>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT task_name, model_id FROM task_models ORDER BY task_name ASC"
    ).map_err(|e| e.to_string())?;
    let tasks: Vec<TaskModelMapping> = stmt
        .query_map([], |row| {
            Ok(TaskModelMapping { task_name: row.get(0)?, model_id: row.get(1)? })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(tasks)
}

pub fn set_task_model(db: &Database, task_name: &str, req: &SetTaskModelRequest) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let id = new_id();
    conn.execute(
        "INSERT INTO task_models (id, task_name, model_id) VALUES (?1, ?2, ?3)
         ON CONFLICT(task_name) DO UPDATE SET model_id=?3",
        rusqlite::params![id, task_name, req.model_id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

/// Resolve the effective LlmConfig for a task. If the task has no model assigned,
/// falls back to the default provider's default model.
pub fn resolve_llm_config(db: &Database, task_name: &str) -> Result<crate::ai::LlmConfig, String> {
    // Try task-specific model
    let model_id: Option<String> = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        conn.query_row(
            "SELECT model_id FROM task_models WHERE task_name = ?1", [task_name], |row| row.get(0)
        ).ok().flatten()
    };

    let (provider, model) = if let Some(mid) = model_id {
        get_model_full(db, &mid)?.ok_or("Assigned model not found")?
    } else {
        get_default_model(db)?
    };

    if provider.api_key.trim().is_empty() {
        return Err(format!(
            "MISSING_API_KEY: 请先在设置 → 模型厂商中为「{}」填写 API Key",
            provider.name
        ));
    }

    let kind = crate::ai::ProviderKind::parse(&provider.api_format)
        .ok_or_else(|| format!("未知厂商类型: {}", provider.api_format))?;
    let base_url = kind.normalize_base_url(&provider.base_url)?;
    let api_key = crate::crypto::decrypt_secret(&provider.api_key)?;

    Ok(crate::ai::LlmConfig {
        kind,
        base_url,
        api_key,
        model: model.model_name,
        temperature: model.temperature,
        max_tokens: model.max_tokens,
    })
}

fn get_default_model(db: &Database) -> Result<(ModelProvider, ProviderModel), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    // Get default provider
    let provider = {
        let mut stmt = conn.prepare(
            "SELECT id, name, base_url, api_key, api_format, is_preset, is_default, created_at
             FROM model_providers WHERE is_default = 1 ORDER BY created_at ASC LIMIT 1"
        ).map_err(|e| e.to_string())?;
        let mut rows = stmt.query_map([], |row| {
            Ok(ModelProvider {
                id: row.get(0)?, name: row.get(1)?, base_url: row.get(2)?,
                api_key: row.get(3)?, api_format: row.get(4)?,
                is_preset: row.get::<_, i32>(5)? != 0,
                is_default: row.get::<_, i32>(6)? != 0,
                created_at: row.get(7)?,
            })
        }).map_err(|e| e.to_string())?;
        rows.next().ok_or("No default provider configured".to_string())?.map_err(|e| e.to_string())?
    };

    // Get default model for that provider
    let mut stmt = conn.prepare(
        "SELECT id, provider_id, model_name, temperature, max_tokens, is_default
         FROM provider_models WHERE provider_id = ?1 AND is_default = 1 LIMIT 1"
    ).map_err(|e| e.to_string())?;
    let mut rows = stmt.query_map([&provider.id], |row| {
        Ok(ProviderModel {
            id: row.get(0)?, provider_id: row.get(1)?, model_name: row.get(2)?,
            temperature: row.get(3)?, max_tokens: row.get(4)?,
            is_default: row.get::<_, i32>(5)? != 0,
        })
    }).map_err(|e| e.to_string())?;
    let model = rows.next().ok_or("No default model found for the default provider".to_string())?.map_err(|e| e.to_string())?;

    Ok((provider, model))
}

// ── Preset Initialization (idempotent) ──

pub fn init_presets(db: &Database) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let count: i32 = conn.query_row("SELECT COUNT(*) FROM model_providers", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if count > 0 {
        return Ok(());
    }

    conn.execute("BEGIN", []).map_err(|e| e.to_string())?;
    let result = (|| -> Result<(), String> {
        // DeepSeek
        let ds_id = new_id();
        conn.execute(
            "INSERT INTO model_providers (id, name, base_url, api_key, api_format, is_preset, is_default) VALUES (?1, 'DeepSeek', 'https://api.deepseek.com', '', 'deepseek', 1, 1)",
            rusqlite::params![ds_id],
        ).map_err(|e| e.to_string())?;
        let ds_flash_id = new_id();
        conn.execute(
            "INSERT INTO provider_models (id, provider_id, model_name, temperature, max_tokens, is_default) VALUES (?1, ?2, 'deepseek-v4-flash', 0.7, 4096, 1)",
            rusqlite::params![ds_flash_id, ds_id],
        ).map_err(|e| e.to_string())?;
        let ds_pro_id = new_id();
        conn.execute(
            "INSERT INTO provider_models (id, provider_id, model_name, temperature, max_tokens, is_default) VALUES (?1, ?2, 'deepseek-v4-pro', 0.7, 8192, 0)",
            rusqlite::params![ds_pro_id, ds_id],
        ).map_err(|e| e.to_string())?;

        // OpenAI
        let oai_id = new_id();
        conn.execute(
            "INSERT INTO model_providers (id, name, base_url, api_key, api_format, is_preset, is_default) VALUES (?1, 'OpenAI', 'https://api.openai.com/v1', '', 'openai', 1, 0)",
            rusqlite::params![oai_id],
        ).map_err(|e| e.to_string())?;
        let oai_model_id = new_id();
        conn.execute(
            "INSERT INTO provider_models (id, provider_id, model_name, temperature, max_tokens, is_default) VALUES (?1, ?2, 'gpt-4o', 0.7, 4096, 1)",
            rusqlite::params![oai_model_id, oai_id],
        ).map_err(|e| e.to_string())?;

        // Anthropic
        let anth_id = new_id();
        conn.execute(
            "INSERT INTO model_providers (id, name, base_url, api_key, api_format, is_preset, is_default) VALUES (?1, 'Anthropic', 'https://api.anthropic.com', '', 'anthropic', 1, 0)",
            rusqlite::params![anth_id],
        ).map_err(|e| e.to_string())?;
        let anth_model_id = new_id();
        conn.execute(
            "INSERT INTO provider_models (id, provider_id, model_name, temperature, max_tokens, is_default) VALUES (?1, ?2, 'claude-sonnet-4-20250514', 0.7, 4096, 1)",
            rusqlite::params![anth_model_id, anth_id],
        ).map_err(|e| e.to_string())?;

        // Default general settings (ignore if already present)
        conn.execute(
            "INSERT OR IGNORE INTO settings (key, value) VALUES ('theme', 'system')",
            [],
        ).map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR IGNORE INTO settings (key, value) VALUES ('language', 'zh')",
            [],
        ).map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR IGNORE INTO settings (key, value) VALUES ('data_path', '')",
            [],
        ).map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR IGNORE INTO settings (key, value) VALUES ('search_enabled', 'false')",
            [],
        ).map_err(|e| e.to_string())?;

        Ok(())
    })();

    if result.is_ok() {
        conn.execute("COMMIT", []).map_err(|e| e.to_string())?;
    } else {
        conn.execute("ROLLBACK", []).map_err(|e| e.to_string())?;
    }
    result
}

/// Migrate legacy DeepSeek model IDs to current V4 API models.
/// Docs: https://api-docs.deepseek.com/zh-cn/ — deepseek-v4-flash / deepseek-v4-pro
/// Encrypt any plaintext API keys left in the DB (idempotent).
pub fn migrate_encrypt_api_keys(db: &Database) -> Result<(), String> {
    let rows: Vec<(String, String)> = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, api_key FROM model_providers")
            .map_err(|e| e.to_string())?;
        let mapped: Result<Vec<_>, _> = stmt
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
            .map_err(|e| e.to_string())?
            .collect();
        mapped.map_err(|e| e.to_string())?
    };

    for (id, key) in rows {
        if key.is_empty() || crate::crypto::is_encrypted(&key) {
            continue;
        }
        let enc = crate::crypto::encrypt_secret(&key)?;
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE model_providers SET api_key = ?1 WHERE id = ?2",
            rusqlite::params![enc, id],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn migrate_deepseek_models(db: &Database) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Normalize DeepSeek provider base URL + format
    conn.execute(
        "UPDATE model_providers
         SET base_url = 'https://api.deepseek.com', api_format = 'deepseek'
         WHERE lower(name) = 'deepseek'
            OR lower(base_url) LIKE '%deepseek.com%'",
        [],
    )
    .map_err(|e| e.to_string())?;

    // Normalize legacy openai_compatible → openai / anthropic
    conn.execute(
        "UPDATE model_providers SET api_format = 'openai'
         WHERE api_format = 'openai_compatible'
           AND (lower(name) LIKE '%openai%' OR lower(base_url) LIKE '%openai.com%')",
        [],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE model_providers SET api_format = 'anthropic'
         WHERE (api_format = 'openai_compatible' OR api_format = 'openai')
           AND (lower(name) LIKE '%anthropic%' OR lower(base_url) LIKE '%anthropic.com%')",
        [],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE model_providers SET api_format = 'openai'
         WHERE api_format = 'openai_compatible'",
        [],
    )
    .map_err(|e| e.to_string())?;

    // Retired aliases → current model IDs
    conn.execute(
        "UPDATE provider_models SET model_name = 'deepseek-v4-flash'
         WHERE model_name IN ('deepseek-chat', 'deepseek-v3', 'deepseek-v3.1', 'deepseek-v3.2')",
        [],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE provider_models SET model_name = 'deepseek-v4-pro'
         WHERE model_name IN ('deepseek-reasoner', 'deepseek-r1')",
        [],
    )
    .map_err(|e| e.to_string())?;

    // Ensure DeepSeek providers have both flash (default) and pro models
    let mut stmt = conn
        .prepare(
            "SELECT id FROM model_providers
             WHERE lower(name) = 'deepseek' OR lower(base_url) LIKE '%deepseek.com%'",
        )
        .map_err(|e| e.to_string())?;
    let provider_ids: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    drop(stmt);

    for provider_id in provider_ids {
        let has_flash: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM provider_models WHERE provider_id = ?1 AND model_name = 'deepseek-v4-flash'",
                [&provider_id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if has_flash == 0 {
            let id = new_id();
            let has_any: i32 = conn
                .query_row(
                    "SELECT COUNT(*) FROM provider_models WHERE provider_id = ?1",
                    [&provider_id],
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())?;
            conn.execute(
                "INSERT INTO provider_models (id, provider_id, model_name, temperature, max_tokens, is_default)
                 VALUES (?1, ?2, 'deepseek-v4-flash', 0.7, 4096, ?3)",
                rusqlite::params![id, provider_id, if has_any == 0 { 1 } else { 0 }],
            )
            .map_err(|e| e.to_string())?;
        }

        let has_pro: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM provider_models WHERE provider_id = ?1 AND model_name = 'deepseek-v4-pro'",
                [&provider_id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if has_pro == 0 {
            let id = new_id();
            conn.execute(
                "INSERT INTO provider_models (id, provider_id, model_name, temperature, max_tokens, is_default)
                 VALUES (?1, ?2, 'deepseek-v4-pro', 0.7, 8192, 0)",
                rusqlite::params![id, provider_id],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

pub fn resolve_for_test(db: &Database, provider_id: &str, model_name: &str) -> Result<crate::ai::LlmConfig, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Validate that model_name exists for this provider
    let model_exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM provider_models WHERE provider_id = ?1 AND model_name = ?2",
        rusqlite::params![provider_id, model_name],
        |row| row.get(0),
    ).map_err(|e| e.to_string())?;
    if !model_exists {
        return Err(format!("Model '{}' not found for provider '{}'", model_name, provider_id));
    }

    let (base_url, api_key, api_format): (String, String, String) = conn.query_row(
        "SELECT base_url, api_key, api_format FROM model_providers WHERE id = ?1",
        [provider_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).map_err(|e| e.to_string())?;

    if api_key.trim().is_empty() {
        return Err("MISSING_API_KEY: 请先填写 API Key".into());
    }

    let kind = crate::ai::ProviderKind::parse(&api_format)
        .ok_or_else(|| format!("未知厂商类型: {}", api_format))?;
    let base_url = kind.normalize_base_url(&base_url)?;
    let api_key = crate::crypto::decrypt_secret(&api_key)?;

    Ok(crate::ai::LlmConfig {
        kind,
        base_url,
        api_key,
        model: model_name.to_string(),
        temperature: 0.7,
        max_tokens: 256,
    })
}
