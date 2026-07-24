# Lexio：设置界面 & 模型配置 设计规格

> 2026-07-24 | 状态: 设计中

## 1. 概述

为 Lexio 增加设置界面，支持用户配置模型厂商和分配模型到各 AI 任务，以及管理应用通用设置。

### 核心需求

- **多厂商管理**：预设主流厂商 + 用户自定义 OpenAI 兼容接口
- **一厂商多模型**：每个厂商下可配置多个模型，各自独立设置 temperature / max_tokens
- **按任务分配模型**：对话、知识点提取、测验生成、网络搜索四项任务可独立选择模型，未指定时使用默认厂商
- **通用设置**：主题、语言、数据存储路径、网络搜索开关
- **入口**：侧栏底部设置按钮 → 主内容区显示

---

## 2. 数据模型

### 2.1 数据库 Schema（新增 4 张表）

```sql
-- 通用键值设置
CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- 模型厂商/提供商
CREATE TABLE IF NOT EXISTS model_providers (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    base_url   TEXT NOT NULL,
    api_key    TEXT NOT NULL DEFAULT '',
    api_format TEXT NOT NULL DEFAULT 'openai_compatible',
    is_preset  INTEGER NOT NULL DEFAULT 0,
    is_default INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 厂商下的具体模型
CREATE TABLE IF NOT EXISTS provider_models (
    id          TEXT PRIMARY KEY,
    provider_id TEXT NOT NULL REFERENCES model_providers(id),
    model_name  TEXT NOT NULL,
    temperature REAL NOT NULL DEFAULT 0.7,
    max_tokens  INTEGER NOT NULL DEFAULT 4096
);

-- 任务-模型映射
CREATE TABLE IF NOT EXISTS task_models (
    id        TEXT PRIMARY KEY,
    task_name TEXT NOT NULL UNIQUE,
    model_id  TEXT,
    FOREIGN KEY (model_id) REFERENCES provider_models(id)
);
```

### 2.2 预设数据

首次启动时写入 3 个预设厂商（`is_preset=1`，API key 为空）：

| 厂商 | Base URL | 预设模型 |
|------|----------|---------|
| DeepSeek | `https://api.deepseek.com` | `deepseek-chat` (temp=0.7, tokens=4096) |
| OpenAI | `https://api.openai.com` | `gpt-4o` (temp=0.7, tokens=4096) |
| Anthropic | `https://api.anthropic.com` | `claude-sonnet-4-20250514` (temp=0.7, tokens=4096) |

DeepSeek 设为默认厂商（`is_default=1`）。每个预设厂商默认启用第一个模型。

### 2.3 默认通用设置值

| 键 | 默认值 | 说明 |
|----|--------|------|
| `theme` | `system` | system / light / dark |
| `language` | `zh` | zh / en |
| `data_path` | (空串) | 空串 = 使用 app data dir 默认路径 |
| `search_enabled` | `false` | 网络搜索开关（v2 功能预留） |

---

## 3. REST API

### 3.1 端点列表

| 方法 | 路径 | 说明 |
|------|------|------|
| `GET` | `/api/settings` | 获取全部设置（聚合） |
| `GET` | `/api/settings/providers` | 列出所有厂商 |
| `POST` | `/api/settings/providers` | 添加厂商 |
| `PUT` | `/api/settings/providers/{id}` | 更新厂商（含模型列表） |
| `DELETE` | `/api/settings/providers/{id}` | 删除厂商（预设不可删） |
| `POST` | `/api/settings/providers/{id}/models` | 添加模型 |
| `PUT` | `/api/settings/providers/{pid}/models/{mid}` | 更新模型 |
| `DELETE` | `/api/settings/providers/{pid}/models/{mid}` | 删除模型（被任务引用时阻止） |
| `GET` | `/api/settings/tasks` | 列出任务模型映射 |
| `PUT` | `/api/settings/tasks/{task_name}` | 设置某任务的模型 |
| `PUT` | `/api/settings/general` | 批量更新通用设置 |
| `POST` | `/api/settings/test-connection` | 测试厂商连接 |

### 3.2 关键请求/响应

**`GET /api/settings`** — 聚合返回：

```json
{
  "general": {
    "theme": "system",
    "language": "zh",
    "data_path": "",
    "search_enabled": false
  },
  "providers": [
    {
      "id": "uuid-xxx",
      "name": "DeepSeek",
      "base_url": "https://api.deepseek.com",
      "api_key": "sk-****xxxx",
      "api_format": "openai_compatible",
      "is_preset": true,
      "is_default": true,
      "models": [
        { "id": "uuid", "model_name": "deepseek-chat", "temperature": 0.7, "max_tokens": 4096 }
      ]
    }
  ],
  "task_models": {
    "chat":     { "model_id": null },
    "extract":  { "model_id": null },
    "quiz_gen": { "model_id": null },
    "search":   { "model_id": null }
  }
}
```

注意：API key 脱敏返回（仅显示后 4 位）。

**`PUT /api/settings/tasks/{task_name}`** — 指定任务模型：

```json
{
  "model_id": "uuid-xxx"
}
```

`model_id` 为 `null` 表示跟随默认。

**`POST /api/settings/test-connection`** — 测试连接：

```json
{
  "provider_id": "uuid-xxx",
  "model_name": "deepseek-chat"
}
```

响应：
```json
{ "ok": true, "message": "连接成功" }
```
或
```json
{ "ok": false, "message": "401 Unauthorized" }
```

### 3.3 约束规则

| 规则 | 实现位置 |
|------|---------|
| 预设厂商不可删除 | API handler |
| 删除模型时检查是否被 task_models 引用 | repo 层 |
| is_default 唯一性：设为默认时取消其他厂商的默认状态 | repo 层 |
| 至少保留一个模型（每个厂商至少有一个模型） | repo 层 |

---

## 4. 前端设计

### 4.1 入口

侧栏底部现有 Web/Desktop 标签同区域添加设置图标按钮（⚙），点击后主内容区切换为 SettingsView 组件。

### 4.2 布局

设置页分 3 个标签页，左侧竖排标签导航，右侧为内容区：

```
标签页: 通用 | 模型厂商 | 任务模型
```

### 4.3 标签页 1：通用设置

- **主题**：单选项（跟随系统 / 亮色 / 暗色）
- **语言**：下拉选择（中文 / English）
- **数据存储路径**：文本输入 + 浏览按钮
- **网络搜索**：开关按钮
- 底部：恢复默认 / 保存 按钮

### 4.4 标签页 2：模型厂商

- 厂商列表（卡片或列表项），每项显示：
  - 名称、Base URL 摘要、是否为默认、是否为预设
  - 「默认」标记（单选，点击即切换默认厂商）
  - 编辑按钮
- 底部：「+ 添加厂商」按钮
- 点击编辑/添加展开表单：
  - 名称、Base URL、API Key（带显示/隐藏切换）
  - 模型列表子区域：每行显示模型名、temperature、max_tokens，可编辑/删除
  - 「+ 添加模型」按钮
  - 「测试连接」按钮
  - 保存 / 取消

### 4.5 标签页 3：任务模型

- 4 项任务（对话聊天、知识点提取、测验生成、网络搜索）
- 每项只有一个下拉：选择模型
- 下拉选项来自所有厂商的所有模型，格式为「厂商名 / 模型名」
- 第一项为「跟随默认」——对应 model_id = null
- 底部：恢复默认 / 保存 按钮

### 4.6 交互细节

| 场景 | 处理 |
|------|------|
| 进入设置页 | 调用 `GET /api/settings` 获取全部配置 |
| 切换标签页 | 前端维护三份编辑态副本，切换不丢失未保存内容 |
| 离开设置页（有未保存更改） | 弹窗确认 |
| 测试连接 | 发送测试请求，显示加载动画 + 成功/失败结果 |
| 保存厂商 | 含模型列表一并提交到 `PUT /api/settings/providers/{id}` |
| 删除模型 | 若被任务引用，后端返回错误，前端提示「该模型被 XX 任务使用，无法删除」 |

---

## 5. 后端集成

### 5.1 数据流

```
用户配置 → REST API → settings 表 / model_providers 表 / provider_models 表 / task_models 表
                                                                          ↓
AI 调用时 → SettingsService::resolve(task_name)
              → 查 task_models.model_id
              → 若 NULL，取默认厂商的默认模型
              → 返回 LlmConfig { base_url, api_key, model, temperature, max_tokens, api_format }
              → LlmClient::new(config).chat(...)
```

### 5.2 LlmClient 新增工厂方法

```rust
impl LlmClient {
    pub fn from_config(config: LlmConfig) -> Self {
        Self { config, client: Client::new() }
    }
}
```

不再需要 `from_task` 方法——调用方（API handler）负责 resolve 配置，LlmClient 只负责发请求。这样职责更清晰。

### 5.3 改动波及

| 文件 | 改动 |
|------|------|
| `src-tauri/src/db.rs` | migration 新增 4 张表 |
| `src-tauri/src/models.rs` | 新增 ModelProvider、ProviderModel、TaskModel、SettingsEntry 等 struct |
| `src-tauri/src/repo/settings.rs` | 🆕 设置层 repository（含预设数据初始化） |
| `src-tauri/src/ai/llm.rs` | LlmConfig 增加 temperature、max_tokens、api_format 字段 |
| `src-tauri/src/ai/extract.rs` | 调用方改为接收 LlmConfig 参数（而非硬编码） |
| `src-tauri/src/ai/quiz_gen.rs` | 同上 |
| `src-tauri/src/api/settings.rs` | 🆕 设置 API 端点 |
| `src-tauri/src/api/ai_routes.rs` | 聊天等端点从数据库 resolve LlmConfig |
| `src-tauri/src/server.rs` | 注册 settings 路由 |
| `src-tauri/src/lib.rs` | 添加 mod declarations，启动时初始化预设数据 |

### 5.4 预设数据初始化

在 `lib.rs` 的启动流程中（数据库 migrate 之后）调用：

```rust
repo::settings::init_presets(&db)?;
```

该函数幂等——检查 `model_providers` 表是否为空，若为空则插入预设数据。

### 5.5 通用设置读取

通用设置提供简单的 get/set 接口：

```rust
pub fn get_setting(db: &Database, key: &str) -> Option<String>
pub fn set_settings(db: &Database, entries: &[(String, String)]) -> Result<(), String>
```

前端通过 API 批量读写，后端缓存？不需要——桌面应用单用户，直接每次读数据库即可。

---

## 6. 错误处理

| 场景 | 处理 |
|------|------|
| 无可用模型 | AI 调用返回友好错误「请先在设置中配置模型」，前端在聊天输入区显示引导 |
| API key 无效 | LLM 返回错误，前端聊天区显示错误气泡 +「去设置」链接 |
| 所有厂商被删 | 预设厂商不可删除，保证永远有至少一个厂商 |
| 任务引用的模型被删 | 删除前检查引用，阻止并提示「该模型被 XX 任务使用」 |
| 未保存离开 | 前端 draft state 机制，弹窗确认 |
| 旧版本迁移 | migration 检测新表是否存在，已存在则跳过，已有数据不丢失 |

---

## 7. api_format 预留设计

`model_providers.api_format` 字段当前固定为 `openai_compatible`。未来扩展方向：

| api_format 值 | 请求格式 | 端点 |
|---------------|---------|------|
| `openai_compatible` | OpenAI Chat Completions | `{base_url}/chat/completions` |
| `anthropic` (未来) | Anthropic Messages API | `{base_url}/messages` |

LlmClient 后续可按 `api_format` 分发到不同实现。现在仅实现 OpenAI 兼容，但 schema 已留好扩展点，无需数据库迁移。

---

## 8. 非功能需求

- **安全**：API key 存储在本地 SQLite（app data dir），不加密但不离开用户设备。GET 返回时脱敏显示（仅后 4 位）。
- **幂等**：预设数据初始化幂等，重复调用不会产生重复数据。
- **兼容**：新增表不影响现有 Source、KnowledgePoint、Quiz 等数据。
- **前端状态**：设置页使用本地 draft state，保存时才提交到后端，避免半截配置生效。
