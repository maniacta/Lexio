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
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, name, base_url, api_key, api_format, is_preset, is_default, created_at FROM model_providers ORDER BY created_at ASC"
    ).map_err(|e| e.to_string())?;
    let providers: Vec<ModelProvider> = stmt
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

    let mut result = Vec::new();
    for p in providers {
        let models = list_models_by_provider(db, &p.id)?;
        let api_key_masked = if p.api_key.len() > 4 {
            format!("sk-****{}", &p.api_key[p.api_key.len()-4..])
        } else if p.api_key.is_empty() {
            String::new()
        } else {
            "****".to_string()
        };
        result.push(ProviderWithModels {
            id: p.id, name: p.name, base_url: p.base_url,
            api_key: api_key_masked, api_format: p.api_format,
            is_preset: p.is_preset, is_default: p.is_default,
            created_at: p.created_at, models,
        });
    }
    Ok(result)
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
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let id = new_id();
    let api_format = req.api_format.clone().unwrap_or_else(|| "openai_compatible".to_string());
    let now = chrono::Utc::now().to_rfc3339();
    let is_preset = false;
    // If this is the first provider, make it default
    let count: i32 = conn.query_row("SELECT COUNT(*) FROM model_providers", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let is_default = count == 0;
    conn.execute(
        "INSERT INTO model_providers (id, name, base_url, api_key, api_format, is_preset, is_default, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![id, req.name, req.base_url, req.api_key, api_format, is_preset as i32, is_default as i32, now],
    ).map_err(|e| e.to_string())?;
    Ok(ModelProvider { id, name: req.name.clone(), base_url: req.base_url.clone(), api_key: req.api_key.clone(), api_format, is_preset, is_default, created_at: now })
}

pub fn update_provider(db: &Database, id: &str, req: &UpdateProviderRequest) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let api_format = req.api_format.clone().unwrap_or_else(|| "openai_compatible".to_string());
    if req.is_default == Some(true) {
        conn.execute("UPDATE model_providers SET is_default = 0", [])
            .map_err(|e| e.to_string())?;
    }
    conn.execute(
        "UPDATE model_providers SET name=?1, base_url=?2, api_key=?3, api_format=?4 WHERE id=?5",
        rusqlite::params![req.name, req.base_url, req.api_key, api_format, id],
    ).map_err(|e| e.to_string())?;
    if let Some(is_def) = req.is_default {
        conn.execute("UPDATE model_providers SET is_default = ?1 WHERE id = ?2",
            rusqlite::params![is_def as i32, id],
        ).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn delete_provider(db: &Database, id: &str) -> Result<(), String> {
    let p = get_provider(db, id)?.ok_or("Provider not found")?;
    if p.is_preset {
        return Err("Cannot delete preset provider".to_string());
    }
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM provider_models WHERE provider_id = ?1", [id])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM model_providers WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
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
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let id = new_id();
    let temp = req.temperature.unwrap_or(0.7);
    let tokens = req.max_tokens.unwrap_or(4096);
    let is_default = if req.is_default == Some(true) {
        conn.execute("UPDATE provider_models SET is_default = 0 WHERE provider_id = ?1", [provider_id])
            .map_err(|e| e.to_string())?;
        true
    } else {
        // First model for this provider becomes default automatically
        let count: i32 = conn.query_row("SELECT COUNT(*) FROM provider_models WHERE provider_id = ?1", [provider_id], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        count == 0
    };
    conn.execute(
        "INSERT INTO provider_models (id, provider_id, model_name, temperature, max_tokens, is_default) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![id, provider_id, req.model_name, temp, tokens, is_default as i32],
    ).map_err(|e| e.to_string())?;
    Ok(ProviderModel { id, provider_id: provider_id.to_string(), model_name: req.model_name.clone(), temperature: temp, max_tokens: tokens, is_default })
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
        "UPDATE provider_models SET model_name=?1, temperature=?2, max_tokens=?3, is_default=?4 WHERE id=?5 AND provider_id=?6",
        rusqlite::params![req.model_name, temp, tokens, req.is_default.unwrap_or(false) as i32, model_id, provider_id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn delete_model(db: &Database, provider_id: &str, model_id: &str) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    // Check if referenced by task_models
    let ref_count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM task_models WHERE model_id = ?1", [model_id], |r| r.get(0)
    ).map_err(|e| e.to_string())?;
    if ref_count > 0 {
        return Err("Model is referenced by one or more task assignments. Remove the assignments first.".to_string());
    }
    // Don't delete the last model of a provider
    let count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM provider_models WHERE provider_id = ?1", [provider_id], |r| r.get(0)
    ).map_err(|e| e.to_string())?;
    if count <= 1 {
        return Err("Cannot delete the only model of a provider.".to_string());
    }
    conn.execute("DELETE FROM provider_models WHERE id = ?1 AND provider_id = ?2", [model_id, provider_id])
        .map_err(|e| e.to_string())?;
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
pub fn resolve_llm_config(db: &Database, task_name: &str) -> Result<crate::ai::llm::LlmConfig, String> {
    // Try task-specific model
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let model_id: Option<String> = conn.query_row(
        "SELECT model_id FROM task_models WHERE task_name = ?1", [task_name], |row| row.get(0)
    ).ok().flatten();

    let (provider, model) = if let Some(mid) = model_id {
        get_model_full(db, &mid)?.ok_or("Assigned model not found")?
    } else {
        get_default_model(db)?
    };

    Ok(crate::ai::llm::LlmConfig {
        base_url: provider.base_url,
        api_key: provider.api_key,
        model: model.model_name,
        temperature: model.temperature,
        max_tokens: model.max_tokens,
        api_format: provider.api_format,
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

    // DeepSeek
    let ds_id = new_id();
    conn.execute(
        "INSERT INTO model_providers (id, name, base_url, api_key, api_format, is_preset, is_default) VALUES (?1, 'DeepSeek', 'https://api.deepseek.com', '', 'openai_compatible', 1, 1)",
        rusqlite::params![ds_id],
    ).map_err(|e| e.to_string())?;
    let ds_model_id = new_id();
    conn.execute(
        "INSERT INTO provider_models (id, provider_id, model_name, temperature, max_tokens, is_default) VALUES (?1, ?2, 'deepseek-chat', 0.7, 4096, 1)",
        rusqlite::params![ds_model_id, ds_id],
    ).map_err(|e| e.to_string())?;

    // OpenAI
    let oai_id = new_id();
    conn.execute(
        "INSERT INTO model_providers (id, name, base_url, api_key, api_format, is_preset, is_default) VALUES (?1, 'OpenAI', 'https://api.openai.com/v1', '', 'openai_compatible', 1, 0)",
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
        "INSERT INTO model_providers (id, name, base_url, api_key, api_format, is_preset, is_default) VALUES (?1, 'Anthropic', 'https://api.anthropic.com', '', 'openai_compatible', 1, 0)",
        rusqlite::params![anth_id],
    ).map_err(|e| e.to_string())?;
    let anth_model_id = new_id();
    conn.execute(
        "INSERT INTO provider_models (id, provider_id, model_name, temperature, max_tokens, is_default) VALUES (?1, ?2, 'claude-sonnet-4-20250514', 0.7, 4096, 1)",
        rusqlite::params![anth_model_id, anth_id],
    ).map_err(|e| e.to_string())?;

    // Default general settings
    conn.execute(
        "INSERT INTO settings (key, value) VALUES ('theme', 'system'), ('language', 'zh'), ('data_path', ''), ('search_enabled', 'false')",
        [],
    ).map_err(|e| e.to_string())?;

    Ok(())
}
