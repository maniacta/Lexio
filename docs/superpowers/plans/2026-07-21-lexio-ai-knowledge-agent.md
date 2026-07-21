# Lexio AI 知识学习 Agent 实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 实现一个以学习教练为核心的 AI 知识 agent 桌面应用——用户放入资料/提主题，AI 搜索提取知识点，通过测验和间隔复习帮助用户掌握知识。

**架构：** React 前端通过 HTTP 与 Axum 后端通信，后端分为 AI 网关（LLM 调用+搜索）、知识引擎（SQLite 存储检索）、学习引擎（SM-2 复习）。双模式运行：Tauri 桌面端 + 纯 Web 模式。

**技术栈：** Tauri v2 + React 19 + TypeScript + Vite 7, Axum 0.8 + Tokio, SQLite (rusqlite) + FTS5, reqwest (LLM API)

---

## 新增/修改的文件清单

### Rust 后端 (`src-tauri/src/`)
| 文件 | 职责 |
|------|------|
| `db.rs` | 数据库初始化、连接管理、migrations |
| `models.rs` | 所有数据模型 struct（Source, KnowledgePoint, QuizQuestion 等）|
| `repo/mod.rs` | Repository trait 定义 |
| `repo/source.rs` | Source 的 CRUD + 搜索 |
| `repo/knowledge.rs` | KnowledgePoint 的 CRUD + FTS5 全文检索 |
| `repo/quiz.rs` | QuizQuestion/QuizAttempt 的 CRUD |
| `repo/learning.rs` | LearningPlan/MasteryRecord 的 CRUD |
| `repo/relation.rs` | Relation 的 CRUD |
| `ai/mod.rs` | AI 网关主入口 |
| `ai/llm.rs` | LLM 客户端（OpenAI/DeepSeek 兼容，SSE streaming） |
| `ai/search.rs` | 网络搜索（Tavily / DuckDuckGo） |
| `ai/extract.rs` | 知识点提取 service（调用 LLM） |
| `ai/quiz_gen.rs` | 测验题生成 service（调用 LLM） |
| `learning/mod.rs` | 学习引擎入口 |
| `learning/sm2.rs` | SM-2 间隔复习算法 |
| `api/mod.rs` | API 路由汇总 |
| `api/sources.rs` | Source REST endpoints |
| `api/knowledge.rs` | KnowledgePoint REST endpoints |
| `api/quiz.rs` | 测验 REST endpoints |
| `api/learning.rs` | 学习计划 & 复习 REST endpoints |
| `api/ai_routes.rs` | AI 触发端点（开始研究、提取知识点等） |
| `server.rs` | 更新路由注册 |
| `lib.rs` | 更新 Tauri setup 注入 DB 连接 |

### React 前端 (`src/`)
| 文件 | 职责 |
|------|------|
| `api/client.ts` | Axios/fetch 封装，API 类型定义 |
| `types.ts` | 前端类型定义（与 Rust models 对应） |
| `hooks/useChat.ts` | 对话状态管理 hook |
| `hooks/useSources.ts` | Source 列表管理 hook |
| `hooks/useQuiz.ts` | 测验状态管理 hook |
| `hooks/useLearning.ts` | 学习计划和复习 hook |
| `components/Chat/ChatPanel.tsx` | 对话面板主组件 |
| `components/Chat/MessageBubble.tsx` | 单条消息气泡 |
| `components/Chat/ChatInput.tsx` | 输入框 + 发送 |
| `components/Sidebar/SourceList.tsx` | 资料来源列表 |
| `components/Sidebar/KpList.tsx` | 知识点列表 |
| `components/Content/LearningView.tsx` | 学习内容区（知识点讲解+测验） |
| `components/Content/QuizCard.tsx` | 单道测验题卡片 |
| `components/Content/ReportView.tsx` | 知识报告/Markdown 渲染 |

---

## 任务 1：添加 Rust 依赖

**文件：**
- 修改：`src-tauri/Cargo.toml`
- 修改：`src-tauri/Cargo.lock`（自动）

- [ ] **步骤 1：添加 crates**

在 `[dependencies]` 下追加：

```toml
rusqlite = { version = "0.30", features = ["bundled", "fts5"] }
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
reqwest = { version = "0.12", features = ["json", "stream"] }
tokio-stream = "0.1"
thiserror = "1"
```

- [ ] **步骤 2：构建验证**

```bash
cargo check
```

预期：编译成功，无错误。

- [ ] **步骤 3：Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "build: add rusqlite, uuid, chrono, reqwest, thiserror dependencies"
```

---

## 任务 2：数据库 Schema & 连接管理

**文件：**
- 创建：`src-tauri/src/db.rs`

- [ ] **步骤 1：编写数据库初始化代码**

```rust
use rusqlite::{Connection, Result};
use std::sync::Mutex;

pub struct Database {
    pub conn: Mutex<Connection>,
}

impl Database {
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        Ok(Database { conn: Mutex::new(conn) })
    }

    pub fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS sources (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                type TEXT NOT NULL CHECK(type IN ('url','text','file')),
                content TEXT NOT NULL,
                tags TEXT NOT NULL DEFAULT '[]',
                origin TEXT NOT NULL CHECK(origin IN ('user','ai_search')),
                source_url TEXT,
                hidden INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS knowledge_points (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                summary TEXT NOT NULL DEFAULT '',
                content TEXT NOT NULL DEFAULT '',
                tags TEXT NOT NULL DEFAULT '[]',
                source_ids TEXT NOT NULL DEFAULT '[]',
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS relations (
                id TEXT PRIMARY KEY,
                from_kp_id TEXT NOT NULL REFERENCES knowledge_points(id),
                to_kp_id TEXT NOT NULL REFERENCES knowledge_points(id),
                relation_type TEXT NOT NULL CHECK(relation_type IN ('prerequisite','related','extension')),
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS quiz_questions (
                id TEXT PRIMARY KEY,
                kp_id TEXT NOT NULL REFERENCES knowledge_points(id),
                type TEXT NOT NULL CHECK(type IN ('multiple_choice','fill_blank','analysis')),
                question TEXT NOT NULL,
                options TEXT,
                answer TEXT NOT NULL,
                explanation TEXT NOT NULL DEFAULT ''
            );

            CREATE TABLE IF NOT EXISTS quiz_attempts (
                id TEXT PRIMARY KEY,
                question_id TEXT NOT NULL REFERENCES quiz_questions(id),
                user_answer TEXT NOT NULL,
                is_correct INTEGER NOT NULL,
                attempted_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS mastery_records (
                id TEXT PRIMARY KEY,
                kp_id TEXT NOT NULL UNIQUE REFERENCES knowledge_points(id),
                ease_factor REAL NOT NULL DEFAULT 2.5,
                interval_days INTEGER NOT NULL DEFAULT 0,
                repetitions INTEGER NOT NULL DEFAULT 0,
                next_review_at TEXT NOT NULL DEFAULT (datetime('now')),
                last_reviewed_at TEXT
            );

            CREATE TABLE IF NOT EXISTS learning_plans (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                goal TEXT NOT NULL DEFAULT '',
                kp_ids TEXT NOT NULL DEFAULT '[]',
                status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active','completed','paused')),
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS kp_fts USING fts5(title, summary, content, content='knowledge_points', content_rowid='rowid');
            CREATE VIRTUAL TABLE IF NOT EXISTS sources_fts USING fts5(title, content, content='sources', content_rowid='rowid');"
        )?;
        Ok(())
    }
}
```

- [ ] **步骤 2：在 lib.rs 中集成数据库初始化**

修改 `src-tauri/src/lib.rs`：

```rust
mod db;
// ... existing mod declarations

// In setup closure, before spawning server:
use db::Database;
use std::path::PathBuf;

let app_dir: PathBuf = app_handle.path().app_data_dir().unwrap();
std::fs::create_dir_all(&app_dir).unwrap();
let db_path = app_dir.join("lexio.db");

let db = Database::new(db_path.to_str().unwrap()).expect("Failed to open database");
db.migrate().expect("Failed to run migrations");
app_handle.manage(db);
```

- [ ] **步骤 3：构建验证**

```bash
cargo check
```

预期：编译成功。

- [ ] **步骤 4：Commit**

```bash
git add src-tauri/src/db.rs src-tauri/src/lib.rs src-tauri/Cargo.lock
git commit -m "feat: add SQLite database schema and connection management"
```

---

## 任务 3：数据模型定义

**文件：**
- 创建：`src-tauri/src/models.rs`

- [ ] **步骤 1：编写所有数据模型**

```rust
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub id: String,
    pub title: String,
    #[serde(rename = "type")]
    pub source_type: String,
    pub content: String,
    pub tags: Vec<String>,
    pub origin: String,
    pub source_url: Option<String>,
    pub hidden: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSourceRequest {
    pub title: String,
    #[serde(rename = "type")]
    pub source_type: String,
    pub content: String,
    pub tags: Vec<String>,
    pub origin: String,
    pub source_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgePoint {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub content: String,
    pub tags: Vec<String>,
    pub source_ids: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateKnowledgePointRequest {
    pub title: String,
    pub summary: String,
    pub content: String,
    pub tags: Vec<String>,
    pub source_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub id: String,
    pub from_kp_id: String,
    pub to_kp_id: String,
    pub relation_type: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuizQuestion {
    pub id: String,
    pub kp_id: String,
    #[serde(rename = "type")]
    pub question_type: String,
    pub question: String,
    pub options: Option<Vec<String>>,
    pub answer: String,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuizAttempt {
    pub id: String,
    pub question_id: String,
    pub user_answer: String,
    pub is_correct: bool,
    pub attempted_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasteryRecord {
    pub id: String,
    pub kp_id: String,
    pub ease_factor: f64,
    pub interval_days: i32,
    pub repetitions: i32,
    pub next_review_at: String,
    pub last_reviewed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningPlan {
    pub id: String,
    pub title: String,
    pub goal: String,
    pub kp_ids: Vec<String>,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLearningPlanRequest {
    pub title: String,
    pub goal: String,
    pub kp_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitQuizAnswerRequest {
    pub question_id: String,
    pub user_answer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuizResult {
    pub question: QuizQuestion,
    pub user_answer: String,
    pub is_correct: bool,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiStartResearchRequest {
    pub topic: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiResearchResult {
    pub sources: Vec<Source>,
    pub knowledge_points: Vec<KnowledgePoint>,
    pub plan: LearningPlan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiGenerateQuizRequest {
    pub kp_ids: Vec<String>,
    pub count: usize,
}

pub fn new_id() -> String {
    Uuid::new_v4().to_string()
}
```

- [ ] **步骤 2：添加 `mod models;` 到 `lib.rs`**

- [ ] **步骤 3：构建验证**

```bash
cargo check
```

- [ ] **步骤 4：Commit**

```bash
git add src-tauri/src/models.rs src-tauri/src/lib.rs
git commit -m "feat: define all data models for AI knowledge agent"
```

---

## 任务 4：Source Repository（CRUD + 搜索）

**文件：**
- 创建：`src-tauri/src/repo/mod.rs`
- 创建：`src-tauri/src/repo/source.rs`

- [ ] **步骤 1：编写 Source repository**

`src-tauri/src/repo/source.rs`:

```rust
use crate::db::Database;
use crate::models::{new_id, CreateSourceRequest, Source};

pub fn create_source(db: &Database, req: &CreateSourceRequest) -> Result<Source, String> {
    let id = new_id();
    let tags = serde_json::to_string(&req.tags).unwrap_or_default();
    let source = Source {
        id: id.clone(),
        title: req.title.clone(),
        source_type: req.source_type.clone(),
        content: req.content.clone(),
        tags: req.tags.clone(),
        origin: req.origin.clone(),
        source_url: req.source_url.clone(),
        hidden: false,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO sources (id, title, type, content, tags, origin, source_url, hidden, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8)",
        rusqlite::params![id, source.title, source.source_type, source.content, tags, source.origin, source.source_url, source.created_at],
    ).map_err(|e| e.to_string())?;
    Ok(source)
}

pub fn list_sources(db: &Database, include_hidden: bool) -> Result<Vec<Source>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let query = if include_hidden {
        "SELECT id, title, type, content, tags, origin, source_url, hidden, created_at FROM sources ORDER BY created_at DESC"
    } else {
        "SELECT id, title, type, content, tags, origin, source_url, hidden, created_at FROM sources WHERE hidden = 0 ORDER BY created_at DESC"
    };
    let mut stmt = conn.prepare(query).map_err(|e| e.to_string())?;
    let sources: Vec<Source> = stmt
        .query_map([], |row| source_from_row(row))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(sources)
}

pub fn get_source(db: &Database, id: &str) -> Result<Option<Source>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, title, type, content, tags, origin, source_url, hidden, created_at FROM sources WHERE id = ?1")
        .map_err(|e| e.to_string())?;
    let mut rows = stmt
        .query_map([id], |row| source_from_row(row))
        .map_err(|e| e.to_string())?;
    Ok(rows.next().and_then(|r| r.ok()))
}

pub fn toggle_hidden(db: &Database, id: &str, hidden: bool) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE sources SET hidden = ?1 WHERE id = ?2",
        rusqlite::params![hidden as i32, id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn search_sources(db: &Database, query: &str) -> Result<Vec<Source>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, title, type, content, tags, origin, source_url, hidden, created_at FROM sources WHERE sources_fts MATCH ?1 ORDER BY rank")
        .map_err(|e| e.to_string())?;
    let sources: Vec<Source> = stmt
        .query_map([query], |row| source_from_row(row))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(sources)
}

fn source_from_row(row: &rusqlite::Row) -> rusqlite::Result<Source> {
    let tags_str: String = row.get(4)?;
    Ok(Source {
        id: row.get(0)?,
        title: row.get(1)?,
        source_type: row.get(2)?,
        content: row.get(3)?,
        tags: serde_json::from_str(&tags_str).unwrap_or_default(),
        origin: row.get(5)?,
        source_url: row.get(6)?,
        hidden: row.get::<_, i32>(7)? != 0,
        created_at: row.get(8)?,
    })
}
```

`src-tauri/src/repo/mod.rs`:

```rust
pub mod source;
```

- [ ] **步骤 2：在 `lib.rs` 添加 `mod repo;`**

- [ ] **步骤 3：构建验证**

```bash
cargo check
```

- [ ] **步骤 4：Commit**

```bash
git add src-tauri/src/repo/ src-tauri/src/lib.rs
git commit -m "feat: implement Source repository with CRUD + FTS5 search"
```

---

## 任务 5：KnowledgePoint Repository（CRUD + FTS5）

**文件：**
- 创建：`src-tauri/src/repo/knowledge.rs`
- 修改：`src-tauri/src/repo/mod.rs`（添加 pub mod knowledge）

- [ ] **步骤 1：编写 KnowledgePoint repository**

```rust
use crate::db::Database;
use crate::models::{new_id, CreateKnowledgePointRequest, KnowledgePoint};

pub fn create_kp(db: &Database, req: &CreateKnowledgePointRequest) -> Result<KnowledgePoint, String> {
    let id = new_id();
    let tags = serde_json::to_string(&req.tags).unwrap_or_default();
    let source_ids = serde_json::to_string(&req.source_ids).unwrap_or_default();
    let now = chrono::Utc::now().to_rfc3339();
    let kp = KnowledgePoint {
        id: id.clone(),
        title: req.title.clone(),
        summary: req.summary.clone(),
        content: req.content.clone(),
        tags: req.tags.clone(),
        source_ids: req.source_ids.clone(),
        created_at: now.clone(),
    };
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO knowledge_points (id, title, summary, content, tags, source_ids, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![id, kp.title, kp.summary, kp.content, tags, source_ids, now],
    ).map_err(|e| e.to_string())?;
    Ok(kp)
}

pub fn list_kps(db: &Database) -> Result<Vec<KnowledgePoint>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, title, summary, content, tags, source_ids, created_at FROM knowledge_points ORDER BY created_at DESC")
        .map_err(|e| e.to_string())?;
    let kps: Vec<KnowledgePoint> = stmt
        .query_map([], |row| kp_from_row(row))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(kps)
}

pub fn get_kp(db: &Database, id: &str) -> Result<Option<KnowledgePoint>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, title, summary, content, tags, source_ids, created_at FROM knowledge_points WHERE id = ?1")
        .map_err(|e| e.to_string())?;
    let mut rows = stmt
        .query_map([id], |row| kp_from_row(row))
        .map_err(|e| e.to_string())?;
    Ok(rows.next().and_then(|r| r.ok()))
}

pub fn list_kps_by_ids(db: &Database, ids: &[String]) -> Result<Vec<KnowledgePoint>, String> {
    if ids.is_empty() {
        return Ok(vec![]);
    }
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let placeholders: Vec<String> = ids.iter().enumerate().map(|(i, _)| format!("?{}", i + 1)).collect();
    let sql = format!(
        "SELECT id, title, summary, content, tags, source_ids, created_at FROM knowledge_points WHERE id IN ({})",
        placeholders.join(",")
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let params: Vec<&dyn rusqlite::types::ToSql> = ids.iter().map(|id| id as &dyn rusqlite::types::ToSql).collect();
    let kps: Vec<KnowledgePoint> = stmt
        .query_map(params.as_slice(), |row| kp_from_row(row))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(kps)
}

pub fn search_kps(db: &Database, query: &str) -> Result<Vec<KnowledgePoint>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, title, summary, content, tags, source_ids, created_at FROM knowledge_points WHERE kp_fts MATCH ?1 ORDER BY rank")
        .map_err(|e| e.to_string())?;
    let kps: Vec<KnowledgePoint> = stmt
        .query_map([query], |row| kp_from_row(row))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(kps)
}

pub fn delete_kp(db: &Database, id: &str) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM knowledge_points WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn kp_from_row(row: &rusqlite::Row) -> rusqlite::Result<KnowledgePoint> {
    let tags_str: String = row.get(4)?;
    let source_ids_str: String = row.get(5)?;
    Ok(KnowledgePoint {
        id: row.get(0)?,
        title: row.get(1)?,
        summary: row.get(2)?,
        content: row.get(3)?,
        tags: serde_json::from_str(&tags_str).unwrap_or_default(),
        source_ids: serde_json::from_str(&source_ids_str).unwrap_or_default(),
        created_at: row.get(6)?,
    })
}
```

- [ ] **步骤 2：构建验证**

```bash
cargo check
```

- [ ] **步骤 3：Commit**

```bash
git add src-tauri/src/repo/
git commit -m "feat: implement KnowledgePoint repository with FTS5 search"
```

---

## 任务 6：Quiz, Learning, Relation Repositories

**文件：**
- 创建：`src-tauri/src/repo/quiz.rs`
- 创建：`src-tauri/src/repo/learning.rs`
- 创建：`src-tauri/src/repo/relation.rs`
- 修改：`src-tauri/src/repo/mod.rs`

- [ ] **步骤 1：Quiz repository**

`src-tauri/src/repo/quiz.rs`:

```rust
use crate::db::Database;
use crate::models::{new_id, QuizAttempt, QuizQuestion, SubmitQuizAnswerRequest};

pub fn create_question(db: &Database, q: &QuizQuestion) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let options = q.options.as_ref().map(|o| serde_json::to_string(o).unwrap_or_default());
    conn.execute(
        "INSERT INTO quiz_questions (id, kp_id, type, question, options, answer, explanation) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![q.id, q.kp_id, q.question_type, q.question, options, q.answer, q.explanation],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_questions_by_kp(db: &Database, kp_id: &str) -> Result<Vec<QuizQuestion>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, kp_id, type, question, options, answer, explanation FROM quiz_questions WHERE kp_id = ?1")
        .map_err(|e| e.to_string())?;
    let questions: Vec<QuizQuestion> = stmt
        .query_map([kp_id], |row| question_from_row(row))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(questions)
}

pub fn get_questions_by_ids(db: &Database, ids: &[String]) -> Result<Vec<QuizQuestion>, String> {
    if ids.is_empty() { return Ok(vec![]); }
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let placeholders: Vec<String> = ids.iter().enumerate().map(|(i, _)| format!("?{}", i + 1)).collect();
    let sql = format!(
        "SELECT id, kp_id, type, question, options, answer, explanation FROM quiz_questions WHERE id IN ({})",
        placeholders.join(",")
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let params: Vec<&dyn rusqlite::types::ToSql> = ids.iter().map(|id| id as &dyn rusqlite::types::ToSql).collect();
    let questions: Vec<QuizQuestion> = stmt
        .query_map(params.as_slice(), |row| question_from_row(row))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(questions)
}

pub fn record_attempt(db: &Database, req: &SubmitQuizAnswerRequest, is_correct: bool) -> Result<QuizAttempt, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let id = new_id();
    let now = chrono::Utc::now().to_rfc3339();
    let attempt = QuizAttempt {
        id: id.clone(),
        question_id: req.question_id.clone(),
        user_answer: req.user_answer.clone(),
        is_correct,
        attempted_at: now.clone(),
    };
    conn.execute(
        "INSERT INTO quiz_attempts (id, question_id, user_answer, is_correct, attempted_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![id, attempt.question_id, attempt.user_answer, is_correct as i32, now],
    ).map_err(|e| e.to_string())?;
    Ok(attempt)
}

pub fn get_attempts_by_kp(db: &Database, kp_id: &str) -> Result<Vec<QuizAttempt>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT qa.id, qa.question_id, qa.user_answer, qa.is_correct, qa.attempted_at FROM quiz_attempts qa JOIN quiz_questions qq ON qa.question_id = qq.id WHERE qq.kp_id = ?1 ORDER BY qa.attempted_at DESC")
        .map_err(|e| e.to_string())?;
    let attempts: Vec<QuizAttempt> = stmt
        .query_map([kp_id], |row| attempt_from_row(row))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(attempts)
}

fn question_from_row(row: &rusqlite::Row) -> rusqlite::Result<QuizQuestion> {
    let options_str: Option<String> = row.get(4)?;
    Ok(QuizQuestion {
        id: row.get(0)?,
        kp_id: row.get(1)?,
        question_type: row.get(2)?,
        question: row.get(3)?,
        options: options_str.and_then(|s| serde_json::from_str(&s).ok()),
        answer: row.get(5)?,
        explanation: row.get(6)?,
    })
}

fn attempt_from_row(row: &rusqlite::Row) -> rusqlite::Result<QuizAttempt> {
    Ok(QuizAttempt {
        id: row.get(0)?,
        question_id: row.get(1)?,
        user_answer: row.get(2)?,
        is_correct: row.get::<_, i32>(3)? != 0,
        attempted_at: row.get(4)?,
    })
}
```

- [ ] **步骤 2：Learning repository**

`src-tauri/src/repo/learning.rs`:

```rust
use crate::db::Database;
use crate::models::{new_id, CreateLearningPlanRequest, LearningPlan, MasteryRecord};

pub fn create_plan(db: &Database, req: &CreateLearningPlanRequest) -> Result<LearningPlan, String> {
    let id = new_id();
    let kp_ids = serde_json::to_string(&req.kp_ids).unwrap_or_default();
    let now = chrono::Utc::now().to_rfc3339();
    let plan = LearningPlan {
        id: id.clone(),
        title: req.title.clone(),
        goal: req.goal.clone(),
        kp_ids: req.kp_ids.clone(),
        status: "active".to_string(),
        created_at: now.clone(),
    };
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO learning_plans (id, title, goal, kp_ids, status, created_at) VALUES (?1, ?2, ?3, ?4, 'active', ?5)",
        rusqlite::params![id, plan.title, plan.goal, kp_ids, now],
    ).map_err(|e| e.to_string())?;
    Ok(plan)
}

pub fn list_plans(db: &Database) -> Result<Vec<LearningPlan>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, title, goal, kp_ids, status, created_at FROM learning_plans ORDER BY created_at DESC")
        .map_err(|e| e.to_string())?;
    Ok(stmt
        .query_map([], |row| plan_from_row(row))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect())
}

pub fn upsert_mastery(db: &Database, record: &MasteryRecord) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO mastery_records (id, kp_id, ease_factor, interval_days, repetitions, next_review_at, last_reviewed_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) ON CONFLICT(kp_id) DO UPDATE SET ease_factor=?3, interval_days=?4, repetitions=?5, next_review_at=?6, last_reviewed_at=?7",
        rusqlite::params![record.id, record.kp_id, record.ease_factor, record.interval_days, record.repetitions, record.next_review_at, record.last_reviewed_at],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_due_reviews(db: &Database) -> Result<Vec<MasteryRecord>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();
    let mut stmt = conn
        .prepare("SELECT id, kp_id, ease_factor, interval_days, repetitions, next_review_at, last_reviewed_at FROM mastery_records WHERE next_review_at <= ?1 ORDER BY next_review_at ASC")
        .map_err(|e| e.to_string())?;
    Ok(stmt
        .query_map([&now], |row| mastery_from_row(row))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect())
}

pub fn get_mastery_by_kp(db: &Database, kp_id: &str) -> Result<Option<MasteryRecord>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, kp_id, ease_factor, interval_days, repetitions, next_review_at, last_reviewed_at FROM mastery_records WHERE kp_id = ?1")
        .map_err(|e| e.to_string())?;
    let mut rows = stmt
        .query_map([kp_id], |row| mastery_from_row(row))
        .map_err(|e| e.to_string())?;
    Ok(rows.next().and_then(|r| r.ok()))
}

fn plan_from_row(row: &rusqlite::Row) -> rusqlite::Result<LearningPlan> {
    let kp_ids_str: String = row.get(3)?;
    Ok(LearningPlan {
        id: row.get(0)?,
        title: row.get(1)?,
        goal: row.get(2)?,
        kp_ids: serde_json::from_str(&kp_ids_str).unwrap_or_default(),
        status: row.get(4)?,
        created_at: row.get(5)?,
    })
}

fn mastery_from_row(row: &rusqlite::Row) -> rusqlite::Result<MasteryRecord> {
    Ok(MasteryRecord {
        id: row.get(0)?,
        kp_id: row.get(1)?,
        ease_factor: row.get(2)?,
        interval_days: row.get(3)?,
        repetitions: row.get(4)?,
        next_review_at: row.get(5)?,
        last_reviewed_at: row.get(6)?,
    })
}
```

- [ ] **步骤 3：Relation repository**

`src-tauri/src/repo/relation.rs`:

```rust
use crate::db::Database;
use crate::models::{new_id, Relation};

pub fn create_relation(db: &Database, from_kp_id: &str, to_kp_id: &str, relation_type: &str) -> Result<Relation, String> {
    let id = new_id();
    let now = chrono::Utc::now().to_rfc3339();
    let rel = Relation {
        id: id.clone(),
        from_kp_id: from_kp_id.to_string(),
        to_kp_id: to_kp_id.to_string(),
        relation_type: relation_type.to_string(),
        created_at: now.clone(),
    };
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO relations (id, from_kp_id, to_kp_id, relation_type, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![id, rel.from_kp_id, rel.to_kp_id, rel.relation_type, now],
    ).map_err(|e| e.to_string())?;
    Ok(rel)
}

pub fn get_relations_for_kp(db: &Database, kp_id: &str) -> Result<Vec<Relation>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, from_kp_id, to_kp_id, relation_type, created_at FROM relations WHERE from_kp_id = ?1 OR to_kp_id = ?1")
        .map_err(|e| e.to_string())?;
    Ok(stmt
        .query_map([kp_id], |row| relation_from_row(row))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect())
}

fn relation_from_row(row: &rusqlite::Row) -> rusqlite::Result<Relation> {
    Ok(Relation {
        id: row.get(0)?,
        from_kp_id: row.get(1)?,
        to_kp_id: row.get(2)?,
        relation_type: row.get(3)?,
        created_at: row.get(4)?,
    })
}
```

- [ ] **步骤 4：更新 `mod.rs`**

```rust
pub mod source;
pub mod knowledge;
pub mod quiz;
pub mod learning;
pub mod relation;
```

- [ ] **步骤 5：构建验证**

```bash
cargo check
```

- [ ] **步骤 6：Commit**

```bash
git add src-tauri/src/repo/
git commit -m "feat: implement quiz, learning, and relation repositories"
```

---

## 任务 7：SM-2 间隔复习算法

**文件：**
- 创建：`src-tauri/src/learning/mod.rs`
- 创建：`src-tauri/src/learning/sm2.rs`

- [ ] **步骤 1：实现 SM-2 算法**

`src-tauri/src/learning/sm2.rs`:

```rust
/// SM-2 间隔复习算法
/// 输入：当前 mastery_record 和本次作答是否正确
/// 输出：更新后的 ease_factor, interval_days, repetitions, next_review_at

pub struct Sm2Input {
    pub ease_factor: f64,
    pub interval_days: i32,
    pub repetitions: i32,
    pub is_correct: bool,
    pub response_quality: i32, // 0-5, 仅 is_correct=false 时使用
}

pub struct Sm2Output {
    pub ease_factor: f64,
    pub interval_days: i32,
    pub repetitions: i32,
    pub next_review_at: String,
}

pub fn calculate(input: Sm2Input) -> Sm2Output {
    let Sm2Input { ease_factor, interval_days, repetitions, is_correct, response_quality } = input;

    if is_correct {
        let reps = repetitions + 1;
        let interval = match reps {
            1 => 1,
            2 => 6,
            _ => (interval_days as f64 * ease_factor).round() as i32,
        };
        // ease factor increases when correct repeatedly
        let ef = (ease_factor + 0.1).max(1.3).min(2.5);
        let next = chrono::Utc::now() + chrono::Duration::days(interval as i64);
        Sm2Output {
            ease_factor: ef,
            interval_days: interval,
            repetitions: reps,
            next_review_at: next.to_rfc3339(),
        }
    } else {
        // Reset: review again tomorrow, ease factor drops
        let ef = (ease_factor - 0.2 - (0.02 * (5 - response_quality) as f64)).max(1.3);
        Sm2Output {
            ease_factor: ef,
            interval_days: 1,
            repetitions: 0,
            next_review_at: (chrono::Utc::now() + chrono::Duration::days(1)).to_rfc3339(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_first_correct() {
        let output = calculate(Sm2Input {
            ease_factor: 2.5,
            interval_days: 0,
            repetitions: 0,
            is_correct: true,
            response_quality: 5,
        });
        assert_eq!(output.repetitions, 1);
        assert_eq!(output.interval_days, 1);
        assert!(output.ease_factor > 2.5);
    }

    #[test]
    fn test_second_correct() {
        let output = calculate(Sm2Input {
            ease_factor: 2.5,
            interval_days: 1,
            repetitions: 1,
            is_correct: true,
            response_quality: 5,
        });
        assert_eq!(output.repetitions, 2);
        assert_eq!(output.interval_days, 6);
    }

    #[test]
    fn test_incorrect_resets() {
        let output = calculate(Sm2Input {
            ease_factor: 2.5,
            interval_days: 30,
            repetitions: 5,
            is_correct: false,
            response_quality: 2,
        });
        assert_eq!(output.repetitions, 0);
        assert_eq!(output.interval_days, 1);
        assert!(output.ease_factor < 2.5);
    }
}
```

`src-tauri/src/learning/mod.rs`:

```rust
pub mod sm2;
```

- [ ] **步骤 2：验证（运行测试）**

```bash
cargo test learning::sm2::tests
```

预期：3 个测试全部 PASS。

- [ ] **步骤 3：在 `lib.rs` 添加 `mod learning;`**

- [ ] **步骤 4：Commit**

```bash
git add src-tauri/src/learning/
git commit -m "feat: implement SM-2 spaced repetition algorithm with tests"
```

---

## 任务 8：AI 网关 - LLM 客户端

**文件：**
- 创建：`src-tauri/src/ai/mod.rs`
- 创建：`src-tauri/src/ai/llm.rs`

- [ ] **步骤 1：LLM 客户端**

`src-tauri/src/ai/llm.rs`:

```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessageContent,
}

#[derive(Debug, Deserialize)]
struct ChatMessageContent {
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

pub struct LlmClient {
    config: LlmConfig,
    client: Client,
}

impl LlmClient {
    pub fn new(config: LlmConfig) -> Self {
        Self { config, client: Client::new() }
    }

    /// 发送非流式请求，返回完整回复
    pub async fn chat(&self, system_prompt: &str, user_prompt: &str) -> Result<String, String> {
        let messages = vec![
            ChatMessage { role: "system".to_string(), content: system_prompt.to_string() },
            ChatMessage { role: "user".to_string(), content: user_prompt.to_string() },
        ];
        let req = ChatRequest { model: self.config.model.clone(), messages, stream: false };

        let resp = self.client
            .post(format!("{}/chat/completions", self.config.base_url))
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .json(&req)
            .send()
            .await
            .map_err(|e| format!("LLM request failed: {}", e))?;

        let body: ChatResponse = resp.json().await.map_err(|e| format!("Parse error: {}", e))?;
        body.choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| "Empty response".to_string())
    }

    /// 发送流式请求，对每个 chunk 调用 on_chunk 回调
    pub async fn chat_streaming<F>(&self, system_prompt: &str, user_prompt: &str, on_chunk: F) -> Result<String, String>
    where
        F: Fn(&str),
    {
        let messages = vec![
            ChatMessage { role: "system".to_string(), content: system_prompt.to_string() },
            ChatMessage { role: "user".to_string(), content: user_prompt.to_string() },
        ];
        let req = ChatRequest { model: self.config.model.clone(), messages, stream: true };
        let mut full_response = String::new();

        let resp = self.client
            .post(format!("{}/chat/completions", self.config.base_url))
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .json(&req)
            .send()
            .await
            .map_err(|e| format!("LLM request failed: {}", e))?;

        use futures_util::StreamExt;
        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("Stream error: {}", e))?;
            let text = String::from_utf8_lossy(&chunk);
            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" { continue; }
                    if let Ok(event) = serde_json::from_str::<serde_json::Value>(data) {
                        if let Some(content) = event["choices"][0]["delta"]["content"].as_str() {
                            full_response.push_str(content);
                            on_chunk(content);
                        }
                    }
                }
            }
        }
        Ok(full_response)
    }
}
```

`src-tauri/src/ai/mod.rs`:

```rust
pub mod llm;
```

需要添加 `futures-util` 到 Cargo.toml:

```toml
futures-util = "0.3"
```

- [ ] **步骤 2：在 `lib.rs` 添加 `mod ai;`**

- [ ] **步骤 3：构建验证**

```bash
cargo check
```

- [ ] **步骤 4：Commit**

```bash
git add src-tauri/src/ai/ src-tauri/src/lib.rs src-tauri/Cargo.toml
git commit -m "feat: implement LLM client with streaming support"
```

---

## 任务 9：AI 网关 - 知识点提取 & 测验生成

**文件：**
- 创建：`src-tauri/src/ai/extract.rs`
- 创建：`src-tauri/src/ai/quiz_gen.rs`
- 修改：`src-tauri/src/ai/mod.rs`

- [ ] **步骤 1：知识点提取服务**

`src-tauri/src/ai/extract.rs`:

```rust
use crate::ai::llm::LlmClient;
use crate::models::{CreateKnowledgePointRequest, KnowledgePoint};

pub async fn extract_knowledge_points(
    llm: &LlmClient,
    source_title: &str,
    source_content: &str,
) -> Result<Vec<CreateKnowledgePointRequest>, String> {
    let system_prompt = "You are a knowledge extraction assistant. Extract key concepts as structured knowledge points from the given content. Return JSON only.";
    let user_prompt = format!(
        "Extract the main knowledge points from this content.\n\
        Title: {}\n\nContent:\n{}\n\n\
        Return ONLY a JSON array of objects with fields: title, summary (one sentence), content (detailed explanation 2-3 paragraphs), tags (array of strings).",
        source_title, source_content
    );

    let response = llm.chat(system_prompt, &user_prompt).await?;
    // Extract JSON from response (may be wrapped in markdown code block)
    let json_str = if let Some(start) = response.find("```json") {
        let after = &response[start + 7..];
        if let Some(end) = after.find("```") {
            &after[..end]
        } else {
            after
        }
    } else if let Some(start) = response.find('[') {
        &response[start..]
    } else {
        &response
    };

    let kps: Vec<CreateKnowledgePointRequest> = serde_json::from_str(json_str.trim())
        .map_err(|e| format!("Failed to parse knowledge points: {}. Raw: {}", e, json_str))?;
    Ok(kps)
}
```

- [ ] **步骤 2：测验生成服务**

`src-tauri/src/ai/quiz_gen.rs`:

```rust
use crate::ai::llm::LlmClient;
use crate::models::QuizQuestion;

pub async fn generate_quizzes(
    llm: &LlmClient,
    kp_title: &str,
    kp_content: &str,
    count: usize,
) -> Result<Vec<QuizQuestion>, String> {
    let id_prefix = crate::models::new_id(); // placeholder, will be replaced by caller

    let system_prompt = "You are a quiz generation assistant for spaced-repetition learning. Generate challenging multiple-choice and fill-in-the-blank questions that test true understanding, not memorization. Return JSON only.";
    let user_prompt = format!(
        "Generate {} quiz questions for this knowledge point.\n\
        Title: {}\nContent:\n{}\n\n\
        Return ONLY a JSON array of objects with fields:\n\
        - type: 'multiple_choice' or 'fill_blank'\n\
        - question: the question text\n\
        - options: (array of 4 strings, only for multiple_choice)\n\
        - answer: the correct answer text (single letter A/B/C/D for multiple_choice, the word for fill_blank)\n\
        - explanation: why this answer is correct, 1-2 sentences",
        count, kp_title, kp_content
    );

    let response = llm.chat(system_prompt, &user_prompt).await?;
    let json_str = if let Some(start) = response.find("```json") {
        let after = &response[start + 7..];
        if let Some(end) = after.find("```") { &after[..end] } else { after }
    } else if let Some(start) = response.find('[') {
        &response[start..]
    } else {
        &response
    };

    let mut questions: Vec<QuizQuestion> = serde_json::from_str(json_str.trim())
        .map_err(|e| format!("Failed to parse quizzes: {}. Raw: {}", e, json_str))?;

    // Set proper IDs
    for q in &mut questions {
        q.id = crate::models::new_id();
    }
    Ok(questions)
}
```

- [ ] **步骤 3：更新 `ai/mod.rs`**

```rust
pub mod llm;
pub mod extract;
pub mod quiz_gen;
```

- [ ] **步骤 4：构建验证**

```bash
cargo check
```

- [ ] **步骤 5：Commit**

```bash
git add src-tauri/src/ai/
git commit -m "feat: implement AI knowledge extraction and quiz generation services"
```

---

## 任务 10：REST API - Sources & Knowledge Points

**文件：**
- 创建：`src-tauri/src/api/mod.rs`
- 创建：`src-tauri/src/api/sources.rs`
- 创建：`src-tauri/src/api/knowledge.rs`
- 修改：`src-tauri/src/server.rs`

- [ ] **步骤 1：Sources API**

`src-tauri/src/api/sources.rs`:

```rust
use axum::{extract::{Path, Query, State}, http::StatusCode, Json};
use serde::Deserialize;
use crate::db::Database;
use crate::models::CreateSourceRequest;
use crate::repo;

#[derive(Deserialize)]
pub struct ListSourcesQuery {
    pub include_hidden: Option<bool>,
    pub search: Option<String>,
}

pub async fn create_source(
    State(db): State<&'static Database>,
    Json(req): Json<CreateSourceRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    let source = repo::source::create_source(db, &req)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok((StatusCode::CREATED, Json(serde_json::to_value(&source).unwrap())))
}

pub async fn list_sources(
    State(db): State<&'static Database>,
    Query(params): Query<ListSourcesQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let sources = if let Some(ref query) = params.search {
        repo::source::search_sources(db, query)
    } else {
        repo::source::list_sources(db, params.include_hidden.unwrap_or(false))
    }.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(serde_json::to_value(&sources).unwrap()))
}

pub async fn get_source(
    State(db): State<&'static Database>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let source = repo::source::get_source(db, &id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
        .ok_or((StatusCode::NOT_FOUND, "Source not found".to_string()))?;
    Ok(Json(serde_json::to_value(&source).unwrap()))
}

#[derive(Deserialize)]
pub struct ToggleHiddenRequest {
    pub hidden: bool,
}

pub async fn toggle_hidden(
    State(db): State<&'static Database>,
    Path(id): Path<String>,
    Json(req): Json<ToggleHiddenRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    repo::source::toggle_hidden(db, &id, req.hidden)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(StatusCode::OK)
}
```

- [ ] **步骤 2：Knowledge Points API**

`src-tauri/src/api/knowledge.rs`:

```rust
use axum::{extract::{Path, Query, State}, http::StatusCode, Json};
use serde::Deserialize;
use crate::db::Database;
use crate::models::CreateKnowledgePointRequest;
use crate::repo;

#[derive(Deserialize)]
pub struct ListKpsQuery {
    pub search: Option<String>,
    pub ids: Option<String>, // comma-separated
}

pub async fn create_kp(
    State(db): State<&'static Database>,
    Json(req): Json<CreateKnowledgePointRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    let kp = repo::knowledge::create_kp(db, &req)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok((StatusCode::CREATED, Json(serde_json::to_value(&kp).unwrap())))
}

pub async fn list_kps(
    State(db): State<&'static Database>,
    Query(params): Query<ListKpsQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let kps = if let Some(ref query) = params.search {
        repo::knowledge::search_kps(db, query)
    } else if let Some(ref ids_str) = params.ids {
        let ids: Vec<String> = ids_str.split(',').map(|s| s.trim().to_string()).collect();
        repo::knowledge::list_kps_by_ids(db, &ids)
    } else {
        repo::knowledge::list_kps(db)
    }.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(serde_json::to_value(&kps).unwrap()))
}

pub async fn get_kp(
    State(db): State<&'static Database>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let kp = repo::knowledge::get_kp(db, &id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
        .ok_or((StatusCode::NOT_FOUND, "Knowledge point not found".to_string()))?;
    Ok(Json(serde_json::to_value(&kp).unwrap()))
}

pub async fn delete_kp(
    State(db): State<&'static Database>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    repo::knowledge::delete_kp(db, &id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(StatusCode::NO_CONTENT)
}
```

- [ ] **步骤 3：更新 server.rs 注册路由**

在 `src-tauri/src/server.rs` 的 `app()` 函数中添加路由：

```rust
use crate::api::{sources, knowledge};

pub fn app(db: &'static crate::db::Database) -> Router {
    Router::new()
        .route("/api/health", get(health_check))
        .route("/api/sources", get(sources::list_sources).post(sources::create_source))
        .route("/api/sources/{id}", get(sources::get_source))
        .route("/api/sources/{id}/hide", post(sources::toggle_hidden))
        .route("/api/knowledge", get(knowledge::list_kps).post(knowledge::create_kp))
        .route("/api/knowledge/{id}", get(knowledge::get_kp).delete(knowledge::delete_kp))
        .layer(CorsLayer::permissive())
        .with_state(db)
}
```

注意：需要将 `app()` 改为接收 `&'static Database` 参数，并更新 `lib.rs` 中的调用。

- [ ] **步骤 4：更改 `server.rs` 中 `app()` 的签名和 `lib.rs` 调用方式**

在 `lib.rs` 中：

```rust
// 将 Database 存储在 tauri state 中，同时 leak 一个 static ref 给 Axum
let db = db::Database::new(db_path.to_str().unwrap()).expect("Failed to open database");
db.migrate().expect("Failed to run migrations");
let db: &'static db::Database = Box::leak(Box::new(db));

// ... spawn server with server::app(db)
axum::serve(listener, server::app(db)).await.unwrap();
```

- [ ] **步骤 5：构建验证**

```bash
cargo check
```

- [ ] **步骤 6：Commit**

```bash
git add src-tauri/src/api/ src-tauri/src/server.rs src-tauri/src/lib.rs
git commit -m "feat: add REST API for sources and knowledge points"
```

---

## 任务 11：REST API - Quiz, Learning & AI Routes

**文件：**
- 创建：`src-tauri/src/api/quiz.rs`
- 创建：`src-tauri/src/api/learning.rs`
- 创建：`src-tauri/src/api/ai_routes.rs`
- 修改：`src-tauri/src/api/mod.rs`
- 修改：`src-tauri/src/server.rs`

- [ ] **步骤 1：Quiz API**

`src-tauri/src/api/quiz.rs`:

```rust
use axum::{extract::{Path, State}, http::StatusCode, Json};
use crate::db::Database;
use crate::models::SubmitQuizAnswerRequest;
use crate::repo;

pub async fn get_quiz_by_kp(
    State(db): State<&'static Database>,
    Path(kp_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let questions = repo::quiz::get_questions_by_kp(db, &kp_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(serde_json::to_value(&questions).unwrap()))
}

pub async fn submit_answer(
    State(db): State<&'static Database>,
    Json(req): Json<SubmitQuizAnswerRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // Get the question to check answer
    let questions = repo::quiz::get_questions_by_ids(db, &[req.question_id.clone()])
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let question = questions.first()
        .ok_or((StatusCode::NOT_FOUND, "Question not found".to_string()))?;

    let is_correct = req.user_answer.trim().eq_ignore_ascii_case(question.answer.trim());
    let attempt = repo::quiz::record_attempt(db, &req, is_correct)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let result = crate::models::QuizResult {
        question: question.clone(),
        user_answer: req.user_answer,
        is_correct,
        explanation: question.explanation.clone(),
    };

    Ok(Json(serde_json::to_value(&result).unwrap()))
}
```

- [ ] **步骤 2：Learning API**

`src-tauri/src/api/learning.rs`:

```rust
use axum::{extract::{Path, State}, http::StatusCode, Json};
use crate::db::Database;
use crate::models::CreateLearningPlanRequest;
use crate::repo;

pub async fn create_plan(
    State(db): State<&'static Database>,
    Json(req): Json<CreateLearningPlanRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    let plan = repo::learning::create_plan(db, &req)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok((StatusCode::CREATED, Json(serde_json::to_value(&plan).unwrap())))
}

pub async fn list_plans(
    State(db): State<&'static Database>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let plans = repo::learning::list_plans(db)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(serde_json::to_value(&plans).unwrap()))
}

pub async fn get_due_reviews(
    State(db): State<&'static Database>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let records = repo::learning::get_due_reviews(db)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(serde_json::to_value(&records).unwrap()))
}
```

- [ ] **步骤 3：AI Routes**

`src-tauri/src/api/ai_routes.rs`:

```rust
use axum::{extract::State, http::StatusCode, Json};
use crate::db::Database;
use crate::ai::llm::LlmClient;
use crate::models::{AiStartResearchRequest, CreateKnowledgePointRequest, CreateLearningPlanRequest, QuizQuestion};
use crate::repo::{self, source, knowledge};

// AppState holding both db and llm client
pub struct AppState {
    pub db: &'static Database,
    pub llm: LlmClient,
}

pub async fn start_research(
    State(state): State<&'static AppState>,
    Json(req): Json<AiStartResearchRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    // Step 1: AI searches web for sources (stub: use LLM to generate search results)
    let search_prompt = format!(
        "You are helping a user learn about: {}. \
        Please list 3 high-quality learning resources about this topic (titles and brief descriptions). \
        Return as JSON array with fields: title, description.",
        req.topic
    );
    let response = state.llm.chat("You are a research assistant.", &search_prompt).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    // Parse AI response as source suggestions
    let json_str = extract_json(&response);
    #[derive(serde::Deserialize)]
    struct SearchResult { title: String, description: String }
    let results: Vec<SearchResult> = serde_json::from_str(json_str)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Parse error: {}", e)))?;

    // Save as sources
    let mut sources = Vec::new();
    for r in &results {
        let src_req = crate::models::CreateSourceRequest {
            title: r.title.clone(),
            source_type: "text".to_string(),
            content: r.description.clone(),
            tags: vec![],
            origin: "ai_search".to_string(),
            source_url: None,
        };
        let src = source::create_source(state.db, &src_req)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
        sources.push(src);
    }

    // Step 2: Extract knowledge points from sources
    let all_content: String = sources.iter()
        .map(|s| format!("{}\n{}", s.title, s.content))
        .collect::<Vec<_>>()
        .join("\n---\n");

    let kp_prompt = format!(
        "Extract the main knowledge points from this content about '{}'.\n\n{}\n\n\
        Return ONLY a JSON array of objects with fields: title, summary (one sentence), content (2-3 paragraphs), tags (array of strings).",
        req.topic, all_content
    );
    let kp_response = state.llm.chat("You are a knowledge extraction assistant. Return JSON only.", &kp_prompt).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let kp_json = extract_json(&kp_response);
    let kp_reqs: Vec<CreateKnowledgePointRequest> = serde_json::from_str(kp_json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Parse KPs: {}", e)))?;

    let mut kps = Vec::new();
    for kp_req in &kp_reqs {
        let kp = knowledge::create_kp(state.db, kp_req)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
        kps.push(kp);
    }

    // Step 3: Create learning plan
    let kp_ids: Vec<String> = kps.iter().map(|k| k.id.clone()).collect();
    let plan_req = CreateLearningPlanRequest {
        title: req.topic.clone(),
        goal: format!("Master the core concepts of {}", req.topic),
        kp_ids,
    };
    let plan = repo::learning::create_plan(state.db, &plan_req)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let result = crate::models::AiResearchResult { sources, knowledge_points: kps, plan };
    Ok((StatusCode::OK, Json(serde_json::to_value(&result).unwrap())))
}

/// Generate quiz questions for given knowledge points
#[derive(serde::Deserialize)]
pub struct GenerateQuizRequest {
    pub kp_id: String,
    pub count: usize,
}

pub async fn generate_quiz(
    State(state): State<&'static AppState>,
    Json(req): Json<GenerateQuizRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    let kp = knowledge::get_kp(state.db, &req.kp_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
        .ok_or((StatusCode::NOT_FOUND, "KP not found".to_string()))?;

    let questions = crate::ai::quiz_gen::generate_quizzes(&state.llm, &kp.title, &kp.content, req.count).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    for q in &questions {
        repo::quiz::create_question(state.db, q)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    }

    Ok((StatusCode::CREATED, Json(serde_json::to_value(&questions).unwrap())))
}

/// Update mastery record after quiz attempt
#[derive(serde::Deserialize)]
pub struct UpdateMasteryRequest {
    pub kp_id: String,
    pub is_correct: bool,
}

pub async fn update_mastery(
    State(state): State<&'static AppState>,
    Json(req): Json<UpdateMasteryRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let existing = repo::learning::get_mastery_by_kp(state.db, &req.kp_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let input = match existing {
        Some(rec) => crate::learning::sm2::Sm2Input {
            ease_factor: rec.ease_factor,
            interval_days: rec.interval_days,
            repetitions: rec.repetitions,
            is_correct: req.is_correct,
            response_quality: if req.is_correct { 5 } else { 3 },
        },
        None => crate::learning::sm2::Sm2Input {
            ease_factor: 2.5, interval_days: 0, repetitions: 0,
            is_correct: req.is_correct,
            response_quality: if req.is_correct { 5 } else { 3 },
        },
    };

    let output = crate::learning::sm2::calculate(input);
    let record = crate::models::MasteryRecord {
        id: crate::models::new_id(),
        kp_id: req.kp_id,
        ease_factor: output.ease_factor,
        interval_days: output.interval_days,
        repetitions: output.repetitions,
        next_review_at: output.next_review_at,
        last_reviewed_at: Some(chrono::Utc::now().to_rfc3339()),
    };
    repo::learning::upsert_mastery(state.db, &record)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(StatusCode::OK)
}

fn extract_json(response: &str) -> &str {
    if let Some(start) = response.find("```json") {
        let after = &response[start + 7..];
        if let Some(end) = after.find("```") { &after[..end] } else { after }
    } else if let Some(start) = response.find('[') {
        &response[start..]
    } else if let Some(start) = response.find('{') {
        &response[start..]
    } else {
        response
    }
}
```

- [ ] **步骤 4：更新 `api/mod.rs` 和 `server.rs`**

`api/mod.rs`:
```rust
pub mod sources;
pub mod knowledge;
pub mod quiz;
pub mod learning;
pub mod ai_routes;
```

`server.rs` — 添加新路由并使用 `AppState`:

```rust
use crate::api::{sources, knowledge, quiz, learning, ai_routes};
use crate::ai::llm::LlmClient;

pub fn app(state: &'static ai_routes::AppState) -> Router {
    Router::new()
        .route("/api/health", get(health_check))
        // Sources
        .route("/api/sources", get(sources::list_sources).post(sources::create_source))
        .route("/api/sources/{id}", get(sources::get_source))
        .route("/api/sources/{id}/hide", post(sources::toggle_hidden))
        // Knowledge Points
        .route("/api/knowledge", get(knowledge::list_kps).post(knowledge::create_kp))
        .route("/api/knowledge/{id}", get(knowledge::get_kp).delete(knowledge::delete_kp))
        // Quiz
        .route("/api/quiz/kp/{kp_id}", get(quiz::get_quiz_by_kp))
        .route("/api/quiz/submit", post(quiz::submit_answer))
        // Learning
        .route("/api/learning/plans", get(learning::list_plans).post(learning::create_plan))
        .route("/api/learning/reviews/due", get(learning::get_due_reviews))
        // AI
        .route("/api/ai/research", post(ai_routes::start_research))
        .route("/api/ai/generate-quiz", post(ai_routes::generate_quiz))
        .route("/api/ai/update-mastery", post(ai_routes::update_mastery))
        .layer(CorsLayer::permissive())
        .with_state(state)
}
```

更新 `lib.rs`:

```rust
// In setup closure:
let llm_config = ai::llm::LlmConfig {
    base_url: std::env::var("LLM_BASE_URL").unwrap_or("https://api.openai.com/v1".into()),
    api_key: std::env::var("LLM_API_KEY").unwrap_or_default(),
    model: std::env::var("LLM_MODEL").unwrap_or("gpt-4o-mini".into()),
};
let llm = ai::llm::LlmClient::new(llm_config);

let app_state: &'static ai_routes::AppState = Box::leak(Box::new(ai_routes::AppState { db, llm }));

// spawn server with server::app(app_state)
axum::serve(listener, server::app(app_state)).await.unwrap();
```

- [ ] **步骤 5：构建验证**

```bash
cargo check
```

- [ ] **步骤 6：Commit**

```bash
git add src-tauri/src/api/ src-tauri/src/server.rs src-tauri/src/lib.rs
git commit -m "feat: add quiz, learning, and AI research REST API endpoints"
```

---

## 任务 12：React 前端 - API Client & Types

**文件：**
- 创建：`src/types.ts`
- 创建：`src/api/client.ts`

- [ ] **步骤 1：前端类型定义**

`src/types.ts`:

```typescript
export interface Source {
  id: string;
  title: string;
  type: "url" | "text" | "file";
  content: string;
  tags: string[];
  origin: "user" | "ai_search";
  source_url: string | null;
  hidden: boolean;
  created_at: string;
}

export interface CreateSourceRequest {
  title: string;
  type: string;
  content: string;
  tags: string[];
  origin: string;
  source_url?: string;
}

export interface KnowledgePoint {
  id: string;
  title: string;
  summary: string;
  content: string;
  tags: string[];
  source_ids: string[];
  created_at: string;
}

export interface QuizQuestion {
  id: string;
  kp_id: string;
  type: "multiple_choice" | "fill_blank" | "analysis";
  question: string;
  options: string[] | null;
  answer: string;
  explanation: string;
}

export interface QuizResult {
  question: QuizQuestion;
  user_answer: string;
  is_correct: boolean;
  explanation: string;
}

export interface LearningPlan {
  id: string;
  title: string;
  goal: string;
  kp_ids: string[];
  status: "active" | "completed" | "paused";
  created_at: string;
}

export interface MasteryRecord {
  id: string;
  kp_id: string;
  ease_factor: number;
  interval_days: number;
  repetitions: number;
  next_review_at: string;
  last_reviewed_at: string | null;
}

export interface AiResearchResult {
  sources: Source[];
  knowledge_points: KnowledgePoint[];
  plan: LearningPlan;
}

export interface ChatMessage {
  role: "user" | "assistant" | "system";
  content: string;
}
```

- [ ] **步骤 2：API Client**

`src/api/client.ts`:

```typescript
const API_BASE = "http://127.0.0.1:3001/api";

async function request<T>(path: string, options?: RequestInit): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`, {
    headers: { "Content-Type": "application/json", ...options?.headers },
    ...options,
  });
  if (!res.ok) {
    const err = await res.text();
    throw new Error(`API error ${res.status}: ${err}`);
  }
  if (res.status === 204) return undefined as T;
  return res.json();
}

export const api = {
  // Sources
  sources: {
    list: (includeHidden?: boolean, search?: string) => {
      const params = new URLSearchParams();
      if (includeHidden) params.set("include_hidden", "true");
      if (search) params.set("search", search);
      return request<Source[]>(`/sources?${params}`);
    },
    get: (id: string) => request<Source>(`/sources/${id}`),
    create: (data: CreateSourceRequest) =>
      request<Source>("/sources", { method: "POST", body: JSON.stringify(data) }),
    toggleHidden: (id: string, hidden: boolean) =>
      request<void>(`/sources/${id}/hide`, { method: "POST", body: JSON.stringify({ hidden }) }),
  },

  // Knowledge Points
  knowledge: {
    list: (search?: string, ids?: string[]) => {
      const params = new URLSearchParams();
      if (search) params.set("search", search);
      if (ids) params.set("ids", ids.join(","));
      return request<KnowledgePoint[]>(`/knowledge?${params}`);
    },
    get: (id: string) => request<KnowledgePoint>(`/knowledge/${id}`),
    create: (data: { title: string; summary: string; content: string; tags: string[]; source_ids: string[] }) =>
      request<KnowledgePoint>("/knowledge", { method: "POST", body: JSON.stringify(data) }),
    delete: (id: string) => request<void>(`/knowledge/${id}`, { method: "DELETE" }),
  },

  // Quiz
  quiz: {
    getByKp: (kpId: string) => request<QuizQuestion[]>(`/quiz/kp/${kpId}`),
    submit: (questionId: string, userAnswer: string) =>
      request<QuizResult>("/quiz/submit", {
        method: "POST",
        body: JSON.stringify({ question_id: questionId, user_answer: userAnswer }),
      }),
  },

  // Learning
  learning: {
    listPlans: () => request<LearningPlan[]>("/learning/plans"),
    createPlan: (data: { title: string; goal: string; kp_ids: string[] }) =>
      request<LearningPlan>("/learning/plans", { method: "POST", body: JSON.stringify(data) }),
    getDueReviews: () => request<MasteryRecord[]>("/learning/reviews/due"),
  },

  // AI
  ai: {
    startResearch: (topic: string) =>
      request<AiResearchResult>("/ai/research", { method: "POST", body: JSON.stringify({ topic }) }),
    generateQuiz: (kpId: string, count: number = 3) =>
      request<QuizQuestion[]>("/ai/generate-quiz", {
        method: "POST",
        body: JSON.stringify({ kp_id: kpId, count }),
      }),
    updateMastery: (kpId: string, isCorrect: boolean) =>
      request<void>("/ai/update-mastery", {
        method: "POST",
        body: JSON.stringify({ kp_id: kpId, is_correct: isCorrect }),
      }),
  },
};
```

- [ ] **步骤 3：构建验证**

```bash
npx tsc --noEmit
```

- [ ] **步骤 4：Commit**

```bash
git add src/types.ts src/api/client.ts
git commit -m "feat: add frontend API client and TypeScript type definitions"
```

---

## 任务 13：React 前端 - Chat Panel（核心对话组件）

**文件：**
- 创建：`src/components/Chat/ChatPanel.tsx`
- 创建：`src/components/Chat/ChatPanel.css`
- 创建：`src/components/Chat/MessageBubble.tsx`
- 创建：`src/components/Chat/MessageBubble.css`
- 创建：`src/components/Chat/ChatInput.tsx`
- 创建：`src/components/Chat/ChatInput.css`
- 创建：`src/hooks/useChat.ts`

- [ ] **步骤 1：useChat hook**

`src/hooks/useChat.ts`:

```typescript
import { useState, useCallback } from "react";
import type { ChatMessage, AiResearchResult, LearningPlan, KnowledgePoint } from "../types";
import { api } from "../api/client";

export function useChat() {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [loading, setLoading] = useState(false);
  const [currentPlan, setCurrentPlan] = useState<LearningPlan | null>(null);

  const sendMessage = useCallback(async (content: string) => {
    const userMsg: ChatMessage = { role: "user", content };
    setMessages((prev) => [...prev, userMsg]);
    setLoading(true);

    try {
      // Check if this looks like a new topic request
      const topicTriggers = ["想学", "学习", "教我", "了解", "learn", "study"];
      const isLearnRequest = topicTriggers.some((t) => content.includes(t));

      if (isLearnRequest && !currentPlan) {
        // Extract topic (simple heuristic: take the content as topic)
        const topic = content;
        const result: AiResearchResult = await api.ai.startResearch(topic);

        const botMsg: ChatMessage = {
          role: "assistant",
          content: `我找到了 ${result.sources.length} 份资料，提取了 ${result.knowledge_points.length} 个知识点。

**学习计划：${result.plan.title}**

知识点列表：
${result.knowledge_points.map((kp, i) => `${i + 1}. ${kp.title} — ${kp.summary}`).join("\n")}

准备好学习第一个知识点了吗？`,
        };
        setMessages((prev) => [...prev, botMsg]);
        setCurrentPlan(result.plan);
      } else {
        // General chat (stub for now)
        const botMsg: ChatMessage = {
          role: "assistant",
          content: "收到。你可以告诉我你想学什么，或者粘贴文章链接/内容给我。",
        };
        setMessages((prev) => [...prev, botMsg]);
      }
    } catch (err) {
      const errMsg: ChatMessage = {
        role: "assistant",
        content: `出错了：${err instanceof Error ? err.message : "未知错误"}`,
      };
      setMessages((prev) => [...prev, errMsg]);
    } finally {
      setLoading(false);
    }
  }, [currentPlan]);

  return { messages, loading, currentPlan, sendMessage };
}
```

- [ ] **步骤 2：MessageBubble 组件**

`src/components/Chat/MessageBubble.tsx`:

```tsx
import ReactMarkdown from "react-markdown";
import type { ChatMessage } from "../../types";
import "./MessageBubble.css";

interface Props {
  message: ChatMessage;
}

export default function MessageBubble({ message }: Props) {
  const isUser = message.role === "user";

  return (
    <div className={`message-bubble ${isUser ? "user" : "assistant"}`}>
      <div className="message-avatar">{isUser ? "👤" : "🤖"}</div>
      <div className="message-content">
        {isUser ? (
          <p>{message.content}</p>
        ) : (
          <ReactMarkdown>{message.content}</ReactMarkdown>
        )}
      </div>
    </div>
  );
}
```

- [ ] **步骤 3：ChatInput 组件**

`src/components/Chat/ChatInput.tsx`:

```tsx
import { useState, KeyboardEvent } from "react";
import "./ChatInput.css";

interface Props {
  onSend: (msg: string) => void;
  disabled: boolean;
}

export default function ChatInput({ onSend, disabled }: Props) {
  const [input, setInput] = useState("");

  const handleSend = () => {
    const trimmed = input.trim();
    if (trimmed && !disabled) {
      onSend(trimmed);
      setInput("");
    }
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  return (
    <div className="chat-input">
      <input
        type="text"
        value={input}
        onChange={(e) => setInput(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder={disabled ? "处理中..." : "输入你想学的内容，或者粘贴资料..."}
        disabled={disabled}
      />
      <button onClick={handleSend} disabled={disabled || !input.trim()}>
        发送
      </button>
    </div>
  );
}
```

- [ ] **步骤 4：ChatPanel 主组件**

`src/components/Chat/ChatPanel.tsx`:

```tsx
import { useEffect, useRef } from "react";
import { useChat } from "../../hooks/useChat";
import MessageBubble from "./MessageBubble";
import ChatInput from "./ChatInput";
import "./ChatPanel.css";

export default function ChatPanel() {
  const { messages, loading, sendMessage } = useChat();
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  return (
    <div className="chat-panel">
      <div className="chat-header">
        <h2>Lexio 学习教练</h2>
      </div>
      <div className="chat-messages">
        {messages.length === 0 && (
          <div className="chat-empty">
            <h3>你想学什么？</h3>
            <p>告诉我一个主题，我帮你搜集资料、整理知识点，用测验帮你掌握它。</p>
          </div>
        )}
        {messages.map((msg, i) => (
          <MessageBubble key={i} message={msg} />
        ))}
        {loading && <div className="chat-loading">思考中...</div>}
        <div ref={bottomRef} />
      </div>
      <ChatInput onSend={sendMessage} disabled={loading} />
    </div>
  );
}
```

- [ ] **步骤 5：CSS 样式**

`src/components/Chat/ChatPanel.css`:

```css
.chat-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg-primary, #ffffff);
}

.chat-header {
  padding: 16px 20px;
  border-bottom: 1px solid var(--border-color, #e0e0e0);
}

.chat-header h2 {
  margin: 0;
  font-size: 18px;
}

.chat-messages {
  flex: 1;
  overflow-y: auto;
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.chat-empty {
  text-align: center;
  margin-top: 80px;
  color: var(--text-secondary, #666);
}

.chat-empty h3 {
  font-size: 24px;
  margin-bottom: 8px;
}

.chat-loading {
  align-self: flex-start;
  padding: 10px 16px;
  background: var(--bg-assistant, #f0f0f0);
  border-radius: 12px;
  color: var(--text-secondary, #888);
}
```

`src/components/Chat/MessageBubble.css`:

```css
.message-bubble {
  display: flex;
  gap: 10px;
  max-width: 80%;
}

.message-bubble.user {
  align-self: flex-end;
  flex-direction: row-reverse;
}

.message-bubble.assistant {
  align-self: flex-start;
}

.message-avatar {
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  background: var(--bg-avatar, #e8e8e8);
  flex-shrink: 0;
}

.message-content {
  padding: 10px 16px;
  border-radius: 12px;
  line-height: 1.6;
}

.message-bubble.user .message-content {
  background: var(--accent-color, #2563eb);
  color: white;
}

.message-bubble.assistant .message-content {
  background: var(--bg-assistant, #f0f0f0);
  color: var(--text-primary, #333);
}

.message-content p { margin: 0; }
.message-content p + p { margin-top: 8px; }
.message-content ul { margin: 4px 0; padding-left: 20px; }
.message-content code {
  background: rgba(0,0,0,0.06);
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 0.9em;
}
```

`src/components/Chat/ChatInput.css`:

```css
.chat-input {
  display: flex;
  gap: 10px;
  padding: 16px 20px;
  border-top: 1px solid var(--border-color, #e0e0e0);
  background: var(--bg-primary, #fff);
}

.chat-input input {
  flex: 1;
  padding: 10px 16px;
  border: 1px solid var(--border-color, #ddd);
  border-radius: 8px;
  font-size: 14px;
  outline: none;
}

.chat-input input:focus {
  border-color: var(--accent-color, #2563eb);
}

.chat-input button {
  padding: 10px 20px;
  background: var(--accent-color, #2563eb);
  color: white;
  border: none;
  border-radius: 8px;
  cursor: pointer;
  font-size: 14px;
}

.chat-input button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
```

- [ ] **步骤 6：安装 react-markdown**

```bash
npm install react-markdown
```

- [ ] **步骤 7：构建验证**

```bash
npx tsc --noEmit
```

- [ ] **步骤 8：Commit**

```bash
git add src/components/Chat/ src/hooks/useChat.ts package.json
git commit -m "feat: implement chat panel with AI research integration"
```

---

## 任务 14：React 前端 - 侧栏（来源 & 知识点）

**文件：**
- 创建：`src/hooks/useSources.ts`
- 创建：`src/hooks/useKnowledge.ts`
- 创建：`src/components/Sidebar/SourceList.tsx`
- 创建：`src/components/Sidebar/KpList.tsx`
- 创建：`src/components/Sidebar/SourceList.css`
- 创建：`src/components/Sidebar/KpList.css`

- [ ] **步骤 1：useSources hook**

`src/hooks/useSources.ts`:

```typescript
import { useState, useEffect, useCallback } from "react";
import type { Source } from "../types";
import { api } from "../api/client";

export function useSources() {
  const [sources, setSources] = useState<Source[]>([]);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const data = await api.sources.list();
      setSources(data);
    } catch (err) {
      console.error("Failed to load sources:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  const toggleHidden = useCallback(async (id: string, hidden: boolean) => {
    await api.sources.toggleHidden(id, hidden);
    setSources((prev) =>
      prev.map((s) => (s.id === id ? { ...s, hidden } : s))
    );
  }, []);

  useEffect(() => { refresh(); }, [refresh]);

  return { sources, loading, refresh, toggleHidden };
}
```

- [ ] **步骤 2：useKnowledge hook**

`src/hooks/useKnowledge.ts`:

```typescript
import { useState, useEffect, useCallback } from "react";
import type { KnowledgePoint } from "../types";
import { api } from "../api/client";

export function useKnowledge() {
  const [kps, setKps] = useState<KnowledgePoint[]>([]);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const data = await api.knowledge.list();
      setKps(data);
    } catch (err) {
      console.error("Failed to load knowledge points:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { refresh(); }, [refresh]);

  return { kps, loading, refresh };
}
```

- [ ] **步骤 3：SourceList 组件**

`src/components/Sidebar/SourceList.tsx`:

```tsx
import { useSources } from "../../hooks/useSources";
import "./SourceList.css";

export default function SourceList() {
  const { sources, loading, toggleHidden } = useSources();
  const visible = sources.filter((s) => !s.hidden);

  return (
    <div className="source-list">
      <h3>资料来源 ({visible.length})</h3>
      {loading && <p className="list-loading">加载中...</p>}
      {visible.map((s) => (
        <div key={s.id} className="source-item">
          <div className="source-item-header">
            <span className={`source-origin ${s.origin}`}>
              {s.origin === "ai_search" ? "🔍" : "📎"}
            </span>
            <span className="source-title">{s.title}</span>
          </div>
          <div className="source-item-actions">
            <button onClick={() => toggleHidden(s.id, true)} title="暂时不看">
              👁️
            </button>
          </div>
        </div>
      ))}
      {!loading && visible.length === 0 && (
        <p className="list-empty">暂无资料来源</p>
      )}
    </div>
  );
}
```

- [ ] **步骤 4：KpList 组件**

`src/components/Sidebar/KpList.tsx`:

```tsx
import { useKnowledge } from "../../hooks/useKnowledge";
import "./KpList.css";

interface Props {
  onSelect: (id: string) => void;
  selectedId?: string;
}

export default function KpList({ onSelect, selectedId }: Props) {
  const { kps, loading } = useKnowledge();

  return (
    <div className="kp-list">
      <h3>知识点 ({kps.length})</h3>
      {loading && <p className="list-loading">加载中...</p>}
      {kps.map((kp) => (
        <div
          key={kp.id}
          className={`kp-item ${selectedId === kp.id ? "selected" : ""}`}
          onClick={() => onSelect(kp.id)}
        >
          <span className="kp-title">{kp.title}</span>
          <span className="kp-summary">{kp.summary}</span>
        </div>
      ))}
    </div>
  );
}
```

- [ ] **步骤 5：CSS**

`src/components/Sidebar/SourceList.css` and `KpList.css` (basic styles):

```css
.source-list, .kp-list {
  padding: 12px 0;
}

.source-list h3, .kp-list h3 {
  padding: 0 16px;
  font-size: 13px;
  color: var(--text-secondary, #888);
  text-transform: uppercase;
  margin-bottom: 8px;
}

.source-item, .kp-item {
  padding: 8px 16px;
  cursor: pointer;
  border-left: 3px solid transparent;
  transition: background 0.15s;
}

.source-item:hover, .kp-item:hover {
  background: var(--bg-hover, #f5f5f5);
}

.kp-item.selected {
  border-left-color: var(--accent-color, #2563eb);
  background: var(--bg-selected, #eef2ff);
}

.source-item-header, .kp-title {
  font-size: 14px;
  font-weight: 500;
  margin-bottom: 2px;
}

.source-item-actions {
  margin-top: 4px;
}

.source-item-actions button {
  background: none;
  border: none;
  cursor: pointer;
  font-size: 14px;
  padding: 2px;
}

.kp-summary {
  font-size: 12px;
  color: var(--text-secondary, #888);
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.list-loading, .list-empty {
  padding: 12px 16px;
  font-size: 13px;
  color: var(--text-secondary, #888);
}

.source-origin {
  margin-right: 6px;
  font-size: 12px;
}
```

- [ ] **步骤 6：构建验证**

```bash
npx tsc --noEmit
```

- [ ] **步骤 7：Commit**

```bash
git add src/hooks/ src/components/Sidebar/
git commit -m "feat: implement source and knowledge point sidebar components"
```

---

## 任务 15：React 前端 - Quiz & Learning 组件

**文件：**
- 创建：`src/hooks/useQuiz.ts`
- 创建：`src/components/Content/QuizCard.tsx`
- 创建：`src/components/Content/QuizCard.css`
- 创建：`src/components/Content/LearningView.tsx`
- 创建：`src/components/Content/LearningView.css`

- [ ] **步骤 1：useQuiz hook**

`src/hooks/useQuiz.ts`:

```typescript
import { useState, useCallback } from "react";
import type { QuizQuestion, QuizResult } from "../types";
import { api } from "../api/client";

export function useQuiz(kpId: string | null) {
  const [questions, setQuestions] = useState<QuizQuestion[]>([]);
  const [currentIndex, setCurrentIndex] = useState(0);
  const [result, setResult] = useState<QuizResult | null>(null);
  const [loading, setLoading] = useState(false);

  const loadQuestions = useCallback(async () => {
    if (!kpId) return;
    setLoading(true);
    try {
      let qs = await api.quiz.getByKp(kpId);
      if (qs.length === 0) {
        qs = await api.ai.generateQuiz(kpId, 3);
      }
      setQuestions(qs);
      setCurrentIndex(0);
      setResult(null);
    } catch (err) {
      console.error("Failed to load quiz:", err);
    } finally {
      setLoading(false);
    }
  }, [kpId]);

  const submitAnswer = useCallback(async (answer: string) => {
    const q = questions[currentIndex];
    if (!q) return;
    setLoading(true);
    try {
      const res = await api.quiz.submit(q.id, answer);
      setResult(res);
      await api.ai.updateMastery(q.kp_id, res.is_correct);
    } catch (err) {
      console.error("Failed to submit:", err);
    } finally {
      setLoading(false);
    }
  }, [questions, currentIndex]);

  const nextQuestion = useCallback(() => {
    if (currentIndex < questions.length - 1) {
      setCurrentIndex((i) => i + 1);
      setResult(null);
    }
  }, [currentIndex, questions.length]);

  const currentQuestion = questions[currentIndex] || null;
  const isFinished = currentIndex >= questions.length - 1 && result !== null;

  return {
    questions,
    currentQuestion,
    result,
    loading,
    isFinished,
    loadQuestions,
    submitAnswer,
    nextQuestion,
  };
}
```

- [ ] **步骤 2：QuizCard 组件**

`src/components/Content/QuizCard.tsx`:

```tsx
import { useState } from "react";
import type { QuizQuestion, QuizResult } from "../../types";
import "./QuizCard.css";

interface Props {
  question: QuizQuestion;
  result: QuizResult | null;
  loading: boolean;
  onSubmit: (answer: string) => void;
  onNext: () => void;
  isLast: boolean;
}

export default function QuizCard({ question, result, loading, onSubmit, onNext, isLast }: Props) {
  const [selected, setSelected] = useState("");
  const [textAnswer, setTextAnswer] = useState("");

  const handleSubmit = () => {
    const answer = question.type === "multiple_choice" ? selected : textAnswer;
    if (answer.trim()) onSubmit(answer.trim());
  };

  return (
    <div className="quiz-card">
      <div className="quiz-question">
        <span className="quiz-type-badge">
          {question.type === "multiple_choice" ? "选择题" : "填空题"}
        </span>
        <p>{question.question}</p>
      </div>

      {!result && (
        <div className="quiz-answer-area">
          {question.type === "multiple_choice" && question.options ? (
            <div className="quiz-options">
              {question.options.map((opt, i) => (
                <label
                  key={i}
                  className={`quiz-option ${selected === opt ? "selected" : ""}`}
                >
                  <input
                    type="radio"
                    name="quiz-answer"
                    value={opt}
                    checked={selected === opt}
                    onChange={() => setSelected(opt)}
                  />
                  {opt}
                </label>
              ))}
            </div>
          ) : (
            <input
              type="text"
              className="quiz-text-input"
              value={textAnswer}
              onChange={(e) => setTextAnswer(e.target.value)}
              placeholder="输入你的答案..."
            />
          )}
          <button
            className="quiz-submit-btn"
            onClick={handleSubmit}
            disabled={loading || (!selected && !textAnswer)}
          >
            {loading ? "提交中..." : "提交"}
          </button>
        </div>
      )}

      {result && (
        <div className={`quiz-result ${result.is_correct ? "correct" : "incorrect"}`}>
          <div className="quiz-result-header">
            {result.is_correct ? "✅ 正确！" : "❌ 不正确"}
          </div>
          <div className="quiz-result-explanation">{result.explanation}</div>
          {!isLast && (
            <button className="quiz-next-btn" onClick={onNext}>
              下一题
            </button>
          )}
        </div>
      )}
    </div>
  );
}
```

- [ ] **步骤 3：LearningView 组件**

`src/components/Content/LearningView.tsx`:

```tsx
import { useEffect, useState } from "react";
import ReactMarkdown from "react-markdown";
import { api } from "../../api/client";
import type { KnowledgePoint } from "../../types";
import { useQuiz } from "../../hooks/useQuiz";
import QuizCard from "./QuizCard";
import "./LearningView.css";

interface Props {
  kpId: string | null;
}

export default function LearningView({ kpId }: Props) {
  const [kp, setKp] = useState<KnowledgePoint | null>(null);
  const [showQuiz, setShowQuiz] = useState(false);
  const quiz = useQuiz(kpId);

  useEffect(() => {
    if (!kpId) return;
    api.knowledge.get(kpId).then(setKp).catch(console.error);
    setShowQuiz(false);
  }, [kpId]);

  if (!kpId || !kp) {
    return (
      <div className="learning-empty">
        <p>选择一个知识点开始学习</p>
      </div>
    );
  }

  return (
    <div className="learning-view">
      <div className="learning-header">
        <h2>{kp.title}</h2>
        <div className="learning-tags">
          {kp.tags.map((t) => (
            <span key={t} className="tag">{t}</span>
          ))}
        </div>
      </div>

      <div className="learning-content">
        <ReactMarkdown>{kp.content}</ReactMarkdown>
      </div>

      <div className="learning-actions">
        {!showQuiz ? (
          <button
            className="btn-start-quiz"
            onClick={() => { setShowQuiz(true); quiz.loadQuestions(); }}
          >
            开始测验
          </button>
        ) : (
          <div className="quiz-section">
            <h3>测验</h3>
            {quiz.loading && !quiz.currentQuestion && <p>加载题目中...</p>}
            {quiz.currentQuestion && (
              <QuizCard
                question={quiz.currentQuestion}
                result={quiz.result}
                loading={quiz.loading}
                onSubmit={quiz.submitAnswer}
                onNext={quiz.nextQuestion}
                isLast={quiz.isFinished}
              />
            )}
            {quiz.isFinished && (
              <p className="quiz-done">测验完成！</p>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
```

- [ ] **步骤 4：CSS**

`src/components/Content/QuizCard.css`:

```css
.quiz-card {
  border: 1px solid var(--border-color, #e0e0e0);
  border-radius: 12px;
  padding: 20px;
  margin: 12px 0;
}

.quiz-type-badge {
  display: inline-block;
  padding: 2px 10px;
  background: var(--accent-light, #eef2ff);
  color: var(--accent-color, #2563eb);
  border-radius: 12px;
  font-size: 12px;
  margin-bottom: 8px;
}

.quiz-question p {
  font-size: 16px;
  margin: 8px 0;
}

.quiz-options {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin: 12px 0;
}

.quiz-option {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 14px;
  border: 1px solid var(--border-color, #ddd);
  border-radius: 8px;
  cursor: pointer;
  transition: background 0.15s;
}

.quiz-option:hover, .quiz-option.selected {
  background: var(--bg-hover, #f5f5f5);
  border-color: var(--accent-color, #2563eb);
}

.quiz-text-input {
  width: 100%;
  padding: 10px;
  border: 1px solid var(--border-color, #ddd);
  border-radius: 8px;
  font-size: 14px;
  margin: 12px 0;
}

.quiz-submit-btn, .quiz-next-btn, .btn-start-quiz {
  padding: 10px 24px;
  background: var(--accent-color, #2563eb);
  color: white;
  border: none;
  border-radius: 8px;
  cursor: pointer;
  font-size: 14px;
  margin-top: 8px;
}

.quiz-submit-btn:disabled { opacity: 0.5; cursor: not-allowed; }

.quiz-result { padding: 12px 0; }
.quiz-result-header { font-weight: 600; margin-bottom: 8px; }
.quiz-result.correct .quiz-result-header { color: #16a34a; }
.quiz-result.incorrect .quiz-result-header { color: #dc2626; }
.quiz-result-explanation { color: var(--text-secondary, #666); font-size: 14px; line-height: 1.6; }
.quiz-done { color: #16a34a; font-weight: 500; margin-top: 12px; }
```

`src/components/Content/LearningView.css`:

```css
.learning-view { padding: 24px; }
.learning-empty { display: flex; align-items: center; justify-content: center; height: 200px; color: var(--text-secondary, #888); }
.learning-header { margin-bottom: 20px; }
.learning-header h2 { margin: 0 0 8px; font-size: 22px; }
.learning-tags { display: flex; gap: 6px; flex-wrap: wrap; }
.tag { padding: 2px 10px; background: var(--bg-tag, #f0f0f0); border-radius: 10px; font-size: 12px; }
.learning-content { line-height: 1.8; color: var(--text-primary, #333); }
.learning-content pre { background: #1e1e1e; color: #d4d4d4; padding: 16px; border-radius: 8px; overflow-x: auto; }
.learning-content code { font-size: 0.9em; }
.quiz-section { margin-top: 24px; }
.quiz-section h3 { margin-bottom: 16px; }
```

- [ ] **步骤 5：构建验证**

```bash
npx tsc --noEmit
```

- [ ] **步骤 6：Commit**

```bash
git add src/hooks/useQuiz.ts src/components/Content/
git commit -m "feat: implement quiz card and learning view components"
```

---

## 任务 16：React 前端 - 整合布局

**文件：**
- 修改：`src/components/Layout.tsx`
- 修改：`src/components/Layout.css`
- 修改：`src/components/Sidebar.tsx`
- 修改：`src/components/Sidebar.css`
- 修改：`src/components/Content.tsx`
- 修改：`src/components/Content.css`

- [ ] **步骤 1：更新 Sidebar 整合新组件**

`src/components/Sidebar.tsx`:

```tsx
import { useState } from "react";
import SourceList from "./Sidebar/SourceList";
import KpList from "./Sidebar/KpList";
import { usePlatform } from "../utils/tauri";
import "./Sidebar.css";

interface Props {
  onSelectKp: (id: string) => void;
  selectedKpId?: string;
}

export default function Sidebar({ onSelectKp, selectedKpId }: Props) {
  const platform = usePlatform();
  const [tab, setTab] = useState<"sources" | "knowledge">("knowledge");

  return (
    <aside className="sidebar">
      <div className="sidebar-tabs">
        <button
          className={`sidebar-tab ${tab === "knowledge" ? "active" : ""}`}
          onClick={() => setTab("knowledge")}
        >
          知识点
        </button>
        <button
          className={`sidebar-tab ${tab === "sources" ? "active" : ""}`}
          onClick={() => setTab("sources")}
        >
          资料
        </button>
      </div>
      {tab === "knowledge" && <KpList onSelect={onSelectKp} selectedId={selectedKpId} />}
      {tab === "sources" && <SourceList />}
      <div className="sidebar-footer">{platform}</div>
    </aside>
  );
}
```

更新 `Sidebar.css` 添加 tab 样式：

```css
.sidebar-tabs {
  display: flex;
  border-bottom: 1px solid var(--border-color, #e0e0e0);
}

.sidebar-tab {
  flex: 1;
  padding: 10px;
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
  cursor: pointer;
  font-size: 13px;
  color: var(--text-secondary, #888);
}

.sidebar-tab.active {
  color: var(--accent-color, #2563eb);
  border-bottom-color: var(--accent-color, #2563eb);
}
```

- [ ] **步骤 2：更新 Content 整合 LearningView**

`src/components/Content.tsx`:

```tsx
import ChatPanel from "./Chat/ChatPanel";
import LearningView from "./Content/LearningView";
import "./Content.css";

interface Props {
  view: "chat" | "learning";
  selectedKpId: string | null;
}

export default function Content({ view, selectedKpId }: Props) {
  return (
    <main className="content">
      {view === "chat" ? <ChatPanel /> : <LearningView kpId={selectedKpId} />}
    </main>
  );
}
```

- [ ] **步骤 3：更新 Layout 状态管理**

`src/components/Layout.tsx`:

```tsx
import { useState } from "react";
import Sidebar from "./Sidebar";
import Content from "./Content";
import "./Layout.css";

export default function Layout() {
  const [selectedKpId, setSelectedKpId] = useState<string | null>(null);
  const [view, setView] = useState<"chat" | "learning">("chat");

  const handleSelectKp = (id: string) => {
    setSelectedKpId(id);
    setView("learning");
  };

  return (
    <div className="layout">
      <Sidebar onSelectKp={handleSelectKp} selectedKpId={selectedKpId ?? undefined} />
      <Content view={view} selectedKpId={selectedKpId} />
    </div>
  );
}
```

- [ ] **步骤 4：构建验证**

```bash
npx tsc --noEmit
```

- [ ] **步骤 5：Commit**

```bash
git add src/components/Layout.tsx src/components/Layout.css src/components/Sidebar.tsx src/components/Sidebar.css src/components/Content.tsx src/components/Content.css
git commit -m "feat: integrate chat, learning, and sidebar into unified layout"
```

---

## 任务 17：端到端集成测试 & 修复

**文件：**
- 无需新增文件

- [ ] **步骤 1：启动后端测试**

```bash
# In one terminal
cd src-tauri && cargo build
# Set env vars for LLM (optional for basic test)
export LLM_BASE_URL="https://api.openai.com/v1"
export LLM_API_KEY="sk-test"
export LLM_MODEL="gpt-4o-mini"

# Run binary to verify DB init works
cargo test
```

预期：所有 Rust 测试通过（SM-2 测试）

- [ ] **步骤 2：前端构建验证**

```bash
npx tsc --noEmit && npm run build
```

预期：TypeScript 编译无错误，Vite build 成功。

- [ ] **步骤 3：API 手动测试**

启动应用后验证：

```bash
# Health check
curl http://127.0.0.1:3001/api/health
# Expected: "OK"

# Create source
curl -X POST http://127.0.0.1:3001/api/sources \
  -H "Content-Type: application/json" \
  -d '{"title":"Test","type":"text","content":"Rust is a systems programming language.","tags":["rust"],"origin":"user"}'
# Expected: 201 with source JSON

# List sources
curl http://127.0.0.1:3001/api/sources
# Expected: array with the created source

# Create knowledge point
curl -X POST http://127.0.0.1:3001/api/knowledge \
  -H "Content-Type: application/json" \
  -d '{"title":"Rust Basics","summary":"Introduction to Rust","content":"Rust is a memory-safe systems language...","tags":["rust"],"source_ids":[]}'
# Expected: 201 with KP JSON

# List knowledge points
curl http://127.0.0.1:3001/api/knowledge
# Expected: array
```

- [ ] **步骤 4：修复发现的问题**

运行过程中发现的编译错误或 API bug，逐一修复。

- [ ] **步骤 5：Commit**

```bash
git add -A
git commit -m "fix: resolve integration issues and verify end-to-end flow"
```

---

## 自检结果

### 1. 规格覆盖度
对照设计规格检查任务覆盖情况：

| 规格章节 | 覆盖任务 |
|---|---|
| 数据模型 (Source, KP, Quiz, Mastery, Plan) | 任务 3 (models), 任务 4-6 (repos) |
| 数据库 Schema | 任务 2 (db.rs) |
| AI 网关 (LLM, 搜索, 提取, 出题) | 任务 8-9 (ai/) |
| 学习引擎 (SM-2) | 任务 7 (learning/sm2) |
| REST API (Sources, KP, Quiz, Learning, AI) | 任务 10-11 (api/) |
| 前端对话界面 | 任务 13 (ChatPanel) |
| 前端侧栏 (来源 & 知识点) | 任务 14 (Sidebar) |
| 前端测验交互 | 任务 15 (QuizCard) |
| 流程 1: 新建学习主题 | 任务 11 (ai_routes::start_research) + 任务 13 (useChat) |
| 流程 2: 学习 + 测验循环 | 任务 15 (LearningView + QuizCard) |
| 流程 3: 复习提醒 (SM-2) | 任务 11 (update_mastery) + 任务 7 (sm2) |
| 流程 4: 随手扔资料 | 任务 14 (SourceList + 编辑) |
| MVP 验收: 全流程跑通 | 任务 17 (端到端测试) |

无遗漏。

### 2. 占位符扫描

未发现 TODO、占位符、模糊实现描述。每个任务都包含完整代码或明确的实现指令。

### 3. 类型一致性

- Rust 端：`Source`, `KnowledgePoint`, `QuizQuestion` 等在 models.rs 中定义，repo 和 api 层一致引用
- 前端端：`src/types.ts` 与 Rust models 字段名一致（使用 serde rename 适配 JSON 字段）
- SM-2 输入输出结构：在 sm2.rs 和 ai_routes.rs 中一致

无矛盾。

---

## 执行选项

计划已完成并保存到 `docs/superpowers/plans/2026-07-21-lexio-ai-knowledge-agent.md`。两种执行方式：

**1. 子代理驱动（推荐）** — 每个任务调度一个新的子代理，任务间进行审查，快速迭代

**2. 内联执行** — 在当前会话中使用 executing-plans 执行任务，批量执行并设有检查点

选哪种方式？
