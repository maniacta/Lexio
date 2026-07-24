# 设置界面 & 模型配置 实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 为 Lexio 增加设置界面，支持配置模型厂商、模型参数、任务-模型映射，以及通用应用设置。

**架构：** 在 SQLite 中新增 4 张表（settings、model_providers、provider_models、task_models）。后端提供 REST API 进行 CRUD。LlmClient 从全局单例改为按请求从数据库动态 resolve 配置。前端新增 SettingsView 组件（3 标签页），通过侧栏底部按钮进入。

**技术栈：** Rust (Axum + rusqlite), React 19 + TypeScript + Vite 7, 沿用现有 CSS 设计令牌

**设计规格：** `docs/superpowers/specs/2026-07-24-lexio-settings-design.md`

---

## 新增/修改的文件清单

### Rust 后端 (`src-tauri/src/`)
| 文件 | 职责 |
|------|------|
| `db.rs` | 新增 4 张表的 migration |
| `models.rs` | 新增 settings 相关 struct |
| `repo/settings.rs` | 🆕 设置层 repository（CRUD + 预设初始化） |
| `repo/mod.rs` | 添加 `pub mod settings;` |
| `api/settings.rs` | 🆕 设置 REST API 端点 |
| `api/mod.rs` | 添加 `pub mod settings;` |
| `api/ai_routes.rs` | 重构：从 DB resolve LlmClient 替代全局单例 |
| `ai/llm.rs` | LlmConfig 增加 temperature、max_tokens、api_format 字段 |
| `ai/extract.rs` | 参数改为接收 LlmConfig（不持有 client） |
| `ai/quiz_gen.rs` | 同上 |
| `server.rs` | 注册 settings 路由，CORS 增加 PUT/DELETE |
| `lib.rs` | mod declarations，启动时调用 `init_presets` |

### React 前端 (`src/`)
| 文件 | 职责 |
|------|------|
| `types.ts` | 新增 settings 相关 TypeScript 类型 |
| `api/client.ts` | 新增 settings API 方法 |
| `components/Content/SettingsView.tsx` | 🆕 设置页主组件（3 标签页） |
| `components/Content/SettingsView.css` | 🆕 设置页样式，使用项目 CSS 变量 |
| `components/Content/GeneralTab.tsx` | 🆕 通用设置标签页 |
| `components/Content/ProvidersTab.tsx` | 🆕 模型厂商标签页 |
| `components/Content/TaskModelsTab.tsx` | 🆕 任务模型标签页 |
| `components/Content.tsx` | 新增 `settings` 视图渲染 |
| `components/Layout.tsx` | 新增 `view` 状态支持 `settings` |
| `components/Sidebar.tsx` | 底部添加设置图标按钮 |
| `components/Sidebar.css` | 设置按钮样式 |

---

### 任务 1：数据库 Migration — 新增 4 张表

**文件：**
- 修改：`src-tauri/src/db.rs`

- [ ] **步骤 1：在 `migrate()` 末尾追加 4 张新表的 SQL**

在 `db.rs` 的 `migrate()` 方法中，在最后一个 FTS5 trigger SQL 的 `);"` 之后、`)?;` 之前，追加：

```sql

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS model_providers (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                base_url TEXT NOT NULL,
                api_key TEXT NOT NULL DEFAULT '',
                api_format TEXT NOT NULL DEFAULT 'openai_compatible',
                is_preset INTEGER NOT NULL DEFAULT 0,
                is_default INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS provider_models (
                id TEXT PRIMARY KEY,
                provider_id TEXT NOT NULL REFERENCES model_providers(id),
                model_name TEXT NOT NULL,
                temperature REAL NOT NULL DEFAULT 0.7,
                max_tokens INTEGER NOT NULL DEFAULT 4096,
                is_default INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS task_models (
                id TEXT PRIMARY KEY,
                task_name TEXT NOT NULL UNIQUE,
                model_id TEXT,
                FOREIGN KEY (model_id) REFERENCES provider_models(id)
            );
```

- [ ] **步骤 2：构建验证**

```bash
cd src-tauri && cargo check
```

预期：编译成功，无错误。

- [ ] **步骤 3：Commit**

```bash
git add src-tauri/src/db.rs src-tauri/Cargo.lock
git commit -m "feat: add settings, model_providers, provider_models, task_models tables"
```

---

### 任务 2：Settings 数据模型

**文件：**
- 修改：`src-tauri/src/models.rs`

- [ ] **步骤 1：在 `models.rs` 末尾追加 settings 相关 struct**

```rust
// ── Settings models ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProvider {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub api_format: String,
    pub is_preset: bool,
    pub is_default: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProviderRequest {
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub api_format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProviderRequest {
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub api_format: Option<String>,
    pub is_default: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderModel {
    pub id: String,
    pub provider_id: String,
    pub model_name: String,
    pub temperature: f64,
    pub max_tokens: i32,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateModelRequest {
    pub model_name: String,
    pub temperature: Option<f64>,
    pub max_tokens: Option<i32>,
    pub is_default: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateModelRequest {
    pub model_name: String,
    pub temperature: Option<f64>,
    pub max_tokens: Option<i32>,
    pub is_default: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskModelMapping {
    pub task_name: String,
    pub model_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetTaskModelRequest {
    pub model_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralSettings {
    pub theme: Option<String>,
    pub language: Option<String>,
    pub data_path: Option<String>,
    pub search_enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderWithModels {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub api_format: String,
    pub is_preset: bool,
    pub is_default: bool,
    pub created_at: String,
    pub models: Vec<ProviderModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsResponse {
    pub general: serde_json::Value,
    pub providers: Vec<ProviderWithModels>,
    pub task_models: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConnectionRequest {
    pub provider_id: String,
    pub model_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConnectionResponse {
    pub ok: bool,
    pub message: String,
}
```

- [ ] **步骤 2：构建验证**

```bash
cd src-tauri && cargo check
```

预期：编译成功。

- [ ] **步骤 3：Commit**

```bash
git add src-tauri/src/models.rs
git commit -m "feat: add settings data models (provider, model, task mapping)"
```

---

### 任务 3：Settings Repository

**文件：**
- 创建：`src-tauri/src/repo/settings.rs`
- 修改：`src-tauri/src/repo/mod.rs`

- [ ] **步骤 1：更新 `repo/mod.rs`**

```rust
pub mod knowledge;
pub mod source;
pub mod quiz;
pub mod learning;
pub mod relation;
pub mod settings;
```

- [ ] **步骤 2：创建 `repo/settings.rs`**

```rust
use crate::db::Database;
use crate::models::{
    self, new_id, ModelProvider, CreateProviderRequest, UpdateProviderRequest,
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
            ProviderModel {
                id: row.get(0)?, provider_id: row.get(1)?, model_name: row.get(2)?,
                temperature: row.get(3)?, max_tokens: row.get(4)?,
                is_default: row.get::<_, i32>(5)? != 0,
            },
            ModelProvider {
                id: row.get(6)?, name: row.get(7)?, base_url: row.get(8)?,
                api_key: row.get(9)?, api_format: row.get(10)?,
                is_preset: row.get::<_, i32>(11)? != 0,
                is_default: row.get::<_, i32>(12)? != 0,
                created_at: row.get(13)?,
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
```

- [ ] **步骤 3：构建验证**

```bash
cd src-tauri && cargo check
```

预期：编译成功。

- [ ] **步骤 4：Commit**

```bash
git add src-tauri/src/repo/settings.rs src-tauri/src/repo/mod.rs
git commit -m "feat: add settings repository with CRUD, resolve logic, and presets"
```

---

### 任务 4：更新 LlmClient — 增加字段，移除全局依赖

**文件：**
- 修改：`src-tauri/src/ai/llm.rs`
- 修改：`src-tauri/src/ai/extract.rs`
- 修改：`src-tauri/src/ai/quiz_gen.rs`

- [ ] **步骤 1：修改 `llm.rs` — LlmConfig 增加 3 个字段**

在 `llm.rs` 中，将 `LlmConfig`：

```rust
#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}
```

改为：

```rust
#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub temperature: f64,
    pub max_tokens: i32,
    pub api_format: String,
}
```

同时在 `ChatRequest` struct 中添加 `temperature` 和 `max_tokens` 字段：

```rust
#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<i32>,
}
```

在 `chat()` 和 `chat_streaming()` 方法中构建 `ChatRequest` 时传入这些值：

```rust
let req = ChatRequest {
    model: self.config.model.clone(),
    messages,
    stream: false,
    temperature: Some(self.config.temperature),
    max_tokens: Some(self.config.max_tokens),
};
```

`chat_streaming` 同理，`stream: true`。

- [ ] **步骤 2：修改 `extract.rs` — 函数签名改为接收 LlmConfig**

将：

```rust
use crate::ai::llm::LlmClient;
use crate::models::{CreateKnowledgePointRequest, KnowledgePoint};

pub async fn extract_knowledge_points(
    llm: &LlmClient,
    source_title: &str,
    source_content: &str,
) -> Result<Vec<CreateKnowledgePointRequest>, String> {
```

改为：

```rust
use crate::ai::llm::{LlmClient, LlmConfig};
use crate::models::{CreateKnowledgePointRequest, KnowledgePoint};

pub async fn extract_knowledge_points(
    config: LlmConfig,
    source_title: &str,
    source_content: &str,
) -> Result<Vec<CreateKnowledgePointRequest>, String> {
    let llm = LlmClient::new(config);
```

- [ ] **步骤 3：修改 `quiz_gen.rs` — 同理改为接收 LlmConfig**

将：

```rust
use crate::ai::llm::LlmClient;
use crate::models::QuizQuestion;

pub async fn generate_quizzes(
    llm: &LlmClient,
```

改为：

```rust
use crate::ai::llm::{LlmClient, LlmConfig};
use crate::models::QuizQuestion;

pub async fn generate_quizzes(
    config: LlmConfig,
```

并在函数体内 `let llm = LlmClient::new(config);`

- [ ] **步骤 4：构建验证**

```bash
cd src-tauri && cargo check
```

预期：ai_routes.rs 可能有编译错误（因为调用方式变了）——下个任务修复。

- [ ] **步骤 5：Commit**

```bash
git add src-tauri/src/ai/
git commit -m "refactor: add temperature/max_tokens/api_format to LlmConfig, accept config by value"
```

---

### 任务 5：Settings REST API 端点

**文件：**
- 创建：`src-tauri/src/api/settings.rs`
- 修改：`src-tauri/src/api/mod.rs`

- [ ] **步骤 1：更新 `api/mod.rs`**

```rust
pub mod sources;
pub mod knowledge;
pub mod quiz;
pub mod learning;
pub mod ai_routes;
pub mod settings;
```

- [ ] **步骤 2：创建 `api/settings.rs`**

```rust
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use crate::db::Database;
use crate::models::{
    CreateProviderRequest, UpdateProviderRequest,
    CreateModelRequest, UpdateModelRequest,
    SetTaskModelRequest, GeneralSettings, SettingsResponse,
    TestConnectionRequest, TestConnectionResponse, TaskModelMapping, ProviderWithModels,
};
use crate::repo;
use serde_json::json;

pub async fn get_settings(
    State(db): State<&'static Database>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let general_entries = repo::settings::get_all_settings(db)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let general: serde_json::Map<_, _> = general_entries.into_iter()
        .map(|(k, v)| (k, serde_json::Value::String(v)))
        .collect();

    let providers = repo::settings::list_providers(db)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let tasks = repo::settings::get_task_models(db)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    // Build task_models map with resolved field
    let mut task_map = serde_json::Map::new();
    for t in &["chat", "extract", "quiz_gen", "search"] {
        let mapping = tasks.iter().find(|m| m.task_name == *t);
        let resolved = if mapping.as_ref().and_then(|m| m.model_id.as_ref()).is_some() {
            let model_id = mapping.as_ref().unwrap().model_id.as_ref().unwrap();
            repo::settings::get_model_full(db, model_id)
                .ok().flatten()
                .map(|(p, m)| format!("{} / {}", p.name, m.model_name))
        } else {
            // Try to resolve default
            resolve_default_display(db).ok()
        };
        task_map.insert(t.to_string(), json!({
            "model_id": mapping.and_then(|m| m.model_id),
            "resolved": resolved,
        }));
    }

    Ok(Json(json!({
        "general": general,
        "providers": providers,
        "task_models": task_map,
    })))
}

fn resolve_default_display(db: &Database) -> Result<String, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let (p_name, m_name): (String, String) = conn.query_row(
        "SELECT mp.name, pm.model_name FROM model_providers mp
         JOIN provider_models pm ON pm.provider_id = mp.id
         WHERE mp.is_default = 1 AND pm.is_default = 1
         LIMIT 1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).map_err(|e| e.to_string())?;
    Ok(format!("{} / {}", p_name, m_name))
}

// ── Providers ──

pub async fn list_providers(
    State(db): State<&'static Database>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let providers = repo::settings::list_providers(db)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(serde_json::to_value(&providers).unwrap()))
}

pub async fn create_provider(
    State(db): State<&'static Database>,
    Json(req): Json<CreateProviderRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    let p = repo::settings::create_provider(db, &req)
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    Ok((StatusCode::CREATED, Json(serde_json::to_value(&p).unwrap())))
}

pub async fn update_provider(
    State(db): State<&'static Database>,
    Path(id): Path<String>,
    Json(req): Json<UpdateProviderRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    repo::settings::update_provider(db, &id, &req)
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    Ok(StatusCode::OK)
}

pub async fn delete_provider(
    State(db): State<&'static Database>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    repo::settings::delete_provider(db, &id)
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Models ──

pub async fn create_model(
    State(db): State<&'static Database>,
    Path(provider_id): Path<String>,
    Json(req): Json<CreateModelRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    let m = repo::settings::create_model(db, &provider_id, &req)
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    Ok((StatusCode::CREATED, Json(serde_json::to_value(&m).unwrap())))
}

pub async fn update_model(
    State(db): State<&'static Database>,
    Path((provider_id, model_id)): Path<(String, String)>,
    Json(req): Json<UpdateModelRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    repo::settings::update_model(db, &provider_id, &model_id, &req)
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    Ok(StatusCode::OK)
}

pub async fn delete_model(
    State(db): State<&'static Database>,
    Path((provider_id, model_id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, String)> {
    repo::settings::delete_model(db, &provider_id, &model_id)
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Task Models ──

pub async fn get_task_models(
    State(db): State<&'static Database>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let tasks = repo::settings::get_task_models(db)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(serde_json::to_value(&tasks).unwrap()))
}

pub async fn set_task_model(
    State(db): State<&'static Database>,
    Path(task_name): Path<String>,
    Json(req): Json<SetTaskModelRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let valid_tasks = ["chat", "extract", "quiz_gen", "search"];
    if !valid_tasks.contains(&task_name.as_str()) {
        return Err((StatusCode::BAD_REQUEST, format!("Invalid task name: {}", task_name)));
    }
    repo::settings::set_task_model(db, &task_name, &req)
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    Ok(StatusCode::OK)
}

// ── General Settings ──

pub async fn update_general(
    State(db): State<&'static Database>,
    Json(req): Json<GeneralSettings>,
) -> Result<StatusCode, (StatusCode, String)> {
    let mut entries = Vec::new();
    if let Some(v) = &req.theme { entries.push(("theme".to_string(), v.clone())); }
    if let Some(v) = &req.language { entries.push(("language".to_string(), v.clone())); }
    if let Some(v) = &req.data_path { entries.push(("data_path".to_string(), v.clone())); }
    if let Some(v) = req.search_enabled {
        entries.push(("search_enabled".to_string(), v.to_string()));
    }
    repo::settings::set_settings(db, &entries)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(StatusCode::OK)
}

// ── Test Connection ──

pub async fn test_connection(
    State(db): State<&'static Database>,
    Json(req): Json<TestConnectionRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let config = repo::settings::resolve_for_test(db, &req.provider_id, &req.model_name)
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let client = crate::ai::llm::LlmClient::new(config);
    match client.chat("You are a helpful assistant.", "Say 'hello' in one word.").await {
        Ok(resp) => Ok(Json(serde_json::to_value(&TestConnectionResponse {
            ok: true,
            message: format!("连接成功。回复: {}", resp),
        }).unwrap())),
        Err(e) => Ok(Json(serde_json::to_value(&TestConnectionResponse {
            ok: false,
            message: e,
        }).unwrap())),
    }
}
```

需要同时在 `repo/settings.rs` 添加 `resolve_for_test` 辅助函数：

```rust
pub fn resolve_for_test(db: &Database, provider_id: &str, model_name: &str) -> Result<crate::ai::llm::LlmConfig, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let (base_url, api_key, api_format): (String, String, String) = conn.query_row(
        "SELECT base_url, api_key, api_format FROM model_providers WHERE id = ?1",
        [provider_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).map_err(|e| e.to_string())?;
    Ok(crate::ai::llm::LlmConfig {
        base_url, api_key, model: model_name.to_string(),
        temperature: 0.7, max_tokens: 100, api_format,
    })
}
```

- [ ] **步骤 3：构建验证**

```bash
cd src-tauri && cargo check
```

预期：编译成功。

- [ ] **步骤 4：Commit**

```bash
git add src-tauri/src/api/settings.rs src-tauri/src/api/mod.rs src-tauri/src/repo/settings.rs
git commit -m "feat: add settings REST API endpoints"
```

---

### 任务 6：重构 AppState & 注册设置路由

**文件：**
- 修改：`src-tauri/src/api/ai_routes.rs`
- 修改：`src-tauri/src/server.rs`
- 修改：`src-tauri/src/lib.rs`

- [ ] **步骤 1：重构 `ai_routes.rs` — 移除 AppState 中的 llm 字段，按需 resolve**

删除 `AppState` 中的 `llm` 字段，改为在 handler 中 resolve：

```rust
// 将 AppState 改为：
pub struct AppState {
    pub db: &'static Database,
}
```

修改 `start_research` handler 开头，在函数开始处 resolve config：

```rust
pub async fn start_research(
    State(state): State<&'static AppState>,
    Json(req): Json<AiStartResearchRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    let llm_config = repo::settings::resolve_llm_config(state.db, "chat")
        .map_err(|e| (StatusCode::SERVICE_UNAVAILABLE, format!("No model configured: {}", e)))?;
    let llm = LlmClient::new(llm_config);
    // ... 其余代码不变，但 state.llm.xxx 改为 llm.xxx
```

同样修改 `generate_quiz` handler——resolve `"quiz_gen"` 任务：

```rust
pub async fn generate_quiz(
    State(state): State<&'static AppState>,
    Json(req): Json<GenerateQuizRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    let kp = knowledge::get_kp(state.db, &req.kp_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
        .ok_or((StatusCode::NOT_FOUND, "KP not found".to_string()))?;

    let llm_config = repo::settings::resolve_llm_config(state.db, "quiz_gen")
        .map_err(|e| (StatusCode::SERVICE_UNAVAILABLE, format!("No model configured: {}", e)))?;

    let mut questions = crate::ai::quiz_gen::generate_quizzes(llm_config, &kp.title, &kp.content, req.count).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    // ... 其余不变
```

- [ ] **步骤 2：修改 `server.rs` — 注册 settings 路由，扩展 CORS**

在 `use` 导入中添加 `settings`：

```rust
use crate::api::{sources, knowledge, quiz, learning, ai_routes, settings};
```

在路由中添加 settings 端点：

```rust
        // Settings
        .route("/api/settings", get(settings::get_settings))
        .route("/api/settings/providers",
            get(settings::list_providers).post(settings::create_provider))
        .route("/api/settings/providers/{id}",
            axum::routing::put(settings::update_provider).delete(settings::delete_provider))
        .route("/api/settings/providers/{id}/models",
            axum::routing::post(settings::create_model))
        .route("/api/settings/providers/{provider_id}/models/{model_id}",
            axum::routing::put(settings::update_model).delete(settings::delete_model))
        .route("/api/settings/tasks", get(settings::get_task_models))
        .route("/api/settings/tasks/{task_name}", axum::routing::put(settings::set_task_model))
        .route("/api/settings/general", axum::routing::put(settings::update_general))
        .route("/api/settings/test-connection", axum::routing::post(settings::test_connection))
```

CORS 增加 PUT、DELETE 方法：

```rust
.allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
```

- [ ] **步骤 3：修改 `lib.rs` — 移除环境变量 LLM 初始化，添加 presets 初始化**

删除整个 `llm_config` 和 `llm` 初始化块：

```rust
            // Initialize LLM client from environment variables
            let llm_config = ai::llm::LlmConfig {
                base_url: std::env::var("LLM_BASE_URL")
                    .unwrap_or_else(|_| "https://api.openai.com/v1".into()),
                api_key: std::env::var("LLM_API_KEY").unwrap_or_default(),
                model: std::env::var("LLM_MODEL")
                    .unwrap_or_else(|_| "gpt-4o-mini".into()),
            };
            let llm = ai::llm::LlmClient::new(llm_config);
```

替换为：

```rust
            // Initialize settings presets (idempotent)
            repo::settings::init_presets(db)
                .expect("Failed to initialize settings presets");
```

并将 `AppState` 定义改为：

```rust
            let app_state: &'static api::ai_routes::AppState =
                Box::leak(Box::new(api::ai_routes::AppState { db }));
```

- [ ] **步骤 4：构建验证**

```bash
cd src-tauri && cargo check
```

预期：编译成功。

- [ ] **步骤 5：启动验证**

```bash
cd .. && npm run dev &
sleep 3
curl -s http://localhost:14200/api/settings | head -c 200
```

预期：返回 JSON 包含 `general`、`providers`、`task_models`。

```bash
curl -s http://localhost:14200/api/health
```

预期：`OK`。

- [ ] **步骤 6：Commit**

```bash
git add src-tauri/src/api/ai_routes.rs src-tauri/src/server.rs src-tauri/src/lib.rs
git commit -m "refactor: resolve LlmClient per-request from settings, register settings routes"
```

---

### 任务 7：前端 — Types & API Client

**文件：**
- 修改：`src/types.ts`
- 修改：`src/api/client.ts`

- [ ] **步骤 1：在 `types.ts` 末尾追加 settings 类型**

```typescript
export interface ModelProvider {
  id: string;
  name: string;
  base_url: string;
  api_key: string;
  api_format: string;
  is_preset: boolean;
  is_default: boolean;
  created_at: string;
}

export interface ProviderModel {
  id: string;
  provider_id: string;
  model_name: string;
  temperature: number;
  max_tokens: number;
  is_default: boolean;
}

export interface ProviderWithModels extends ModelProvider {
  models: ProviderModel[];
}

export interface TaskModelEntry {
  model_id: string | null;
  resolved: string | null;
}

export interface SettingsData {
  general: Record<string, string>;
  providers: ProviderWithModels[];
  task_models: Record<string, TaskModelEntry>;
}

export interface CreateProviderRequest {
  name: string;
  base_url: string;
  api_key: string;
  api_format?: string;
}

export interface UpdateProviderRequest {
  name: string;
  base_url: string;
  api_key: string;
  api_format?: string;
  is_default?: boolean;
}

export interface CreateModelRequest {
  model_name: string;
  temperature?: number;
  max_tokens?: number;
  is_default?: boolean;
}

export interface UpdateModelRequest {
  model_name: string;
  temperature?: number;
  max_tokens?: number;
  is_default?: boolean;
}

export interface TestConnectionResponse {
  ok: boolean;
  message: string;
}
```

- [ ] **步骤 2：在 `api/client.ts` 的 `api` 对象中添加 `settings` 方法**

在 `api` 对象的 `// AI` 之后追加：

```typescript
  // Settings
  settings: {
    getAll: () => request<SettingsData>("/settings"),
    listProviders: () => request<ProviderWithModels[]>("/settings/providers"),
    createProvider: (data: CreateProviderRequest) =>
      request<ModelProvider>("/settings/providers", { method: "POST", body: JSON.stringify(data) }),
    updateProvider: (id: string, data: UpdateProviderRequest) =>
      request<void>(`/settings/providers/${id}`, { method: "PUT", body: JSON.stringify(data) }),
    deleteProvider: (id: string) =>
      request<void>(`/settings/providers/${id}`, { method: "DELETE" }),
    createModel: (providerId: string, data: CreateModelRequest) =>
      request<ProviderModel>(`/settings/providers/${providerId}/models`, {
        method: "POST", body: JSON.stringify(data),
      }),
    updateModel: (providerId: string, modelId: string, data: UpdateModelRequest) =>
      request<void>(`/settings/providers/${providerId}/models/${modelId}`, {
        method: "PUT", body: JSON.stringify(data),
      }),
    deleteModel: (providerId: string, modelId: string) =>
      request<void>(`/settings/providers/${providerId}/models/${modelId}`, { method: "DELETE" }),
    getTaskModels: () => request<Record<string, TaskModelEntry>>("/settings/tasks"),
    setTaskModel: (taskName: string, modelId: string | null) =>
      request<void>(`/settings/tasks/${taskName}`, {
        method: "PUT", body: JSON.stringify({ model_id: modelId }),
      }),
    updateGeneral: (data: Record<string, string | boolean>) =>
      request<void>("/settings/general", { method: "PUT", body: JSON.stringify(data) }),
    testConnection: (providerId: string, modelName: string) =>
      request<TestConnectionResponse>("/settings/test-connection", {
        method: "POST",
        body: JSON.stringify({ provider_id: providerId, model_name: modelName }),
      }),
  },
```

同时需要在 `api/client.ts` 顶部 import 新增的类型：

```typescript
import type { Source, CreateSourceRequest, KnowledgePoint, QuizQuestion, QuizResult, LearningPlan, MasteryRecord, AiResearchResult, SettingsData, ProviderWithModels, ModelProvider, ProviderModel, CreateProviderRequest, UpdateProviderRequest, CreateModelRequest, UpdateModelRequest, TestConnectionResponse, TaskModelEntry } from "../types";
```

- [ ] **步骤 3：构建验证**

```bash
npx tsc --noEmit
```

预期：无类型错误。

- [ ] **步骤 4：Commit**

```bash
git add src/types.ts src/api/client.ts
git commit -m "feat: add settings types and API client methods"
```

---

### 任务 8：前端 — SettingsView 组件（3 标签页）

**文件：**
- 创建：`src/components/Content/SettingsView.tsx`
- 创建：`src/components/Content/SettingsView.css`
- 创建：`src/components/Content/GeneralTab.tsx`
- 创建：`src/components/Content/ProvidersTab.tsx`
- 创建：`src/components/Content/TaskModelsTab.tsx`

- [ ] **步骤 1：创建 `SettingsView.tsx` — 标签页框架**

```tsx
import { useState, useEffect } from "react";
import type { SettingsData } from "../../types";
import { api } from "../../api/client";
import GeneralTab from "./GeneralTab";
import ProvidersTab from "./ProvidersTab";
import TaskModelsTab from "./TaskModelsTab";
import "./SettingsView.css";

type Tab = "general" | "providers" | "tasks";

export default function SettingsView() {
  const [tab, setTab] = useState<Tab>("general");
  const [settings, setSettings] = useState<SettingsData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadSettings = async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await api.settings.getAll();
      setSettings(data);
    } catch (e: any) {
      setError(e.message || "加载设置失败");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { loadSettings(); }, []);

  if (loading) return <div className="settings-loading">加载中...</div>;
  if (error) return <div className="settings-error">{error}<button onClick={loadSettings}>重试</button></div>;
  if (!settings) return null;

  return (
    <div className="settings-view">
      <h2 className="settings-title">⚙ 设置</h2>
      <div className="settings-body">
        <nav className="settings-nav">
          {(["general", "providers", "tasks"] as Tab[]).map(t => (
            <button
              key={t}
              className={`settings-nav-btn ${tab === t ? "active" : ""}`}
              onClick={() => setTab(t)}
            >
              {{ general: "通用", providers: "模型厂商", tasks: "任务模型" }[t]}
            </button>
          ))}
        </nav>
        <div className="settings-content">
          {tab === "general" && <GeneralTab settings={settings} onSaved={loadSettings} />}
          {tab === "providers" && <ProvidersTab settings={settings} onSaved={loadSettings} />}
          {tab === "tasks" && <TaskModelsTab settings={settings} onSaved={loadSettings} />}
        </div>
      </div>
    </div>
  );
}
```

- [ ] **步骤 2：创建 `GeneralTab.tsx`**

```tsx
import { useState } from "react";
import type { SettingsData } from "../../../types";
import { api } from "../../../api/client";

interface Props {
  settings: SettingsData;
  onSaved: () => void;
}

export default function GeneralTab({ settings, onSaved }: Props) {
  const [theme, setTheme] = useState(settings.general.theme || "system");
  const [language, setLanguage] = useState(settings.general.language || "zh");
  const [dataPath, setDataPath] = useState(settings.general.data_path || "");
  const [searchEnabled, setSearchEnabled] = useState(
    settings.general.search_enabled === "true"
  );
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState("");

  const handleSave = async () => {
    setSaving(true);
    setMessage("");
    try {
      await api.settings.updateGeneral({
        theme,
        language,
        data_path: dataPath,
        search_enabled: searchEnabled,
      });
      setMessage("✅ 已保存");
      onSaved();
    } catch (e: any) {
      setMessage(`❌ ${e.message}`);
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="settings-tab general-tab">
      <div className="setting-group">
        <label className="setting-label">主题</label>
        <div className="setting-options">
          {[
            { value: "system", label: "跟随系统" },
            { value: "light", label: "亮色" },
            { value: "dark", label: "暗色" },
          ].map(o => (
            <label key={o.value} className="radio-label">
              <input type="radio" name="theme" value={o.value}
                checked={theme === o.value} onChange={e => setTheme(e.target.value)} />
              {o.label}
            </label>
          ))}
        </div>
      </div>

      <div className="setting-group">
        <label className="setting-label">语言</label>
        <select value={language} onChange={e => setLanguage(e.target.value)}>
          <option value="zh">中文</option>
          <option value="en">English</option>
        </select>
      </div>

      <div className="setting-group">
        <label className="setting-label">数据存储路径</label>
        <input type="text" value={dataPath}
          onChange={e => setDataPath(e.target.value)}
          placeholder="留空使用默认路径" />
      </div>

      <div className="setting-group">
        <label className="setting-label">网络搜索</label>
        <label className="switch-label">
          <input type="checkbox" checked={searchEnabled}
            onChange={e => setSearchEnabled(e.target.checked)} />
          {searchEnabled ? "已启用" : "已禁用"}
        </label>
      </div>

      <div className="setting-actions">
        <span className="setting-msg">{message}</span>
        <button className="btn-primary" onClick={handleSave} disabled={saving}>
          {saving ? "保存中..." : "保存"}
        </button>
      </div>
    </div>
  );
}
```

- [ ] **步骤 3：创建 `ProvidersTab.tsx`** — 厂商列表 + 编辑表单

完整代码见下方。因为代码较长，核心逻辑：

1. 展开/折叠编辑表单（内联）
2. 模型列表显示在编辑区（名称、温度、token 数）
3. 测试连接按钮
4. 设为默认、删除等操作

```tsx
import { useState } from "react";
import type { SettingsData, ProviderWithModels, CreateProviderRequest, UpdateProviderRequest, CreateModelRequest, UpdateModelRequest } from "../../../types";
import { api } from "../../../api/client";

interface Props {
  settings: SettingsData;
  onSaved: () => void;
}

export default function ProvidersTab({ settings, onSaved }: Props) {
  const [providers, setProviders] = useState(settings.providers);
  const [editId, setEditId] = useState<string | null>(null);
  const [addNew, setAddNew] = useState(false);
  const [saving, setSaving] = useState(false);
  const [testing, setTesting] = useState<string | null>(null);
  const [testResult, setTestResult] = useState<string>("");

  // Form state for editing/adding
  const [formName, setFormName] = useState("");
  const [formUrl, setFormUrl] = useState("");
  const [formKey, setFormKey] = useState("");

  const startEdit = (p: ProviderWithModels) => {
    setEditId(p.id);
    setAddNew(false);
    setFormName(p.name);
    setFormUrl(p.base_url);
    setFormKey("");
    setTestResult("");
  };

  const startAdd = () => {
    setEditId(null);
    setAddNew(true);
    setFormName("");
    setFormUrl("");
    setFormKey("");
    setTestResult("");
  };

  const cancelEdit = () => {
    setEditId(null);
    setAddNew(false);
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      if (addNew) {
        await api.settings.createProvider({
          name: formName, base_url: formUrl, api_key: formKey,
        });
      } else if (editId) {
        await api.settings.updateProvider(editId, {
          name: formName, base_url: formUrl, api_key: formKey || undefined,
        } as UpdateProviderRequest);
      }
      cancelEdit();
      onSaved();
      // Reload
      const data = await api.settings.getAll();
      setProviders(data.providers);
    } catch (e: any) {
      setTestResult(`❌ ${e.message}`);
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async (id: string, isPreset: boolean) => {
    if (isPreset) {
      setTestResult("❌ 预设厂商不可删除");
      return;
    }
    if (!confirm("确定删除此厂商？所有关联模型也将被删除。")) return;
    try {
      await api.settings.deleteProvider(id);
      onSaved();
      const data = await api.settings.getAll();
      setProviders(data.providers);
    } catch (e: any) {
      setTestResult(`❌ ${e.message}`);
    }
  };

  const handleSetDefault = async (id: string) => {
    try {
      await api.settings.updateProvider(id, {
        name: providers.find(p => p.id === id)!.name,
        base_url: providers.find(p => p.id === id)!.base_url,
        api_key: undefined,
        is_default: true,
      });
      onSaved();
      const data = await api.settings.getAll();
      setProviders(data.providers);
    } catch (e: any) {
      setTestResult(`❌ ${e.message}`);
    }
  };

  const handleTest = async (providerId: string, modelName: string) => {
    setTesting(modelName);
    setTestResult("");
    try {
      const res = await api.settings.testConnection(providerId, modelName);
      setTestResult(res.ok ? `✅ ${res.message}` : `❌ ${res.message}`);
    } catch (e: any) {
      setTestResult(`❌ ${e.message}`);
    } finally {
      setTesting(null);
    }
  };

  // Model operations within the edit form
  const editProvider = providers.find(p => p.id === editId);
  const [modelName, setModelName] = useState("");
  const [modelTemp, setModelTemp] = useState(0.7);
  const [modelTokens, setModelTokens] = useState(4096);

  const handleAddModel = async () => {
    if (!editId || !modelName) return;
    try {
      await api.settings.createModel(editId, {
        model_name: modelName, temperature: modelTemp, max_tokens: modelTokens,
      });
      setModelName("");
      onSaved();
      const data = await api.settings.getAll();
      setProviders(data.providers);
    } catch (e: any) {
      setTestResult(`❌ ${e.message}`);
    }
  };

  const handleDeleteModel = async (providerId: string, modelId: string) => {
    try {
      await api.settings.deleteModel(providerId, modelId);
      onSaved();
      const data = await api.settings.getAll();
      setProviders(data.providers);
    } catch (e: any) {
      setTestResult(`❌ ${e.message}`);
    }
  };

  return (
    <div className="settings-tab providers-tab">
      <ul className="provider-list">
        {providers.map(p => (
          <li key={p.id} className={`provider-item ${p.is_default ? "default" : ""}`}>
            <span className="provider-name">
              {p.is_default && "● "}{p.name}
              {p.is_preset && <span className="badge-preset">预设</span>}
            </span>
            <span className="provider-url">{p.base_url}</span>
            <div className="provider-actions">
              {!p.is_default && (
                <button className="btn-sm" onClick={() => handleSetDefault(p.id)}>设为默认</button>
              )}
              <button className="btn-sm" onClick={() => startEdit(p)}>编辑</button>
              <button className="btn-sm btn-danger" onClick={() => handleDelete(p.id, p.is_preset)}
                disabled={p.is_preset}>删除</button>
            </div>

            {(editId === p.id || addNew) && (
              <div className="provider-edit-form">
                <h4>{addNew ? "添加厂商" : `编辑 ${p.name}`}</h4>
                <label>名称</label>
                <input value={formName} onChange={e => setFormName(e.target.value)} />
                <label>Base URL</label>
                <input value={formUrl} onChange={e => setFormUrl(e.target.value)} />
                <label>API Key</label>
                <input type="password" value={formKey} onChange={e => setFormKey(e.target.value)}
                  placeholder={!addNew ? "留空则不修改" : ""} />

                {editId && editProvider && (
                  <div className="models-section">
                    <h4>模型列表</h4>
                    <ul>
                      {editProvider.models.map(m => (
                        <li key={m.id} className="model-item">
                          <span>{m.model_name}</span>
                          <span>Temp: {m.temperature}</span>
                          <span>Tokens: {m.max_tokens}</span>
                          <button className="btn-sm btn-danger"
                            onClick={() => handleDeleteModel(editId!, m.id)}
                            disabled={editProvider.models.length <= 1}>
                            删除
                          </button>
                          <button className="btn-sm"
                            onClick={() => handleTest(editId!, m.model_name)}
                            disabled={testing === m.model_name}>
                            {testing === m.model_name ? "测试中..." : "测试"}
                          </button>
                        </li>
                      ))}
                    </ul>
                    <div className="add-model-row">
                      <input placeholder="模型名" value={modelName}
                        onChange={e => setModelName(e.target.value)} />
                      <input type="number" value={modelTemp} step="0.1" min="0" max="2"
                        onChange={e => setModelTemp(+e.target.value)} title="Temperature" />
                      <input type="number" value={modelTokens}
                        onChange={e => setModelTokens(+e.target.value)} title="Max Tokens" />
                      <button className="btn-sm" onClick={handleAddModel}>+ 添加</button>
                    </div>
                  </div>
                )}

                <div className="form-actions">
                  {testResult && <span className="setting-msg">{testResult}</span>}
                  <button className="btn-primary" onClick={handleSave} disabled={saving}>
                    {saving ? "保存中..." : "保存"}
                  </button>
                  <button className="btn-secondary" onClick={cancelEdit}>取消</button>
                </div>
              </div>
            )}
          </li>
        ))}
      </ul>
      {!addNew && <button className="btn-secondary" onClick={startAdd}>+ 添加厂商</button>}
    </div>
  );
}
```

- [ ] **步骤 4：创建 `TaskModelsTab.tsx`**

```tsx
import { useState } from "react";
import type { SettingsData } from "../../../types";
import { api } from "../../../api/client";

interface Props {
  settings: SettingsData;
  onSaved: () => void;
}

const TASK_LABELS: Record<string, string> = {
  chat: "对话聊天",
  extract: "知识点提取",
  quiz_gen: "测验生成",
  search: "网络搜索",
};

export default function TaskModelsTab({ settings, onSaved }: Props) {
  const [taskModels, setTaskModels] = useState(settings.task_models);
  const [saving, setSaving] = useState<string | null>(null);
  const [message, setMessage] = useState("");

  const buildOptions = () => {
    const opts: { label: string; modelId: string | null }[] = [
      { label: "跟随默认", modelId: null },
    ];
    for (const p of settings.providers) {
      for (const m of p.models) {
        opts.push({ label: `${p.name} / ${m.model_name}`, modelId: m.id });
      }
    }
    return opts;
  };

  const options = buildOptions();

  const handleChange = async (taskName: string, modelId: string | null) => {
    setSaving(taskName);
    setMessage("");
    try {
      await api.settings.setTaskModel(taskName, modelId);
      setTaskModels(prev => ({
        ...prev,
        [taskName]: { model_id: modelId, resolved: options.find(o => o.modelId === modelId)?.label || null },
      }));
      setMessage("✅ 已保存");
      onSaved();
    } catch (e: any) {
      setMessage(`❌ ${e.message}`);
    } finally {
      setSaving(null);
    }
  };

  return (
    <div className="settings-tab taskmodels-tab">
      <p className="tab-desc">为每项 AI 任务指定使用的模型。未指定时自动使用默认厂商。</p>
      {Object.entries(TASK_LABELS).map(([taskName, label]) => {
        const current = taskModels[taskName];
        return (
          <div key={taskName} className="setting-group">
            <label className="setting-label">{label}</label>
            <select
              value={current?.model_id || ""}
              onChange={e => handleChange(taskName, e.target.value || null)}
              disabled={saving === taskName}
            >
              {options.map(o => (
                <option key={o.modelId || "__default"} value={o.modelId || ""}>
                  {o.label}
                </option>
              ))}
            </select>
            {current?.resolved && (
              <span className="resolved-hint">当前生效：{current.resolved}</span>
            )}
          </div>
        );
      })}
      {message && <p className="setting-msg">{message}</p>}
    </div>
  );
}
```

- [ ] **步骤 5：创建 `SettingsView.css`**

```css
.settings-view {
  padding: var(--spacing-lg);
  overflow-y: auto;
  height: 100%;
}

.settings-title {
  font-size: 22px;
  font-weight: 700;
  margin-bottom: var(--spacing-lg);
  color: var(--color-text-primary);
}

.settings-body {
  display: flex;
  gap: var(--spacing-lg);
}

.settings-nav {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 120px;
  border-right: 1px solid var(--color-border);
  padding-right: var(--spacing-md);
}

.settings-nav-btn {
  padding: 10px 14px;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: var(--color-text-secondary);
  font-size: 14px;
  cursor: pointer;
  text-align: left;
  transition: background 0.15s;
}

.settings-nav-btn:hover {
  background: var(--color-sidebar-hover);
  color: var(--color-text-primary);
}

.settings-nav-btn.active {
  background: var(--color-sidebar-active);
  color: var(--color-accent);
  font-weight: 600;
}

.settings-content {
  flex: 1;
  min-width: 0;
}

.settings-tab {
  max-width: 600px;
}

.setting-group {
  margin-bottom: var(--spacing-md);
}

.setting-label {
  display: block;
  font-weight: 600;
  margin-bottom: 6px;
  color: var(--color-text-primary);
}

.setting-options {
  display: flex;
  gap: var(--spacing-md);
}

.radio-label {
  display: flex;
  align-items: center;
  gap: 4px;
  cursor: pointer;
  color: var(--color-text-secondary);
}

.setting-group select,
.setting-group input[type="text"],
.setting-group input[type="password"],
.setting-group input[type="number"] {
  width: 100%;
  padding: 8px 12px;
  border: 1px solid var(--color-border);
  border-radius: 6px;
  background: var(--color-surface);
  color: var(--color-text-primary);
  font-size: 14px;
  font-family: inherit;
}

.switch-label {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  color: var(--color-text-secondary);
}

.setting-actions {
  display: flex;
  justify-content: flex-end;
  align-items: center;
  gap: var(--spacing-sm);
  margin-top: var(--spacing-lg);
  padding-top: var(--spacing-md);
  border-top: 1px solid var(--color-border);
}

.setting-msg {
  font-size: 13px;
  color: var(--color-text-secondary);
}

.btn-primary {
  padding: 8px 20px;
  background: var(--color-accent);
  color: #fff;
  border: none;
  border-radius: 6px;
  font-size: 14px;
  cursor: pointer;
}
.btn-primary:hover { background: var(--color-accent-hover); }
.btn-primary:disabled { opacity: 0.6; cursor: default; }

.btn-secondary {
  padding: 8px 20px;
  background: transparent;
  color: var(--color-text-secondary);
  border: 1px solid var(--color-border);
  border-radius: 6px;
  font-size: 14px;
  cursor: pointer;
}
.btn-secondary:hover { background: var(--color-sidebar-hover); }

.btn-sm {
  padding: 4px 10px;
  font-size: 12px;
  border: 1px solid var(--color-border);
  border-radius: 4px;
  background: transparent;
  color: var(--color-text-secondary);
  cursor: pointer;
}
.btn-sm:hover { background: var(--color-sidebar-hover); }
.btn-danger { color: #e53e3e; border-color: #e53e3e; }
.btn-danger:disabled { opacity: 0.3; cursor: default; }

/* Provider list */
.provider-list {
  list-style: none;
  padding: 0;
}

.provider-item {
  padding: var(--spacing-sm);
  margin-bottom: var(--spacing-sm);
  border: 1px solid var(--color-border);
  border-radius: 8px;
  background: var(--color-surface);
}

.provider-item.default {
  border-color: var(--color-accent);
}

.provider-name {
  font-weight: 600;
  color: var(--color-text-primary);
}

.badge-preset {
  display: inline-block;
  margin-left: 6px;
  padding: 1px 6px;
  font-size: 10px;
  background: rgba(124, 111, 247, 0.12);
  color: var(--color-accent);
  border-radius: 4px;
}

.provider-url {
  display: block;
  font-size: 12px;
  color: var(--color-text-muted);
  margin-top: 4px;
}

.provider-actions {
  display: flex;
  gap: 6px;
  margin-top: 8px;
}

.provider-edit-form {
  margin-top: var(--spacing-sm);
  padding: var(--spacing-sm);
  background: var(--color-bg);
  border-radius: 8px;
}

.provider-edit-form label {
  display: block;
  font-size: 13px;
  font-weight: 600;
  margin: 8px 0 4px;
  color: var(--color-text-secondary);
}

.models-section {
  margin-top: var(--spacing-md);
  padding-top: var(--spacing-sm);
  border-top: 1px solid var(--color-border);
}

.model-item {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  padding: 6px 0;
  font-size: 13px;
  color: var(--color-text-secondary);
}

.model-item span:first-child {
  font-weight: 600;
  color: var(--color-text-primary);
}

.add-model-row {
  display: flex;
  gap: 6px;
  margin-top: 8px;
  align-items: center;
}

.add-model-row input {
  width: 100px;
  padding: 4px 8px !important;
  font-size: 13px !important;
}

.form-actions {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  margin-top: var(--spacing-md);
}

.resolved-hint {
  display: block;
  font-size: 12px;
  color: var(--color-text-muted);
  margin-top: 4px;
}

.tab-desc {
  font-size: 13px;
  color: var(--color-text-secondary);
  margin-bottom: var(--spacing-md);
}
```

- [ ] **步骤 6：构建验证**

```bash
npx tsc --noEmit
```

预期：无类型错误。

- [ ] **步骤 7：Commit**

```bash
git add src/components/Content/SettingsView.tsx src/components/Content/SettingsView.css \
        src/components/Content/GeneralTab.tsx src/components/Content/ProvidersTab.tsx \
        src/components/Content/TaskModelsTab.tsx
git commit -m "feat: add SettingsView component with 3 tabs (general, providers, task models)"
```

---

### 任务 9：前端 — 集成设置入口到 Layout / Sidebar / Content

**文件：**
- 修改：`src/components/Layout.tsx`
- 修改：`src/components/Content.tsx`
- 修改：`src/components/Sidebar.tsx`
- 修改：`src/components/Sidebar.css`

- [ ] **步骤 1：修改 `Layout.tsx` — 新增 `settings` 视图**

```tsx
import { useState } from "react";
import Sidebar from "./Sidebar";
import Content from "./Content";
import "./Layout.css";

export type View = "chat" | "learning" | "settings";

export default function Layout() {
  const [selectedKpId, setSelectedKpId] = useState<string | null>(null);
  const [view, setView] = useState<View>("chat");

  const handleSelectKp = (id: string) => {
    setSelectedKpId(id);
    setView("learning");
  };

  return (
    <div className="layout">
      <Sidebar onSelectKp={handleSelectKp} selectedKpId={selectedKpId ?? undefined}
        currentView={view} onNavigate={setView} />
      <Content view={view} selectedKpId={selectedKpId} />
    </div>
  );
}
```

- [ ] **步骤 2：修改 `Content.tsx` — 支持 `settings` 视图**

```tsx
import ChatPanel from "./Chat/ChatPanel";
import LearningView from "./Content/LearningView";
import SettingsView from "./Content/SettingsView";
import type { View } from "./Layout";
import "./Content.css";

interface Props {
  view: View;
  selectedKpId: string | null;
}

export default function Content({ view, selectedKpId }: Props) {
  return (
    <main className="content">
      {view === "settings" ? <SettingsView /> :
       view === "learning" ? <LearningView kpId={selectedKpId} /> :
       <ChatPanel />}
    </main>
  );
}
```

- [ ] **步骤 3：修改 `Sidebar.tsx` — 底部加设置按钮，新增 Props**

将 Sidebar 的 props 改为：

```tsx
import { useState } from "react";
import SourceList from "./Sidebar/SourceList";
import KpList from "./Sidebar/KpList";
import { usePlatform, getRunMode } from "../utils/tauri";
import type { View } from "./Layout";
import "./Sidebar.css";

interface Props {
  onSelectKp: (id: string) => void;
  selectedKpId?: string;
  currentView: View;
  onNavigate: (view: View) => void;
}
```

在侧栏底部（`sidebar-footer` div）上方加入设置按钮：

```tsx
      <div className="sidebar-footer">
        <button
          className={`sidebar-settings-btn ${currentView === "settings" ? "active" : ""}`}
          onClick={() => onNavigate("settings")}
          title="设置"
        >
          ⚙
        </button>
        <span className={`sidebar-mode-badge ${runMode}`}>
          {platform}
        </span>
        <span className="sidebar-version">v0.1.0</span>
      </div>
```

- [ ] **步骤 4：在 `Sidebar.css` 追加设置按钮样式**

```css
.sidebar-settings-btn {
  display: block;
  margin-bottom: var(--spacing-xs);
  padding: 8px;
  background: transparent;
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 8px;
  color: var(--sidebar-text);
  font-size: 18px;
  cursor: pointer;
  text-align: center;
  transition: background 0.15s, color 0.15s;
}

.sidebar-settings-btn:hover {
  background: var(--sidebar-hover);
  color: #e0e0f0;
}

.sidebar-settings-btn.active {
  background: var(--sidebar-active-bg);
  color: var(--sidebar-accent);
  border-color: var(--sidebar-accent);
}
```

- [ ] **步骤 5：构建验证**

```bash
npx tsc --noEmit
```

预期：无类型错误。

- [ ] **步骤 6：完整启动验证**

```bash
npm run dev &
sleep 3
# 测试 settings API
curl -s http://localhost:14200/api/settings | python3 -m json.tool | head -30
# 测试 health
curl -s http://localhost:14200/api/health
```

预期：settings API 返回完整 JSON（含 3 个预设厂商），health 返回 OK。

- [ ] **步骤 7：Commit**

```bash
git add src/components/Layout.tsx src/components/Content.tsx \
        src/components/Sidebar.tsx src/components/Sidebar.css
git commit -m "feat: integrate settings entry into sidebar and content routing"
```

---

## 验证检查清单

实现全部任务完成后，逐项验证：

- [ ] `cargo check` 在 `src-tauri/` 下零错误
- [ ] `npx tsc --noEmit` 在项目根目录零错误
- [ ] `npm run dev` 启动成功，访问 `http://localhost:14200`
- [ ] `GET /api/settings` 返回预设的 DeepSeek、OpenAI、Anthropic 厂商
- [ ] 侧栏底部有 ⚙ 设置按钮，点击进入设置页面
- [ ] 通用设置页：修改主题/语言/路径/搜索 → 保存 → 刷新 → 值保持
- [ ] 模型厂商页：添加自定义厂商 → 添加模型 → 编辑 → 测试连接 → 设为默认 → 删除
- [ ] 任务模型页：为各任务选择不同模型 → 保存
- [ ] 聊天功能：`POST /api/ai/research` 使用配置的模型（不再依赖环境变量）
- [ ] 删除被任务引用的模型 → 后端返回错误提示
- [ ] 删除预设厂商 → 后端返回错误提示
