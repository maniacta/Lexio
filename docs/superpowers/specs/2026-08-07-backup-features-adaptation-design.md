# Lexio：备份分支功能适配设计（chat 教练 + 日志审计 + 独立修复）

日期：2026-08-07
状态：已批准（brainstorming 完成）

## 背景

本地 main 曾积累 42 个提交（备份在 `backup/local-main-pre-reset`，HEAD 为 5f27180），
因与远端新架构冲突过多，按用户指示放弃本地重放、直接采用远端代码（8653425）。
本设计把这些功能适配到新架构：

- **新架构基准**：per-vendor AI providers（`ProviderKind`/`LlmProvider`/`create_provider`，
  仅 DeepSeek 已实现）、`blocking::run` SQLite 卸载、`crypto` API key 加密、
  `require_token` 全路由认证（`X-Lexio-Token`）、`db.migrate()` 顺序 IF NOT EXISTS 建表。
- **适配原则**：架构与远端已有实现以远端为准；备份分支独有功能恢复；远端已有等价物的跳过。

## 1. LLM chat 教练 + 多会话持久化

### 1.1 后端

**repo/learning.rs**：恢复 `get_plan(db, id) -> Result<Option<LearningPlan>, String>`
（备份 3fb45a6）。

**repo/chat.rs（新）**：多会话 CRUD，全部走 `Mutex<Connection>`：

| 函数 | 说明 |
|---|---|
| `list_sessions(db)` | 会话列表，`updated_at` 倒序，含消息数 |
| `create_session(db, title) -> Session` | 新建（默认标题「新对话」） |
| `get_messages(db, session_id)` | 某会话消息，`created_at` 正序 |
| `append_message(db, session_id, role, content, actions, context) -> Message` | 追加消息 |
| `delete_session(db, session_id)` | 删除会话（级联删消息） |
| `set_session_title(db, session_id, title)` | 更新标题（首条用户消息后设为前 ~20 字） |
| `set_session_plan(db, session_id, plan_id)` | 研究结果绑定 plan 到会话 |

**api/chat_routes.rs（新）**：REST 端点（全部在 `require_token` 内）：

| 端点 | 说明 |
|---|---|
| `GET /api/chat/sessions` | 会话列表 |
| `POST /api/chat/sessions` | 新建会话 |
| `GET /api/chat/sessions/{id}/messages` | 会话消息 |
| `POST /api/chat/messages` | 追加消息（body: session_id, role, content, actions?, context?） |
| `DELETE /api/chat/sessions/{id}` | 删除会话 |
| `POST /api/chat/sessions/{id}/plan` | 绑定 plan_id |

DB 调用全部包 `blocking::run`；handler 加 `#[instrument]` + audit 事件（见第 2 节）。

**api/ai_routes.rs**：新增 chat 结构体与 handler：

- `ChatRequest { messages: Vec<ChatMessageItem>, context: Option<ChatContext> }`
- `ChatMessageItem { role, content }`、`ChatContext { plan_id?, current_kp_id? }`
- `ChatAction { type, label, payload }`、`ChatResponse { content, actions }`
- `chat` handler：
  - `blocking::run` 包 `repo::settings::resolve_llm_config(db, "chat")`，沿用远端 `map_llm_resolve_err`
  - `crate::ai::create_provider(config)` 构建 provider（仅 DeepSeek 可用）
  - 上下文注入（plan/kp 读取全走 `blocking::run`）：`get_plan` + `list_kps_by_ids`
    + `get_kp`，拼入 system prompt
  - 响应解析用 `crate::ai::extract_json_payload`
  - 解析失败回退：原样文本 + 空 actions

**System prompt 行为指南**（恢复备份 + 增强）：
- 用中文回复；用户表达学习意图时引导选择知识点（`navigate_learning`，kpId 用列表中的 id）
- 建议测验（`start_quiz`）；概念问题用已有知识点回答；无对应知识点建议研究
- **新增兜底**：用户表达"想学新主题 X"意图时返回 `start_research` action
  （payload 带 topic），前端渲染「🔍 研究：X」按钮，点击执行研究

**server.rs**：注册 `/api/ai/chat` 与 chat_routes 全部路由（`require_token` 内）。

### 1.2 前端

**types.ts**：
- `ChatAction { type: "navigate_learning" | "start_quiz" | "view_source" | "start_research", label, payload }`
  （payload: kpId?/kpTitle?/sourceId?/topic?）
- `ChatMessage` 扩展：`actions?: ChatAction[]`、`context?: { plan?, kps? }`
- `ChatRequest`/`ChatResponse`、`ChatSession { id, title, plan_id?, message_count, updated_at }`

**api/client.ts**：`api.ai.chat(data)`；研究沿用远端已有 `api.ai.startResearch`。

**hooks/useChat.ts**（重构）：
- 状态：`sessions`、`activeSessionId`、会话内 `messages`、`loading`、`planContext {plan_id,title}`
- 挂载：加载会话列表 → 打开最近会话（无则新建）；加载该会话消息；用该会话的 `plan_id` 恢复 `planContext`（无则 null）
- 新建 / 切换 / 删除会话
- `sendMessage`：
  1. 持久化 user 消息（失败不阻塞，logger 记录）
  2. 触发词检测（备份 5f27180 版：`startsWith` + 长度检查，"我不学习"不误触发；
     `sameTopic`：已有 plan 且消息含 plan 标题时视为聊天跟进）
  3. 命中研究 → `api.ai.startResearch` → 结果消息（含 KP 导航按钮）+ 设置并持久化 plan_id
  4. 否则 → `api.ai.chat`（上下文带 planContext）→ 持久化 assistant 消息
  5. **处理 `start_research` action** → 执行研究（方案 3 可纠正兜底）
- logger 埋点：start_research / send_chat / chat_error（见第 2 节）
- `clearPlan` 保留

**Chat/MessageBubble.tsx**：恢复 action 按钮渲染（`message.actions` → 按钮列表，点击
`onAction(action)`）；保留远端 markdown + rehype-sanitize。

**Chat/ChatPanel.tsx**：会话列表侧栏（标题 + 删除按钮）+「新对话」按钮 + 消息区；
保留远端 API key 引导 banner（`needsApiKeySetup`）与示例话题。

**Layout.tsx / Content.tsx**：恢复导航管道 `onChatNavigate`；保留远端常挂载
（hidden 切换）结构与 `onOpenSettings`。

**action 处理**（Content/ChatPanel 层）：
- `navigate_learning` → `setSelectedKpId` + 切 learning 视图
- `start_quiz` → 切 learning 视图并直接打开测验（按备份 d15c755 行为）
- `view_source` → 打开 SourceViewer modal（按备份 d15c755 行为）
- `start_research` → 执行 `api.ai.startResearch(topic)` 并展示结果

### 1.3 明确不做

- 会话重命名 UI（自动标题足够）、搜索历史消息、导出
- LLM 流式输出（记忆中的规划项，不在本次范围）

## 2. 日志 / 审计系统（完整恢复）

### 2.1 后端

- **Cargo.toml**：`tracing`、`tracing-subscriber`（env-filter, json）、`tracing-appender`
- **db.rs**（配合第 3 节 v2 迁移）：`audit_logs` 表
  （id, level, category, action, user_action, params_summary JSON, result_summary JSON,
  error_message, duration_ms, ip, created_at）
- **repo/audit.rs（新）**：`insert` / `list` / `prune`；**保留策略**：每次写入时清理
  超过 30 天的旧记录
- **tracing_layer.rs（新）**：`AuditDbLayer` 自定义 tracing Layer，把 `target="audit"`
  事件写入 `audit_logs` 表（同步写 Mutex，非 async）
- **lib.rs**：setup 中初始化 tracing subscriber：
  - `RollingFileAppender` 文件日志（app_data_dir/logs，日滚 + 30 天清理）
  - `EnvFilter`（`LEXIO_LOG_LEVEL`，默认 info）
  - `AuditDbLayer::new(db)`（db 为 `&'static Database`）
  - `Box::leak` 保活 `non_blocking` guard；幂等初始化（重复 setup 不重复注册）
- **server.rs**：`audit_middleware`（**最外层**，先审计后认证）：
  - 记录 method/path/status/duration
  - 跳过 `/api/health`、`/api/auth/token`、`/api/logs/batch`
  - ≥400 收集错误体；≥500 且含内部 SQL 特征时返回「服务器内部错误，详情已记录到日志」
- **所有 handler**：`#[instrument]` + audit tracing 事件（备份 8d01003 合并模式：
  `blocking::run` 结构 + duration_ms + params/result summary）
- **POST /api/logs/batch**：前端日志上传端点（`require_token` 内；audit 跳过防噪声）；
  校验 level 白名单（info/warn/error）、字段长度上限、批量大小上限

### 2.2 前端

- **utils/logger.ts（新）**：队列 + 批量上传；复用远端 client 的
  `getApiBase()`/token 机制（带 `X-Lexio-Token`）；level/category/action 字段
- **api/client.ts**：接入日志上传；保留远端 getApiBase/错误处理
- **接线点**：全局未捕获异常/未处理 rejection + `useChat` 埋点
  （start_research / send_chat / chat_error）+ 消息持久化失败

### 2.3 说明

- audit 在 auth 外层：未认证请求（401）也入审计（安全审计需要）
- 日志上传端点跳过审计防噪声
- 远端已有等价错误清洗逻辑时保留远端的

## 3. 独立修复

### 3.1 FTS 搜索修复（远端 bug，必须修）

远端 `repo/knowledge.rs::search_kps` 与 `repo/source.rs::search_sources` 用
`WHERE kp_fts MATCH ?1 ORDER BY rank` 直接查 external-content 表，会报
"no such column"。按备份 5c21026 重写：

- 查询形式：`JOIN (SELECT rowid, rank FROM kp_fts WHERE kp_fts MATCH ?1)`
- 输入转义为 FTS5 短语：`format!("\"{}\"", q.replace('"', "\"\""))`

### 3.2 级联删除微调

远端 `delete_kp` 已级联（quiz_attempts/quiz_questions/mastery_records/relations）。
对照备份 9a18b77 补：从 `learning_plans.kp_ids`（JSON 数组）移除被删 kp 引用。

### 3.3 SM-2 每日限制（移植 + 单测）

备份 88db2c6：`update_mastery` 时若该 KP 今天已 review 过则不再推进 SM-2
（返回现有记录，audit 事件 `advanced: false`），防止一次测验算多次复习、
间隔被夸大（0→1→6→16 天）。适配到远端 `blocking::run` 闭包内；移植单测。

### 3.4 迁移跟踪（PRAGMA user_version）

把远端 `migrate()` 重构为版本化：

- v1 = 远端现有全部表（sources/knowledge_points/relations/quiz_*/mastery_records/
  learning_plans/settings/model_providers/provider_models/task_models + FTS 表）
- v2 = `audit_logs`
- v3 = `chat_sessions` / `chat_messages`

每版本 IF NOT EXISTS 幂等；执行后设置 `user_version`；旧库（user_version=0）
自动顺序升级。`delete_kp` 级联里的新表约束一并覆盖。

### 3.5 明确跳过（远端已有等价物）

- 复习 retry/skip UI（远端 ReviewSession 已有）
- 请求超时 / CSP / extract_json 整合（远端已有）
- `bin/server.rs` 独立二进制（远端为 Tauri 嵌入模式）
- Anthropic 适配器（用户决定暂不包含）

## 4. 错误处理与验证

### 错误处理

- chat：LLM 调用失败 → 错误消息展示（`formatApiError`）+ logger；持久化失败不阻塞聊天
- 日志上传：失败静默降级（console 保留），不影响业务
- 审计写入：失败仅记录到 tracing（不阻断请求）
- 会话加载失败：显示错误提示，不崩溃

### 验证

- 后端：`cargo check` + `cargo test`（SM-2 单测、chat repo 单测如可行）
- 前端：`npm run build`（tsc + vite）
- 冒烟：设置配置 DeepSeek key → 聊天（触发研究 / 概念问答 / start_research 兜底 /
  动作按钮导航）→ 刷新后会话恢复 → 多会话新建/切换/删除 → 检查 audit_logs 有记录 →
  搜索一个关键词（验证 FTS 修复）→ 测验后同 KP 当天再答（验证 SM-2 限制）

## 实现顺序建议

1. 迁移跟踪（v1/v2/v3 表结构）→ 2. repo 层（chat/audit/get_plan/FTS/SM-2）→
3. chat 后端（handler + 路由）→ 4. 日志后端（tracing/中间件/端点）→
5. 前端（types/client/useChat/组件）→ 6. 验证与冒烟
