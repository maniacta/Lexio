# 备份分支功能适配实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 把 `backup/local-main-pre-reset`（5f27180）的 LLM chat 教练（含多会话持久化）、日志/审计系统、以及独立修复（FTS/SM-2/迁移跟踪/级联补全）适配到新架构（origin/main 8653425）。

**架构：** 新架构基准：per-vendor AI providers（`crate::ai::create_provider`/`LlmProvider`，仅 DeepSeek）、`blocking::run` 卸载 SQLite、`require_token` 认证、`db.migrate()` 版本化（v1 现有表 / v2 audit_logs / v3 chat 表）。备份分支是**参考实现**：每个任务给出备份文件路径，执行者用 `git show backup/local-main-pre-reset:<path>` 取完整代码后按任务中的适配要点改造。

**技术栈：** Rust（Axum 0.8 / rusqlite / tracing / tracing-subscriber / tracing-appender）+ React 19 + TypeScript（Vite）+ Tauri v2。

**规格：** `docs/superpowers/specs/2026-08-07-backup-features-adaptation-design.md`

**测试基建：** 现有单测模式：`Database::new(":memory:")` + `db.migrate()`（见 `src-tauri/src/learning/sm2.rs` 的 `#[cfg(test)]` 与备份 `repo/knowledge.rs` 的 `fn test_db()`）。后端测试运行：`cd src-tauri && cargo test`。前端无测试框架，验证走 `npm run build` + 手动冒烟。

**提交规范：** 每个任务独立 commit；后端 `feat`/`fix` 前缀，前端同。

---

### 任务 1：迁移框架（PRAGMA user_version）+ 表结构 v1/v2/v3

**文件：**
- 修改：`src-tauri/src/db.rs`

**说明：** 把远端 `migrate()`（当前是顺序 `CREATE TABLE IF NOT EXISTS`）重构为版本化迁移。备份参考：`git show 968f6d5:src-tauri/src/db.rs`（版本化骨架）。远端现有建表语句全部保留为 v1（sources / knowledge_points / relations / quiz_questions / quiz_attempts / mastery_records / learning_plans / settings / model_providers / provider_models / task_models / kp_fts / sources_fts）。v2 加 `audit_logs`，v3 加 `chat_sessions`/`chat_messages`。

**audit_logs 表（v2，字段对齐备份 `repo/audit.rs`）：**

```sql
CREATE TABLE IF NOT EXISTS audit_logs (
  id TEXT PRIMARY KEY,
  timestamp TEXT NOT NULL,
  source TEXT NOT NULL,              -- 'backend' | 'frontend'
  level TEXT NOT NULL CHECK (level IN ('info','warn','error')),
  category TEXT NOT NULL,
  action TEXT NOT NULL,
  user_action TEXT,
  method TEXT,
  path TEXT,
  status_code INTEGER,
  duration_ms INTEGER,
  params_summary TEXT,
  result_summary TEXT,
  error_message TEXT
);
CREATE INDEX IF NOT EXISTS idx_audit_logs_created ON audit_logs(timestamp);
```

**chat 表（v3）：**

```sql
CREATE TABLE IF NOT EXISTS chat_sessions (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  plan_id TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS chat_messages (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL REFERENCES chat_sessions(id) ON DELETE CASCADE,
  role TEXT NOT NULL CHECK (role IN ('user','assistant')),
  content TEXT NOT NULL,
  actions TEXT,        -- JSON 数组
  context TEXT,        -- JSON 对象
  created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_chat_messages_session ON chat_messages(session_id, created_at);
```

**迁移骨架：**

```rust
pub fn migrate(&self) -> Result<()> {
    let conn = self.conn.lock().map_err(|e| e.to_string())?;
    let version: i32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version < 1 { /* v1：远端全部现有 CREATE TABLE IF NOT EXISTS（原样保留）+ FTS */ }
    if version < 2 { /* v2：audit_logs */ }
    if version < 3 { /* v3：chat_sessions / chat_messages */ }
    conn.pragma_update(None, "user_version", 3)?;
    Ok(())
}
```

> 注意：每个版本内语句用 `CREATE TABLE IF NOT EXISTS` 保持幂等；旧库（user_version=0）自动顺序升级。`db.rs` 中现有的 FTS 虚拟表创建（`CREATE VIRTUAL TABLE ... USING fts5(...)`）必须保留在 v1。

- [ ] **步骤 1：编写失败的迁移测试**（追加到 db.rs `#[cfg(test)]`）

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Database {
        let db = Database::new(":memory:").expect("in-memory db");
        db.migrate().expect("migrate");
        db
    }

    #[test]
    fn migrate_sets_user_version_3() {
        let db = test_db();
        let conn = db.conn.lock().unwrap();
        let v: i32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(v, 3);
    }

    #[test]
    fn migrate_creates_new_tables() {
        let db = test_db();
        let conn = db.conn.lock().unwrap();
        for tbl in ["audit_logs", "chat_sessions", "chat_messages"] {
            let n: i32 = conn
                .query_row("SELECT count(*) FROM sqlite_master WHERE type='table' AND name=?1", [tbl], |r| r.get(0))
                .unwrap();
            assert_eq!(n, 1, "table {tbl} should exist");
        }
    }

    #[test]
    fn migrate_is_idempotent() {
        let db = test_db();
        db.migrate().expect("second migrate");
        let conn = db.conn.lock().unwrap();
        let v: i32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(v, 3);
    }
}
```

- [ ] **步骤 2：运行测试确认失败**

运行：`cd src-tauri && cargo test migrate_`
预期：`migrate_sets_user_version_3` 失败（user_version 为 0），表不存在。

- [ ] **步骤 3：重构 migrate() 实现版本化 + 新表**

按上文骨架改造 `db.rs::migrate()`；保留远端全部 v1 建表语句与 FTS 虚拟表；追加 v2/v3。参考备份 `git show 968f6d5:src-tauri/src/db.rs` 与 `git show 5f27180:src-tauri/src/db.rs`（含新表语句）。

- [ ] **步骤 4：运行测试确认通过**

运行：`cd src-tauri && cargo test migrate_`
预期：3 个测试全部 PASS。

- [ ] **步骤 5：Commit**

```bash
git add src-tauri/src/db.rs
git commit -m "feat(db): versioned migrations via PRAGMA user_version (v2 audit_logs, v3 chat)"
```

---

### 任务 2：repo/chat.rs — 多会话 CRUD

**文件：**
- 创建：`src-tauri/src/repo/chat.rs`
- 修改：`src-tauri/src/lib.rs`（加 `pub mod repo;` 已存在，确认 `repo/mod.rs` 声明 `pub mod chat;`）

**说明：** 全新模块（备份分支无此功能，多会话持久化是本设计新增）。参考现有 repo 风格（`repo/learning.rs` 的 Mutex 模式）。类型：`ChatSession { id, title, plan_id: Option<String>, message_count: i64, updated_at }`、`ChatMessage { id, session_id, role, content, actions: Option<String>, context: Option<String>, created_at }`（actions/context 直接存 JSON 字符串，前端解析）。

```rust
use crate::db::Database;
use crate::models::new_id;

pub struct ChatSession {
    pub id: String,
    pub title: String,
    pub plan_id: Option<String>,
    pub message_count: i64,
    pub updated_at: String,
}

pub struct ChatMessage {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub actions: Option<String>,
    pub context: Option<String>,
    pub created_at: String,
}

pub fn list_sessions(db: &Database) -> Result<Vec<ChatSession>, String>;
pub fn create_session(db: &Database, title: &str) -> Result<ChatSession, String>;
pub fn get_messages(db: &Database, session_id: &str) -> Result<Vec<ChatMessage>, String>;
pub fn append_message(db: &Database, session_id: &str, role: &str, content: &str,
    actions: Option<&str>, context: Option<&str>) -> Result<ChatMessage, String>;
pub fn delete_session(db: &Database, session_id: &str) -> Result<(), String>;
pub fn set_session_title(db: &Database, session_id: &str, title: &str) -> Result<(), String>;
pub fn set_session_plan(db: &Database, session_id: &str, plan_id: &str) -> Result<(), String>;
```

- [ ] **步骤 1：编写失败的 repo 测试**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Database {
        let db = Database::new(":memory:").expect("in-memory db");
        db.migrate().expect("migrate");
        db
    }

    #[test]
    fn session_crud_flow() {
        let db = test_db();
        let s = create_session(&db, "新对话").unwrap();
        assert_eq!(s.title, "新对话");
        assert_eq!(s.message_count, 0);

        let m = append_message(&db, &s.id, "user", "你好", None, None).unwrap();
        assert_eq!(m.role, "user");
        let msgs = get_messages(&db, &s.id).unwrap();
        assert_eq!(msgs.len(), 1);

        set_session_title(&db, &s.id, "你好，世界").unwrap();
        set_session_plan(&db, &s.id, "plan-1").unwrap();
        let list = list_sessions(&db).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].title, "你好，世界");
        assert_eq!(list[0].plan_id.as_deref(), Some("plan-1"));
        assert_eq!(list[0].message_count, 1);

        delete_session(&db, &s.id).unwrap();
        assert!(list_sessions(&db).unwrap().is_empty());
        assert!(get_messages(&db, &s.id).unwrap().is_empty());
    }
}
```

- [ ] **步骤 2：运行测试确认失败**

运行：`cd src-tauri && cargo test session_crud_flow`
预期：FAIL（`unresolved import crate::repo::chat` / 函数未定义）。

- [ ] **步骤 3：实现 repo/chat.rs**

按上文签名实现；SQL 参考 `repo/learning.rs` 风格（`conn.prepare` + `query_map`）；`list_sessions` 用 `LEFT JOIN` 统计消息数并按 `updated_at DESC` 排序；`append_message` 同步更新 `chat_sessions.updated_at`；`delete_session` 依赖外键 `ON DELETE CASCADE`（注意：rusqlite 需 `PRAGMA foreign_keys=ON`，或显式先删 messages 再删 session——**选择显式两条 DELETE，避免依赖 pragma**）。

- [ ] **步骤 4：运行测试确认通过**

运行：`cd src-tauri && cargo test session_crud_flow`
预期：PASS。

- [ ] **步骤 5：Commit**

```bash
git add src-tauri/src/repo/chat.rs src-tauri/src/repo/mod.rs
git commit -m "feat(chat): add multi-session repository CRUD"
```

---

### 任务 3：repo/audit.rs + 保留策略

**文件：**
- 创建：`src-tauri/src/repo/audit.rs`
- 修改：`src-tauri/src/repo/mod.rs`（声明 `pub mod audit;`）

**说明：** 备份参考：`git show 5f27180:src-tauri/src/repo/audit.rs`（完整实现）。`AuditRecord` 结构字段与备份一致（id/timestamp/source/level/category/action/user_action/method/path/status_code/duration_ms/params_summary/result_summary/error_message，summary 字段为 `Option<String>`）。函数：`insert(db, &AuditRecord)`、`batch_insert(db, &[AuditRecord])`（事务）、`list(db, limit)`、`prune(db, older_than_days)`。

```rust
pub struct AuditRecord {
    pub id: String,
    pub timestamp: String,
    pub source: String,
    pub level: String,
    pub category: String,
    pub action: String,
    pub user_action: Option<String>,
    pub method: Option<String>,
    pub path: Option<String>,
    pub status_code: Option<i32>,
    pub duration_ms: Option<i64>,
    pub params_summary: Option<String>,
    pub result_summary: Option<String>,
    pub error_message: Option<String>,
}
```

- [ ] **步骤 1：编写失败的测试**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Database {
        let db = Database::new(":memory:").expect("in-memory db");
        db.migrate().expect("migrate");
        db
    }

    fn rec(action: &str, days_ago: i64) -> AuditRecord {
        let ts = (chrono::Utc::now() - chrono::Duration::days(days_ago))
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        AuditRecord {
            id: crate::models::new_id(),
            timestamp: ts,
            source: "backend".into(),
            level: "info".into(),
            category: "test".into(),
            action: action.into(),
            user_action: None, method: None, path: None, status_code: None,
            duration_ms: None, params_summary: None, result_summary: None, error_message: None,
        }
    }

    #[test]
    fn insert_and_list_roundtrip() {
        let db = test_db();
        insert(&db, &rec("a", 0)).unwrap();
        let rows = list(&db, 10).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].action, "a");
    }

    #[test]
    fn batch_insert_transactional() {
        let db = test_db();
        let mut batch = vec![rec("b1", 0), rec("b2", 0)];
        batch_insert(&db, &batch).unwrap();
        assert_eq!(list(&db, 10).unwrap().len(), 2);
    }

    #[test]
    fn prune_removes_old_rows_keeps_recent() {
        let db = test_db();
        insert(&db, &rec("old", 31)).unwrap();
        insert(&db, &rec("new", 1)).unwrap();
        prune(&db, 30).unwrap();
        let rows = list(&db, 10).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].action, "new");
    }
}
```

- [ ] **步骤 2：运行测试确认失败**

运行：`cd src-tauri && cargo test audit`
预期：FAIL（模块不存在）。

- [ ] **步骤 3：实现 repo/audit.rs**

从备份取完整实现：`git show 5f27180:src-tauri/src/repo/audit.rs`。`prune` 用 `DELETE FROM audit_logs WHERE timestamp < ?`（比较 RFC3339 字符串，UTC 有序）。

- [ ] **步骤 4：运行测试确认通过**

运行：`cd src-tauri && cargo test audit`
预期：3 个测试 PASS。

- [ ] **步骤 5：Commit**

```bash
git add src-tauri/src/repo/audit.rs src-tauri/src/repo/mod.rs
git commit -m "feat(logs): audit_logs repository with retention pruning"
```

---

### 任务 4：repo/learning.rs — get_plan

**文件：**
- 修改：`src-tauri/src/repo/learning.rs`

**说明：** 备份参考：`git show 3fb45a6:src-tauri/src/repo/learning.rs`（`get_plan` 函数）。签名：`pub fn get_plan(db: &Database, id: &str) -> Result<Option<LearningPlan>, String>`，SQL 与 `list_plans` 相同字段但带 `WHERE id = ?1`。

- [ ] **步骤 1：编写失败的测试**（追加到 learning.rs `#[cfg(test)]`；若该文件无 tests 模块则新建）

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Database {
        let db = Database::new(":memory:").expect("in-memory db");
        db.migrate().expect("migrate");
        db
    }

    #[test]
    fn get_plan_returns_existing_plan() {
        let db = test_db();
        let plan = create_plan(&db, "title", "goal", &[]).unwrap();
        let fetched = get_plan(&db, &plan.id).unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().title, "title");
    }

    #[test]
    fn get_plan_returns_none_for_missing() {
        let db = test_db();
        assert!(get_plan(&db, "nope").unwrap().is_none());
    }
}
```

> 若远端 `create_plan` 签名与备份不同（备份：`create_plan(db, title, goal, kp_ids)`），以远端 `repo/learning.rs` 实际签名为准调整测试。

- [ ] **步骤 2：运行测试确认失败**

运行：`cd src-tauri && cargo test get_plan`
预期：FAIL（`get_plan` 未定义）。

- [ ] **步骤 3：实现 get_plan**

按远端 `list_plans` 的字段映射（`plan_from_row`）写单行查询。

- [ ] **步骤 4：运行测试确认通过**

运行：`cd src-tauri && cargo test get_plan`
预期：PASS。

- [ ] **步骤 5：Commit**

```bash
git add src-tauri/src/repo/learning.rs
git commit -m "feat(chat): add get_plan repository function"
```

---

### 任务 5：FTS 搜索修复（远端 bug）

**文件：**
- 修改：`src-tauri/src/repo/knowledge.rs`（`search_kps`）
- 修改：`src-tauri/src/repo/source.rs`（`search_sources`）

**说明：** 远端写法 `WHERE kp_fts MATCH ?1` 直接查 external-content 表会报 "no such column"。按备份 5c21026 重写。参考：`git show 5f27180:src-tauri/src/repo/knowledge.rs`（`search_kps` 完整 SQL，见规格 3.1）。

**search_kps 新实现：**

```rust
pub fn search_kps(db: &Database, query: &str) -> Result<Vec<KnowledgePoint>, String> {
    let escaped = format!("\"{}\"", query.replace('"', "\"\""));
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT kp.id, kp.title, kp.summary, kp.content, kp.tags, kp.source_ids, kp.created_at
             FROM knowledge_points kp
             JOIN (SELECT rowid, rank FROM kp_fts WHERE kp_fts MATCH ?1) fts ON kp.rowid = fts.rowid
             ORDER BY fts.rank",
        )
        .map_err(|e| e.to_string())?;
    let kps: Vec<KnowledgePoint> = stmt
        .query_map([&escaped], kp_from_row)
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(kps)
}
```

`search_sources` 同理（表 `sources_fts`，字段映射用 `source_from_row`）。**注意：`kp_fts`/`sources_fts` 是 external-content 表，rowid 与主表对齐**（v1 建表时若 FTS 用 `content=''` 需确认 rowid 对应主表 rowid；备份 SQL 假设 rowid 对齐，照抄）。

- [ ] **步骤 1：编写失败的测试**（追加到 knowledge.rs `#[cfg(test)]`）

```rust
#[test]
fn search_kps_matches_via_fts() {
    let db = test_db();
    create_kp(&db, "Rust ownership", "所有权", "Rust 的所有权系统", &[], &[]).unwrap();
    let hits = search_kps(&db, "所有权").unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].title, "Rust ownership");
}

#[test]
fn search_kps_escapes_special_chars() {
    let db = test_db();
    create_kp(&db, "quotes", "引号", "带引号 \" 的内容", &[], &[]).unwrap();
    // 引号/操作符不应导致 FTS 语法错误
    let hits = search_kps(&db, "\"quote\"").unwrap();
    assert!(hits.len() <= 1);
}
```

> 若远端 `create_kp` 签名不同，以实际签名为准（备份：`create_kp(db, title, summary, content, tags, source_ids)`）。

- [ ] **步骤 2：运行测试确认失败**

运行：`cd src-tauri && cargo test search_kps`
预期：FAIL（当前远端 SQL 报 "no such column: kp_fts" 或类似）。

- [ ] **步骤 3：实现 FTS 重写**

按上文重写 `search_kps` + `search_sources`（source 侧写一个 `search_sources_matches_via_fts` 测试同理）。

- [ ] **步骤 4：运行测试确认通过**

运行：`cd src-tauri && cargo test search_kps search_sources`
预期：PASS。

- [ ] **步骤 5：Commit**

```bash
git add src-tauri/src/repo/knowledge.rs src-tauri/src/repo/source.rs
git commit -m "fix(search): rewrite FTS queries for external-content tables"
```

---

### 任务 6：级联删除补 plan.kp_ids 清理

**文件：**
- 修改：`src-tauri/src/repo/knowledge.rs`（`delete_kp`）

**说明：** 远端 `delete_kp` 已级联 quiz_attempts/quiz_questions/mastery_records/relations。对照备份 9a18b77 补：从所有 `learning_plans.kp_ids`（JSON 字符串数组）中移除被删 kp 的 id。

**补充 SQL（在事务内、删 knowledge_points 前执行）：**

```rust
// Remove the deleted KP id from every plan's kp_ids JSON array.
let plans: Vec<(String, String)> = {
    let mut stmt = tx.prepare("SELECT id, kp_ids FROM learning_plans").map_err(|e| e.to_string())?;
    stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect()
};
for (plan_id, kp_ids_str) in plans {
    let ids: Vec<String> = serde_json::from_str(&kp_ids_str).unwrap_or_default();
    if ids.iter().any(|i| i == id) {
        let filtered = ids.into_iter().filter(|i| i != id).collect::<Vec<_>>();
        tx.execute(
            "UPDATE learning_plans SET kp_ids = ?1 WHERE id = ?2",
            rusqlite::params![serde_json::to_string(&filtered).unwrap_or_else(|_| "[]".into()), plan_id],
        ).map_err(|e| e.to_string())?;
    }
}
```

- [ ] **步骤 1：编写失败的测试**

```rust
#[test]
fn delete_kp_removes_id_from_plans() {
    let db = test_db();
    let kp = create_kp(&db, "kp", "s", "c", &[], &[]).unwrap();
    let plan = crate::repo::learning::create_plan(&db, "p", "g", &[kp.id.clone()]).unwrap();
    delete_kp(&db, &kp.id).unwrap();
    let plan2 = crate::repo::learning::get_plan(&db, &plan.id).unwrap().unwrap();
    assert!(!plan2.kp_ids.contains(&kp.id));
}
```

- [ ] **步骤 2：运行测试确认失败**

运行：`cd src-tauri && cargo test delete_kp_removes`
预期：FAIL（kp_ids 仍含被删 id）。

- [ ] **步骤 3：实现**

在 `delete_kp` 事务内补上述逻辑。

- [ ] **步骤 4：运行测试确认通过**

运行：`cd src-tauri && cargo test delete_kp_removes`
预期：PASS。

- [ ] **步骤 5：Commit**

```bash
git add src-tauri/src/repo/knowledge.rs
git commit -m "fix(knowledge): remove deleted KP id from plan kp_ids"
```

---

### 任务 7：SM-2 每日限制

**文件：**
- 修改：`src-tauri/src/api/ai_routes.rs`（`update_mastery` handler + `should_advance_sm2` 辅助）

**说明：** 备份参考：`git show 88db2c6:src-tauri/src/api/ai_routes.rs`（守卫逻辑 + `should_advance_sm2` + 单测）。适配点：远端 handler 是 `blocking::run(move || {...})` 闭包结构——守卫与 `existing` 读取都在闭包内完成；audit 事件用远端已有的 tracing 模式（duration_ms + `advanced: true/false`）。

```rust
/// SM-2 advances at most once per day per KP: a quiz session answers several
/// questions in a row and each answer calls update_mastery; without this guard
/// a single session would count as several reviews and inflate the interval.
fn should_advance_sm2(last_reviewed_at: Option<&str>, now: chrono::DateTime<chrono::Utc>) -> bool {
    let Some(last) = last_reviewed_at.and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok()) else {
        return true;
    };
    let last = last.with_timezone(&chrono::Utc);
    last.date_naive() != now.date_naive()
}
```

- [ ] **步骤 1：编写失败的单元测试**（追加 ai_routes.rs `#[cfg(test)]`，测试辅助函数）

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sm2_advances_when_no_previous_review() {
        assert!(should_advance_sm2(None, chrono::Utc::now()));
    }

    #[test]
    fn sm2_does_not_advance_twice_same_day() {
        let now = chrono::Utc::now();
        let last = now.to_rfc3339();
        assert!(!should_advance_sm2(Some(&last), now));
    }

    #[test]
    fn sm2_advances_next_day() {
        let now = chrono::Utc::now();
        let yesterday = (now - chrono::Duration::days(1)).to_rfc3339();
        assert!(should_advance_sm2(Some(&yesterday), now));
    }
}
```

- [ ] **步骤 2：运行测试确认失败**

运行：`cd src-tauri && cargo test sm2_advances`
预期：FAIL（`should_advance_sm2` 未定义）。

- [ ] **步骤 3：实现守卫**

在 `update_mastery` 的 `blocking::run` 闭包内、读取 `existing` 之后：若 `existing` 存在且 `!should_advance_sm2(...)`，返回现有 record（audit 事件 `advanced: false`）；否则照常计算。从备份 `git show 88db2c6` 取完整 diff 适配（注意备份在闭包外读 existing，此处移入闭包；`start` 计时在闭包外）。

- [ ] **步骤 4：运行测试确认通过**

运行：`cd src-tauri && cargo test sm2_advances && cargo check`
预期：PASS + 编译通过。

- [ ] **步骤 5：Commit**

```bash
git add src-tauri/src/api/ai_routes.rs
git commit -m "fix(ai): advance SM-2 at most once per day per knowledge point"
```

---

### 任务 8：chat 后端 — ai_routes.rs chat handler

**文件：**
- 修改：`src-tauri/src/api/ai_routes.rs`

**说明：** 结构体与 handler 参考备份 `git show e7c503d:src-tauri/src/api/ai_routes.rs`（ChatRequest/ChatMessageItem/ChatContext/ChatAction/ChatResponse + `chat` handler），**适配新架构**（此前 rebase 已验证的适配版，要点如下）。`ChatAction` 的 `type` 用 `#[serde(rename = "type")]`。

**结构体定义（追加到 ai_routes.rs，`ChatRequest` 等）：**

```rust
#[derive(serde::Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessageItem>,
    pub context: Option<ChatContext>,
}

#[derive(serde::Deserialize)]
pub struct ChatMessageItem {
    pub role: String,
    pub content: String,
}

#[derive(serde::Deserialize)]
pub struct ChatContext {
    pub plan_id: Option<String>,
    pub current_kp_id: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ChatAction {
    #[serde(rename = "type")]
    pub action_type: String,
    pub label: String,
    pub payload: serde_json::Value,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ChatResponse {
    pub content: String,
    pub actions: Vec<ChatAction>,
}
```

**适配要点（与备份差异）：**

```rust
pub async fn chat(
    State(state): State<&'static AppState>,
    Json(req): Json<ChatRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    use crate::repo::{knowledge, learning};

    let llm_config = match blocking::run(move || repo::settings::resolve_llm_config(state.db, "chat")).await {
        Ok(v) => v,
        Err((_, e)) => return Err(map_llm_resolve_err(e)),
    };
    let llm = crate::ai::create_provider(llm_config)
        .map_err(|e| (StatusCode::SERVICE_UNAVAILABLE, e))?;

    // 上下文注入：plan/kp 读取全走 blocking::run（get_plan / list_kps_by_ids / get_kp）
    // system prompt：中文学习教练行为指南（照抄备份）+ 增强：
    //   "用户表达想学新主题X时返回 {\"type\":\"start_research\",\"label\":\"🔍 研究：X\",\"payload\":{\"topic\":\"X\"}}"

    let user_prompt = req.messages.iter()
        .map(|m| format!("{}: {}", m.role, m.content))
        .collect::<Vec<_>>()
        .join("\n");
    let response = llm.chat(&system_prompt, &user_prompt).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let json_str = crate::ai::extract_json_payload(&response);
    let chat_resp: ChatResponse = serde_json::from_str(json_str)
        .unwrap_or_else(|_| ChatResponse { content: response, actions: vec![] });
    Ok((StatusCode::OK, Json(serde_json::to_value(&chat_resp).unwrap())))
}
```

**完整 system prompt 从备份取**（`git show e7c503d:src-tauri/src/api/ai_routes.rs` 中 `chat` 的 `concat!` 行为指南），按规格 1.1 追加 `start_research` 行为。`extract_json` 本地函数**不要**复制，用 `crate::ai::extract_json_payload`。

- [ ] **步骤 1：实现 chat 结构体 + handler**

按上文与备份实现（此任务无独立单测，验证 = `cargo check`；解析/审计逻辑的正确性靠任务 15 冒烟）。

- [ ] **步骤 2：编译验证**

运行：`cd src-tauri && cargo check`
预期：编译通过（警告可容忍，后续任务清理）。

- [ ] **步骤 3：Commit**

```bash
git add src-tauri/src/api/ai_routes.rs
git commit -m "feat(chat): add /api/ai/chat handler with context-aware system prompt"
```

---

### 任务 9：chat REST 端点 + 路由注册

**文件：**
- 创建：`src-tauri/src/api/chat_routes.rs`
- 修改：`src-tauri/src/api/mod.rs`（声明模块）
- 修改：`src-tauri/src/server.rs`（注册路由）

**说明：** 端点全部走 `blocking::run` + audit tracing 事件（模式与 sources.rs 一致）。`AppState` 已含 `db` + `api_token`（远端 ai_routes.rs）。

```rust
// GET  /api/chat/sessions
pub async fn list_sessions(State(state): State<&'static AppState>)
    -> Result<Json<serde_json::Value>, (StatusCode, String)>;
// POST /api/chat/sessions  body: { title?: string }
pub async fn create_session(State(state): State<&'static AppState>, Json(req): Json<CreateSessionRequest>)
    -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)>;
// GET  /api/chat/sessions/{id}/messages
pub async fn get_messages(State(state): State<&'static AppState>, Path(id): Path<String>)
    -> Result<Json<serde_json::Value>, (StatusCode, String)>;
// POST /api/chat/messages  body: { session_id, role, content, actions?, context? }
pub async fn append_message(State(state): State<&'static AppState>, Json(req): Json<AppendMessageRequest>)
    -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)>;
// DELETE /api/chat/sessions/{id}
pub async fn delete_session(State(state): State<&'static AppState>, Path(id): Path<String>)
    -> Result<StatusCode, (StatusCode, String)>;
// POST /api/chat/sessions/{id}/plan  body: { plan_id: string }
pub async fn set_session_plan(State(state): State<&'static AppState>, Path(id): Path<String>, Json(req): Json<SetPlanRequest>)
    -> Result<StatusCode, (StatusCode, String)>;
```

**server.rs 注册（在 require_token 中间件之内的路由区）：**

```rust
.route("/api/chat/sessions", get(chat_routes::list_sessions).post(chat_routes::create_session))
.route("/api/chat/sessions/{id}", axum::routing::delete(chat_routes::delete_session))
.route("/api/chat/sessions/{id}/messages", get(chat_routes::get_messages))
.route("/api/chat/sessions/{id}/plan", axum::routing::post(chat_routes::set_session_plan))
.route("/api/chat/messages", axum::routing::post(chat_routes::append_message))
.route("/api/ai/chat", axum::routing::post(ai_routes::chat))
```

- [ ] **步骤 1：实现 chat_routes.rs**

按上文签名 + 远端 sources.rs 的 handler 模式（`blocking::run` + `#[instrument]` + audit `tracing::info!` 事件）。`session_id` 不存在时返回 404。

- [ ] **步骤 2：注册路由**

修改 server.rs 加 6 条路由（见上文）。

- [ ] **步骤 3：编译验证**

运行：`cd src-tauri && cargo check`
预期：通过。

- [ ] **步骤 4：Commit**

```bash
git add src-tauri/src/api/chat_routes.rs src-tauri/src/api/mod.rs src-tauri/src/server.rs
git commit -m "feat(chat): chat sessions REST API and route registration"
```

---

### 任务 10：日志后端 — tracing + AuditDbLayer + 中间件 + 端点

**文件：**
- 修改：`src-tauri/Cargo.toml`
- 创建：`src-tauri/src/tracing_layer.rs`
- 创建：`src-tauri/src/api/logs.rs`
- 修改：`src-tauri/src/lib.rs`（subscriber 初始化）
- 修改：`src-tauri/src/server.rs`（audit_middleware）
- 修改：所有 `src-tauri/src/api/*.rs` handler（`#[instrument]` + audit 事件）

**说明：** 备份参考：
- `git show 7c084f1:src-tauri/src/tracing_layer.rs`（AuditDbLayer，完整）
- `git show 6a7cf1f:src-tauri/src/lib.rs`（subscriber 初始化，适配远端 lib.rs setup 流程）
- `git show a2b78fd:src-tauri/src/server.rs`（audit_middleware，完整）
- `git show 8d01003`（handler 埋点 diff，适配 blocking::run 模式）
- `git show 5f27180:src-tauri/src/api/logs.rs`（ingest_logs + 校验/截断/脱敏，完整，本任务已核实）

**Cargo.toml 追加：**

```toml
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
tracing-appender = "0.2"
```

**lib.rs 初始化要点（远端 setup 内、db 创建后）：**

```rust
let log_dir = app_dir.join("logs");
std::fs::create_dir_all(&log_dir).unwrap();
let file_appender = tracing_appender::rolling::daily(&log_dir, "lexio.log");
let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
// 保活 guard：let _guard = Box::leak(Box::new(guard));
let audit_layer = crate::tracing_layer::AuditDbLayer::new(db);
let log_level = std::env::var("LEXIO_LOG_LEVEL").unwrap_or_else(|_| "info".into());
let env_filter = tracing_subscriber::EnvFilter::try_new(&log_level)
    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
let file_layer = tracing_subscriber::fmt::layer().with_writer(non_blocking).with_target(true);
tracing_subscriber::registry()
    .with(env_filter)
    .with(audit_layer)
    .with(file_layer)
    .init();
```

**audit_middleware（server.rs）：** 参考备份 a2b78fd 完整实现；适配点——注册位置为**最外层**（先于认证，记录 401）。axum 的 layer 顺序：**最后 `.layer()` 调用的是最外层**。远端现有顺序为 `... .layer(require_token).layer(cors())`（cors 最外层）；需改为：

```rust
.layer(cors())
.layer(middleware::from_fn_with_state(state, auth::require_token))
.layer(middleware::from_fn(audit_middleware)) // 最后注册 = 最外层 = 先于 auth 执行
```

请求顺序：audit → cors → require_token → handler。跳过路径：`/api/health`、`/api/auth/token`、`/api/logs/batch`。错误体收集 + `looks_like_internal_error` 清洗逻辑照抄备份。

**handler 埋点：** 对远端现有 handler（sources/knowledge/quiz/learning/settings/relation/ai_routes）逐个加 `#[instrument(skip(state, ...), fields(path = "..."))]` + `tracing::info!(target: "audit", ...)`。**合并模式**（rebase 期间已验证）：保留远端 `blocking::run` 结构，`let start = std::time::Instant::now();` 放闭包前，闭包后 `duration_ms` + audit 事件；audit 需要的 req 字段在闭包前 clone。备份 8d01003 diff 是直接参考。

**logs.rs：** `git show 5f27180:src-tauri/src/api/logs.rs` 完整照抄（含 `#[cfg(test)]` 测试与 `mask_sensitive`/`truncate`/`validate_entry`）；`app(state)` 注册 `.route("/api/logs/batch", axum::routing::post(logs::ingest_logs))`。

- [ ] **步骤 1：加依赖 + 编译**

改 Cargo.toml，运行：`cd src-tauri && cargo check`（此时依赖可解析）。

- [ ] **步骤 2：实现 tracing_layer.rs**

`git show 7c084f1:src-tauri/src/tracing_layer.rs` 取完整实现；`AuditDbLayer::new(db: &'static Database)`；`on_event` 中 `filter.target() == "audit"` 时构造 `AuditRecord` 并 `repo::audit::insert` + `repo::audit::prune(db, 30)`（每次写入顺带清理）。写库失败静默（`let _ = ...`）。

- [ ] **步骤 3：实现 logs.rs**

`git show 5f27180:src-tauri/src/api/logs.rs` 照抄（依赖 `repo::audit::batch_insert`，任务 3 已实现；`crate::models::new_id` 远端已有）。

- [ ] **步骤 4：初始化 subscriber + audit_middleware + 路由**

lib.rs 初始化（注意 guard 保活）+ server.rs 注册 audit_middleware 与 `/api/logs/batch` 路由 + `use crate::api::{... logs ...}`。

- [ ] **步骤 5：handler 埋点**

对全部 handler 加 `#[instrument]` + audit 事件（按合并模式）。验证：`cd src-tauri && cargo check`。

- [ ] **步骤 6：运行测试**

运行：`cd src-tauri && cargo test`
预期：全部 PASS（含 logs.rs 的校验/脱敏测试、audit repo 测试）。

- [ ] **步骤 7：Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/tracing_layer.rs src-tauri/src/api/logs.rs src-tauri/src/lib.rs src-tauri/src/server.rs src-tauri/src/api/
git commit -m "feat(logs): tracing subscriber, audit middleware and log ingestion"
```

---

### 任务 11：前端 types + api client

**文件：**
- 修改：`src/types.ts`
- 修改：`src/api/client.ts`

**说明：** 备份参考：`git show 5f27180:src/types.ts`（ChatAction/ChatMessage 扩展/ChatRequest/ChatResponse/ChatSession）+ `git show 5f27180:src/api/client.ts`（`api.ai.chat`）。适配：`ChatAction.type` 增加 `"start_research"`，payload 增加 `topic?: string`；`ChatSession` 为新类型；**保留远端 client.ts 的 getApiBase/token 机制**。

```ts
export interface ChatAction {
  type: "navigate_learning" | "start_quiz" | "view_source" | "start_research";
  label: string;
  payload: { kpId?: string; kpTitle?: string; sourceId?: string; topic?: string };
}

export interface ChatMessage {
  role: "user" | "assistant";
  content: string;
  actions?: ChatAction[];
  context?: { plan?: LearningPlan; kps?: KnowledgePoint[] };
}

export interface ChatRequest {
  messages: { role: string; content: string }[];
  context?: { plan_id?: string; current_kp_id?: string };
}

export interface ChatResponse {
  content: string;
  actions: ChatAction[];
}

export interface ChatSession {
  id: string;
  title: string;
  plan_id: string | null;
  message_count: number;
  updated_at: string;
}
```

**client.ts 追加（ai 段）：**

```ts
chat: (data: ChatRequest, signal?: AbortSignal) =>
  request<ChatResponse>("/ai/chat", { method: "POST", body: JSON.stringify(data), signal }),
```

**client.ts 追加（新 chat 段，模式照 settings 段）：**

```ts
chatApi: {
  listSessions: (signal?: AbortSignal) => request<ChatSession[]>("/chat/sessions", { signal }),
  createSession: (title: string, signal?: AbortSignal) =>
    request<ChatSession>("/chat/sessions", { method: "POST", body: JSON.stringify({ title }), signal }),
  getMessages: (sessionId: string, signal?: AbortSignal) =>
    request<ChatMessage[]>("/chat/sessions/${sessionId}/messages", { signal }),
  appendMessage: (data: { session_id: string; role: string; content: string; actions?: ChatAction[]; context?: unknown }, signal?: AbortSignal) =>
    request<ChatMessage>("/chat/messages", { method: "POST", body: JSON.stringify(data), signal }),
  deleteSession: (sessionId: string, signal?: AbortSignal) =>
    request<void>(`/chat/sessions/${sessionId}`, { method: "DELETE", signal }),
  setSessionPlan: (sessionId: string, planId: string, signal?: AbortSignal) =>
    request<void>(`/chat/sessions/${sessionId}/plan`, { method: "POST", body: JSON.stringify({ plan_id: planId }), signal }),
},
```

> 注意：远端 `request()` 对空响应（DELETE/204）的处理方式——若远端 `request<void>` 有差异，以远端实现为准调整。

- [ ] **步骤 1：改 types.ts**

按上文追加/修改类型（保留远端所有现有类型）。

- [ ] **步骤 2：改 client.ts**

加 `api.ai.chat` + `api.chatApi`（上文）。

- [ ] **步骤 3：编译验证**

运行：`npm run build`
预期：tsc 通过（vite 产物可忽略后续任务再构建）。

- [ ] **步骤 4：Commit**

```bash
git add src/types.ts src/api/client.ts
git commit -m "feat(chat): chat API types and client methods"
```

---

### 任务 12：前端 logger

**文件：**
- 创建：`src/utils/logger.ts`
- 修改：`src/main.tsx`（`logger.start()`）
- 修改：`src/App.tsx` 或入口（全局未捕获错误/未处理 rejection 埋点）

**说明：** 备份参考：`git show 5f27180:src/utils/logger.ts`（完整，本任务已核实）+ `git show 5f27180:src/main.tsx`。适配：`getApiBase` 从远端 `src/api/client.ts` 导入（远端已导出？**检查**：若远端 client.ts 未导出 `getApiBase`，则导出它或 logger 内复用 `API_BASE`/token 逻辑——首选导出远端 `getApiBase`）；上传带 `X-Lexio-Token` 头（用远端的 token 获取逻辑，参考 `client.ts` 内 `getToken`/`cachedToken`）。

```ts
import { getApiBase } from "../api/client";

export interface LogEntry {
  level: "info" | "warn" | "error";
  category: string;
  action: string;
  user_action?: string;
  timestamp?: string;
  params_summary?: Record<string, unknown>;
  result_summary?: Record<string, unknown>;
  duration_ms?: number;
  error_message?: string;
}

// 实现照抄备份 logger.ts（BATCH_SIZE=20, FLUSH_INTERVAL=5000, MAX_BUFFER=100）
// flush() 内：fetch(`${base}/logs/batch`, { method: "POST", headers: { "Content-Type": "application/json", "X-Lexio-Token": <token> }, body: JSON.stringify({ logs: batch }) })
export const logger = new Logger();
```

- [ ] **步骤 1：检查远端 client.ts 是否导出 getApiBase**

运行：`grep -n "export.*getApiBase\|function getApiBase" src/api/client.ts`
若未导出：在 client.ts 中把 `getApiBase` 改为 `export function`（或导出）。

- [ ] **步骤 2：创建 logger.ts**

照抄备份实现 + 上传带 token（参考 client.ts 的 token 获取）。

- [ ] **步骤 3：接线**

main.tsx：`logger.start()`（render 前）；入口加：

```ts
window.addEventListener("unhandledrejection", (e) => {
  logger.log({ level: "error", category: "ui", action: "unhandled_rejection",
    error_message: e.reason instanceof Error ? e.reason.message : String(e.reason) });
});
window.addEventListener("error", (e) => {
  logger.log({ level: "error", category: "ui", action: "window_error", error_message: e.message });
});
```

- [ ] **步骤 4：编译验证**

运行：`npm run build`
预期：通过。

- [ ] **步骤 5：Commit**

```bash
git add src/utils/logger.ts src/main.tsx src/api/client.ts src/App.tsx
git commit -m "feat(logs): frontend logger with batch upload"
```

---

### 任务 13：useChat 重构（多会话 + 触发词 + start_research）

**文件：**
- 修改：`src/hooks/useChat.ts`

**说明：** 基于备份 5f27180 的 useChat（触发词 `startsWith` + 长度检查、sameTopic、planContext{plan_id,title}、logger 埋点、研究结果 KP 按钮），叠加多会话管理。**参考顺序：** 先 `git show 5f27180:src/hooks/useChat.ts` 理解原逻辑，再按本任务改造。

**核心结构：**

```ts
export function useChat() {
  const [sessions, setSessions] = useState<ChatSession[]>([]);
  const [activeSessionId, setActiveSessionId] = useState<string | null>(null);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [loading, setLoading] = useState(false);
  const [planContext, setPlanContext] = useState<{ plan_id: string; title: string } | null>(null);

  // 挂载：listSessions → 打开最近（无则 createSession）；getMessages → setMessages；
  //   planContext 由 session.plan_id 恢复（有 plan_id 时再查 listPlans 找标题，找不到则仅 plan_id）
  // newSession / switchSession(sessionId) / deleteSession(sessionId)
  // sendMessage(content)：
  //   1. 若 !activeSessionId 先 createSession
  //   2. appendMessage(user) 持久化（失败不阻塞，logger 记 chat_error）
  //   3. 触发词检测（备份逻辑）→ startResearch(content) → handleResearchResult
  //      （结果消息含 navigate_learning actions；setSessionPlan(activeSessionId, plan.id)）
  //   4. 否则 api.ai.chat({ messages, context: planContext }) → appendMessage(assistant, actions, context)
  //   5. resp.actions 中若含 start_research → 执行研究（topic 取 payload.topic 或消息原文）
  // handleResearchResult：与备份一致（planContext 设置 + KP 按钮消息）
  // clearPlan：保留（setPlanContext(null)）
  return { sessions, activeSessionId, messages, loading, sendMessage, clearPlan,
           newSession, switchSession, deleteSession };
}
```

**关键适配点：**
- `appendMessage` 的 actions/context 参数：后端存 JSON 字符串，前端序列化 `JSON.stringify(actions)` 传入
- 恢复会话时 `planContext`：`session.plan_id` 存在时，用 `api.learning.listPlans()` 找到对应 plan 标题；找不到则 `{ plan_id, title: "" }`
- 触发词逻辑、sameTopic、logger 埋点与备份完全一致（`git show 5f27180` 照抄该部分）

- [ ] **步骤 1：重写 useChat.ts**

按上文结构实现（详细逻辑照抄备份 + 多会话包装）。

- [ ] **步骤 2：编译验证**

运行：`npm run build`
预期：通过（此时 ChatPanel 还未适配可能报 props 错——若报错，先临时最小化 ChatPanel 使用面，任务 14 补齐）。

- [ ] **步骤 3：Commit**

```bash
git add src/hooks/useChat.ts
git commit -m "feat(chat): rewrite useChat with sessions persistence and start_research"
```

---

### 任务 14：前端组件（ChatPanel / MessageBubble / Layout / Content）

**文件：**
- 修改：`src/components/Chat/MessageBubble.tsx`
- 修改：`src/components/Chat/MessageBubble.css`
- 修改：`src/components/Chat/ChatPanel.tsx`
- 修改：`src/components/Chat/ChatPanel.css`
- 修改：`src/components/Layout.tsx`
- 修改：`src/components/Content.tsx`
- 修改：`src/components/Content/LearningView.tsx`（start_quiz 直接开测）
- 修改：`src/components/Content/SourceViewer.tsx`（view_source modal，若远端有该组件；无则新建）

**说明：** 备份参考：`git show 5f27180:src/components/Chat/MessageBubble.tsx`（action 按钮）、`git show 5f27180:src/components/Chat/ChatPanel.tsx`、`git show bc53733:src/components/Layout.tsx`（导航管道）、`git show bc53733:src/components/Content.tsx`、`git show d15c755`（start_quiz/view_source 行为）。**保留远端已有**：MessageBubble 的 markdown + rehype-sanitize、ChatPanel 的 API key banner（`needsApiKeySetup`）+ 示例话题、Content 的常挂载 hidden 结构 + onOpenSettings。

**MessageBubble：** 备份实现 + 保留远端 sanitize：

```tsx
interface Props { message: ChatMessage; onAction?: (action: ChatAction) => void; }
// message.actions 渲染为 <button className="btn-action">（照抄备份）
```

**ChatPanel（会话侧栏 + 消息区 + 输入）：**

```tsx
interface Props {
  onOpenSettings?: () => void;
  onNavigate: (action: ChatAction) => void;
}
// 左侧栏：sessions.map（标题 + 删除按钮）+「新对话」按钮
// 消息区：照抄远端（banner + empty 示例话题 + messages.map(<MessageBubble onAction={onNavigate}>) + loading）
```

**Layout：** `onChatNavigate`（备份 bc53733）+ 保留远端 `onOpenSettings`：

```tsx
<Content view={view} selectedKpId={selectedKpId}
  onOpenSettings={() => setView("settings")} onChatNavigate={handleChatNavigate} />
```

**Content：** 备份导航 prop + 远端常挂载结构：

```tsx
interface Props {
  view: View;
  selectedKpId: string | null;
  onOpenSettings?: () => void;
  onChatNavigate: (action: ChatAction) => void;
}
// <div className="content-pane" hidden={view !== "chat"}>
//   <ChatPanel onOpenSettings={onOpenSettings} onNavigate={onChatNavigate} />
// </div>
// {view === "settings" && <SettingsView />} 等（照抄远端）
```

**action 分发（Content 或独立 hook）：**

```ts
function handleChatAction(action: ChatAction) {
  switch (action.type) {
    case "navigate_learning": setSelectedKpId(action.payload.kpId ?? null); setView("learning"); break;
    case "start_quiz": setSelectedKpId(action.payload.kpId ?? null); setView("learning"); /* LearningView 自动开测 */ break;
    case "view_source": /* SourceViewer modal 打开 action.payload.sourceId */ break;
    case "start_research": /* useChat 内已处理，此处忽略 */ break;
  }
}
```

- [ ] **步骤 1：MessageBubble + CSS**

备份实现 + 保留 sanitize。

- [ ] **步骤 2：ChatPanel + CSS**

会话侧栏 + 远端既有 UI。

- [ ] **步骤 3：Layout / Content**

导航管道 + 保留远端结构。

- [ ] **步骤 4：start_quiz / view_source 行为**

按备份 d15c755 适配 LearningView（接收"自动开测"标志）与 SourceViewer（若远端无此组件，按备份 `git show d15c755^:src/components/Content/SourceViewer.tsx` 创建；若远端已有则加 modal 触发）。

- [ ] **步骤 5：编译验证**

运行：`npm run build`
预期：通过（tsc 无错误）。

- [ ] **步骤 6：Commit**

```bash
git add src/components/
git commit -m "feat(chat): chat sessions UI, action buttons and navigation pipeline"
```

---

### 任务 15：验证与冒烟

**文件：** 无（验证 + 修复）

- [ ] **步骤 1：全量测试**

运行：`cd src-tauri && cargo test`
预期：全部 PASS（迁移 / chat repo / audit / FTS / SM-2 / logs 校验）。

- [ ] **步骤 2：全量构建**

运行：`cd src-tauri && cargo check && cd .. && npm run build`
预期：均通过。

- [ ] **步骤 3：冒烟（手动，dev 模式）**

运行：`npm run tauri dev`（或 `npm run dev` + 后端）
清单（规格第 4 节）：
1. 设置配置 DeepSeek API key（settings → 模型厂商）
2. 聊天：输入「我想学 Rust 所有权」→ 触发研究 → 结果含知识点导航按钮
3. 概念问答：「什么是所有权」→ LLM 直接回答
4. 触发词边界：「我不学习」→ 不触发研究（走 LLM 聊天）
5. 兜底：发「帮我研究一下 HTTP 缓存」→ LLM 回复含 start_research 按钮 → 点击执行研究
6. 动作按钮：点导航按钮 → 切到 learning 视图；测验按钮 → 开测
7. 刷新（F5）→ 会话恢复，消息完整
8. 多会话：新建/切换/删除
9. 搜索：顶部搜索一个关键词 → 返回结果（FTS 修复验证）
10. 测验后当天重复作答 → 不再推进（SM-2 限制）
11. 检查日志：`app_data_dir/logs/lexio.log` 有记录；SQLite `audit_logs` 表有行（含前端 batch 上传）

- [ ] **步骤 4：修复冒烟问题并 commit**

发现的问题逐项修复（本任务允许非 TDD 直接修复 + commit：`fix: ...`）。

---

## 依赖顺序

任务 1 → 2/3/4/5/6（repo 层，可部分并行）→ 7/8/9（chat 后端）→ 10（日志后端）→ 11/12/13/14（前端）→ 15（验证）。
任务 3（repo::audit）被任务 10（logs.rs / tracing_layer）依赖；任务 4（get_plan）被任务 8 依赖。
