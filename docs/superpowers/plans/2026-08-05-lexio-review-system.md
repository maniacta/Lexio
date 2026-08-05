# 复习系统 实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 实现完整的复习闭环——到期提醒入口、待复习列表、答题循环、结果总结、SM-2 掌握度更新。

**架构：** 后端新增一个 JOIN 查询（`reviews/due?with_kp=true`）聚合 mastery 和 knowledge_point 数据。前端新增 `ReviewView` 三阶段状态机（list → session → summary），侧栏新增「复习」tab 带红点数字徽标。答题循环复用现有 `QuizCard` 组件。

**技术栈：** Rust (Axum + rusqlite), React 19 + TypeScript, 复用现有 CSS 设计令牌

**设计规格：** `docs/superpowers/specs/2026-08-05-lexio-review-system-design.md`

---

## 新增/修改的文件清单

### Rust 后端 (`src-tauri/src/`)
| 文件 | 操作 | 职责 |
|------|------|------|
| `api/learning.rs` | 修改 | 解析 `with_kp` query param |
| `repo/learning.rs` | 修改 | 新增 `get_due_reviews_with_kp()` JOIN 查询 |

### React 前端 (`src/`)
| 文件 | 操作 | 职责 |
|------|------|------|
| `types.ts` | 修改 | 新增 `ReviewItem`、`ReviewResult` 类型 |
| `api/client.ts` | 修改 | 新增 `learning.getDueReviewsWithKp()` 方法 |
| `components/Layout.tsx` | 修改 | View 类型新增 `"review"`, review tab click handler |
| `components/Content.tsx` | 修改 | 分发 `ReviewView` 渲染 |
| `components/Sidebar.tsx` | 修改 | 新增「复习」tab + 红点数字徽标 |
| `components/Sidebar.css` | 修改 | 复习 tab 样式 + 红点数字样式 |
| `components/Content/ReviewView.tsx` | 🆕 | 三阶段状态机容器 |
| `components/Content/ReviewView.css` | 🆕 | 复习视图样式 |
| `components/Content/ReviewList.tsx` | 🆕 | 待复习列表（含勾选） |
| `components/Content/ReviewSession.tsx` | 🆕 | 答题循环（复用 QuizCard） |
| `components/Content/ReviewSummary.tsx` | 🆕 | 结果总结 |

---

### 任务 1：后端 — reviews/due 聚合查询

**文件：**
- 修改：`src-tauri/src/repo/learning.rs`
- 修改：`src-tauri/src/api/learning.rs`

- [ ] **步骤 1：在 repo 层新增 `get_due_reviews_with_kp()` 函数**

在 `src-tauri/src/repo/learning.rs` 的 `get_mastery_by_kp` 函数之后，添加：

```rust
/// Return due reviews along with the associated KnowledgePoint data.
pub fn get_due_reviews_with_kp(db: &Database) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();
    let mut stmt = conn
        .prepare(
            "SELECT m.id, m.kp_id, m.ease_factor, m.interval_days, m.repetitions,
                    m.next_review_at, m.last_reviewed_at,
                    kp.id, kp.title, kp.summary, kp.content, kp.tags, kp.source_ids, kp.created_at
             FROM mastery_records m
             JOIN knowledge_points kp ON m.kp_id = kp.id
             WHERE m.next_review_at <= ?1
             ORDER BY m.next_review_at ASC"
        )
        .map_err(|e| e.to_string())?;

    let items: Vec<serde_json::Value> = stmt
        .query_map([&now], |row| {
            Ok(serde_json::json!({
                "mastery": {
                    "id": row.get::<_, String>(0)?,
                    "kp_id": row.get::<_, String>(1)?,
                    "ease_factor": row.get::<_, f64>(2)?,
                    "interval_days": row.get::<_, i32>(3)?,
                    "repetitions": row.get::<_, i32>(4)?,
                    "next_review_at": row.get::<_, String>(5)?,
                    "last_reviewed_at": row.get::<_, Option<String>>(6)?
                },
                "knowledge_point": {
                    "id": row.get::<_, String>(7)?,
                    "title": row.get::<_, String>(8)?,
                    "summary": row.get::<_, String>(9)?,
                    "content": row.get::<_, String>(10)?,
                    "tags": {
                        let raw: String = row.get(11)?;
                        serde_json::from_str(&raw).unwrap_or_default()
                    },
                    "source_ids": {
                        let raw: String = row.get(12)?;
                        serde_json::from_str(&raw).unwrap_or_default()
                    },
                    "created_at": row.get::<_, String>(13)?
                }
            }))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(items)
}
```

- [ ] **步骤 2：修改 API handler 支持 `with_kp` 参数**

修改 `src-tauri/src/api/learning.rs` 的 `get_due_reviews` 函数，解析 query param：

```rust
use axum::extract::Query;
use std::collections::HashMap;

pub async fn get_due_reviews(
    State(state): State<&'static AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let with_kp = params.get("with_kp").map(|v| v == "true").unwrap_or(false);
    if with_kp {
        let items = repo::learning::get_due_reviews_with_kp(state.db)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
        Ok(Json(serde_json::json!(items)))
    } else {
        let records = repo::learning::get_due_reviews(state.db)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
        Ok(Json(serde_json::to_value(&records).unwrap()))
    }
}
```

- [ ] **步骤 3：验证后端**

```bash
cd /home/ubuntu/Lexio && cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5
```
预期：编译成功，无错误。

- [ ] **步骤 4：Commit**

```bash
git add src-tauri/src/repo/learning.rs src-tauri/src/api/learning.rs
git commit -m "feat(review): add get_due_reviews_with_kp JOIN query"
```

---

### 任务 2：前端类型定义 + API 客户端

**文件：**
- 修改：`src/types.ts`
- 修改：`src/api/client.ts`

- [ ] **步骤 1：新增 `ReviewItem` 和 `ReviewResult` 类型**

在 `src/types.ts` 中 `SettingsData` 类型之前添加：

```typescript
export interface ReviewItem {
  mastery: MasteryRecord;
  knowledge_point: KnowledgePoint;
}

export interface ReviewResult {
  kp_id: string;
  kp_title: string;
  is_correct: boolean;
  next_review_at: string;
}
```

- [ ] **步骤 2：在 API 客户端新增 `getDueReviewsWithKp` 方法**

在 `src/api/client.ts` 的 `learning` 对象中添加方法。在 `getDueReviews` 方法之后添加：

```typescript
    getDueReviewsWithKp: () => request<ReviewItem[]>("/learning/reviews/due?with_kp=true"),
```

确保文件顶部 import 了 `ReviewItem`。

- [ ] **步骤 3：检查 TypeScript 编译**

```bash
cd /home/ubuntu/Lexio && npx tsc --noEmit 2>&1 | head -20
```
预期：无类型错误（或仅有原有的无关警告）。

- [ ] **步骤 4：Commit**

```bash
git add src/types.ts src/api/client.ts
git commit -m "feat(review): add ReviewItem/ReviewResult types and API client method"
```

---

### 任务 3：路由分发 — Layout / Content / Sidebar

**文件：**
- 修改：`src/components/Layout.tsx`
- 修改：`src/components/Content.tsx`
- 修改：`src/components/Sidebar.tsx`
- 修改：`src/components/Sidebar.css`

- [ ] **步骤 1：Layout.tsx — 扩展 View 类型，新增 review 点击 handler**

在 `src/components/Layout.tsx` 中：

修改 `View` 类型定义：
```typescript
export type View = "chat" | "learning" | "settings" | "review";
```

修改 `handleSelectKp` 将 `setView("learning")` 保持不动（点击 KP 仍然进入学习视图）。

无需新增额外的 handler——Sidebar 通过 `onNavigate` 直接调用 `setView("review")`。

- [ ] **步骤 2：Content.tsx — 新增 review 分发**

在 `src/components/Content.tsx` 中：

```typescript
import ReviewView from "./Content/ReviewView";

// 在 return 的 JSX 中添加 review 分支：
{view === "settings" ? <SettingsView /> :
 view === "learning" ? <LearningView kpId={selectedKpId} /> :
 view === "review" ? <ReviewView /> :
 <ChatPanel />}
```

- [ ] **步骤 3：Sidebar.tsx — 新增「复习」tab + 红点数字**

在 `src/components/Sidebar.tsx` 中：

添加 state 用于到期数量：
```typescript
// 修改已有的 import：将 useState 扩展为 useState, useEffect
import { useState, useEffect } from "react";
import { api } from "../api/client";

// 在组件内部（现有代码之后）
const [dueCount, setDueCount] = useState<number>(0);

useEffect(() => {
  api.learning.getDueReviews().then(records => setDueCount(records.length)).catch(() => {});
}, [view]); // view 变化时刷新（从复习页返回时重新拉取）
```

在 sidebar-tabs 区域，knowledge 和 sources 两个 tab 之后，sidebar-footer 之前，添加复习 tab：

```tsx
<button
  className={`sidebar-tab review ${view === "review" ? "active" : ""}`}
  onClick={() => onNavigate("review")}
>
  复习
  {dueCount > 0 && <span className="review-badge">{dueCount}</span>}
</button>
```

- [ ] **步骤 4：Sidebar.css — 复习 tab 和红点样式**

在 `src/components/Sidebar.css` 中添加：

```css
.sidebar-tab.review {
  position: relative;
}

.review-badge {
  position: absolute;
  top: 2px;
  right: 4px;
  background: #ef4444;
  color: white;
  font-size: 11px;
  font-weight: 700;
  min-width: 18px;
  height: 18px;
  line-height: 18px;
  text-align: center;
  border-radius: 9px;
  padding: 0 4px;
}
```

- [ ] **步骤 5：验证编译**

```bash
cd /home/ubuntu/Lexio && npx tsc --noEmit 2>&1 | head -20
```
预期：无新增类型错误。

- [ ] **步骤 6：Commit**

```bash
git add src/components/Layout.tsx src/components/Content.tsx src/components/Sidebar.tsx src/components/Sidebar.css
git commit -m "feat(review): add review route in layout, content, and sidebar with badge"
```

---

### 任务 4：ReviewList — 待复习列表组件

**文件：**
- 创建：`src/components/Content/ReviewList.tsx`

- [ ] **步骤 1：创建 ReviewList 组件**

完整文件内容：

```tsx
import { useState } from "react";
import type { ReviewItem } from "../../types";

interface Props {
  items: ReviewItem[];
  onStart: (ids: string[]) => void;
}

export default function ReviewList({ items, onStart }: Props) {
  const [selected, setSelected] = useState<Set<string>>(
    () => new Set(items.map((i) => i.mastery.kp_id))
  );

  const toggle = (id: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const toggleAll = () => {
    if (selected.size === items.length) {
      setSelected(new Set());
    } else {
      setSelected(new Set(items.map((i) => i.mastery.kp_id)));
    }
  };

  const formatTimeAgo = (dateStr: string | null): string => {
    if (!dateStr) return "首次复习";
    const diff = Date.now() - new Date(dateStr).getTime();
    const days = Math.floor(diff / 86400000);
    if (days === 0) return "今天";
    if (days === 1) return "1 天前";
    return `${days} 天前`;
  };

  if (items.length === 0) {
    return (
      <div className="review-empty">
        <h3>暂无待复习知识点 🎉</h3>
        <p>完成测验后，系统会根据你的掌握程度自动安排复习。</p>
      </div>
    );
  }

  return (
    <div className="review-list">
      <div className="review-list-header">
        <h2>📋 待复习 ({items.length})</h2>
        <button className="btn-link" onClick={toggleAll}>
          {selected.size === items.length ? "取消全选" : "全选"}
        </button>
      </div>

      <div className="review-items">
        {items.map((item) => (
          <label
            key={item.mastery.kp_id}
            className={`review-item-card ${selected.has(item.mastery.kp_id) ? "selected" : ""}`}
          >
            <input
              type="checkbox"
              checked={selected.has(item.mastery.kp_id)}
              onChange={() => toggle(item.mastery.kp_id)}
            />
            <div className="review-item-info">
              <span className="review-item-title">📌 {item.knowledge_point.title}</span>
              <span className="review-item-meta">
                上次复习：{formatTimeAgo(item.mastery.last_reviewed_at)}
                &nbsp;·&nbsp;间隔：{item.mastery.interval_days} 天
                &nbsp;·&nbsp;连续正确：{item.mastery.repetitions} 次
              </span>
            </div>
          </label>
        ))}
      </div>

      <button
        className="btn-primary btn-start-review"
        disabled={selected.size === 0}
        onClick={() => onStart(Array.from(selected))}
      >
        开始复习 ({selected.size})
      </button>
    </div>
  );
}
```

- [ ] **步骤 2：验证编译**

```bash
cd /home/ubuntu/Lexio && npx tsc --noEmit 2>&1 | head -20
```

- [ ] **步骤 3：Commit**

```bash
git add src/components/Content/ReviewList.tsx
git commit -m "feat(review): add ReviewList component with selection"
```

---

### 任务 5：ReviewSession — 答题循环组件

**文件：**
- 创建：`src/components/Content/ReviewSession.tsx`

- [ ] **步骤 1：创建 ReviewSession 组件**

```tsx
import { useState, useEffect, useCallback, useRef } from "react";
import type { QuizQuestion, ReviewResult } from "../../types";
import { api } from "../../api/client";
import QuizCard from "./QuizCard";

interface Props {
  kpIds: string[];
  onComplete: (results: ReviewResult[]) => void;
}

export default function ReviewSession({ kpIds, onComplete }: Props) {
  const [index, setIndex] = useState(0);
  const [question, setQuestion] = useState<QuizQuestion | null>(null);
  const [result, setResult] = useState<{
    user_answer: string;
    is_correct: boolean;
    explanation: string;
    next_review_at: string;
  } | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [kpTitle, setKpTitle] = useState("");
  const resultsRef = useRef<ReviewResult[]>([]);
  const usedQuestionIds = useRef<Set<string>>(new Set());

  const fetchQuestion = useCallback(async (kpId: string) => {
    setLoading(true);
    setError(null);
    try {
      // Get KP title for display
      const kp = await api.knowledge.get(kpId);
      setKpTitle(kp.title);

      // Try to find an unused question from existing bank
      const existingQuestions = await api.quiz.getByKp(kpId);
      const unused = existingQuestions.filter((q) => !usedQuestionIds.current.has(q.id));
      let q: QuizQuestion;

      if (unused.length > 0) {
        q = unused[Math.floor(Math.random() * unused.length)];
      } else {
        // Generate new questions via AI
        const generated = await api.ai.generateQuiz(kpId, 1);
        if (generated.length === 0) throw new Error("无法生成题目");
        q = generated[0];
      }

      usedQuestionIds.current.add(q.id);
      setQuestion(q);
      setResult(null);
    } catch (e: any) {
      // If can't generate, skip this KP
      const reviewResult: ReviewResult = {
        kp_id: kpId,
        kp_title: kpTitle || "(加载失败)",
        is_correct: false,
        next_review_at: "",
      };
      resultsRef.current.push(reviewResult);
      setError(e.message);
      setQuestion(null);
      setIndex((i) => i + 1);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (index < kpIds.length) {
      fetchQuestion(kpIds[index]);
    } else {
      onComplete(resultsRef.current);
    }
  }, [index, kpIds, fetchQuestion, onComplete]);

  const submitAnswer = async (answer: string) => {
    if (!question) return;
    setLoading(true);
    try {
      const res = await api.quiz.submit(question.id, answer);
      await api.ai.updateMastery(question.kp_id, res.is_correct);
      // Get updated mastery for next_review_at
      const reviews = await api.learning.getDueReviews();
      const masteryForKp = reviews.find((r) => r.kp_id === question.kp_id);
      setResult({
        user_answer: answer,
        is_correct: res.is_correct,
        explanation: res.explanation,
        next_review_at: masteryForKp?.next_review_at || "",
      });
    } catch (e: any) {
      setError(e.message);
    } finally {
      setLoading(false);
    }
  };

  const nextQuestion = () => {
    if (question && result) {
      resultsRef.current.push({
        kp_id: question.kp_id,
        kp_title: kpTitle,
        is_correct: result.is_correct,
        next_review_at: result.next_review_at,
      });
    }
    setIndex((i) => i + 1);
  };

  if (index >= kpIds.length) {
    return (
      <div className="review-session-loading">
        <p>完成中...</p>
      </div>
    );
  }

  if (loading && !question) {
    return (
      <div className="review-session-loading">
        <p>准备题目中...</p>
      </div>
    );
  }

  if (error && !question) {
    return (
      <div className="review-session-error">
        <p>⚠️ 出题失败：{error}</p>
        <button className="btn-primary" onClick={nextQuestion}>
          跳过，继续下一个
        </button>
      </div>
    );
  }

  if (!question) {
    return (
      <div className="review-session-loading">
        <p>加载中...</p>
      </div>
    );
  }

  return (
    <div className="review-session">
      <div className="review-session-header">
        <span className="review-progress">
          第 {index + 1}/{kpIds.length} 题 — {kpTitle}
        </span>
      </div>

      <QuizCard
        question={question}
        result={
          result
            ? {
                question,
                user_answer: result.user_answer,
                is_correct: result.is_correct,
                explanation: result.explanation,
              }
            : null
        }
        loading={loading}
        onSubmit={submitAnswer}
        onNext={nextQuestion}
        isLast={index === kpIds.length - 1}
      />
    </div>
  );
}
```

- [ ] **步骤 2：验证编译**

```bash
cd /home/ubuntu/Lexio && npx tsc --noEmit 2>&1 | head -20
```

- [ ] **步骤 3：Commit**

```bash
git add src/components/Content/ReviewSession.tsx
git commit -m "feat(review): add ReviewSession quiz loop component"
```

---

### 任务 6：ReviewSummary — 结果总结组件

**文件：**
- 创建：`src/components/Content/ReviewSummary.tsx`

- [ ] **步骤 1：创建 ReviewSummary 组件**

```tsx
import type { ReviewResult } from "../../types";

interface Props {
  results: ReviewResult[];
  onBack: () => void;
}

export default function ReviewSummary({ results, onBack }: Props) {
  const correctCount = results.filter((r) => r.is_correct).length;
  const totalCount = results.length;

  const formatNextReview = (dateStr: string): string => {
    if (!dateStr) return "未更新";
    try {
      const date = new Date(dateStr);
      const diffDays = Math.round(
        (date.getTime() - Date.now()) / 86400000
      );
      if (diffDays <= 0) return "今天";
      if (diffDays === 1) return "明天";
      return `${diffDays} 天后`;
    } catch {
      return dateStr;
    }
  };

  return (
    <div className="review-summary">
      <div className="review-summary-header">
        <h2>✅ 复习完成！</h2>
        <p className="review-summary-score">
          正确：{correctCount} / {totalCount}
        </p>
      </div>

      <div className="review-summary-items">
        {results.map((r, i) => (
          <div key={i} className={`review-summary-item ${r.is_correct ? "correct" : "incorrect"}`}>
            <span className="review-summary-icon">{r.is_correct ? "✅" : "❌"}</span>
            <span className="review-summary-title">{r.kp_title}</span>
            <span className="review-summary-next">
              下次复习：{formatNextReview(r.next_review_at)}
            </span>
          </div>
        ))}
      </div>

      <button className="btn-primary" onClick={onBack}>
        返回待复习列表
      </button>
    </div>
  );
}
```

- [ ] **步骤 2：验证编译**

```bash
cd /home/ubuntu/Lexio && npx tsc --noEmit 2>&1 | head -20
```

- [ ] **步骤 3：Commit**

```bash
git add src/components/Content/ReviewSummary.tsx
git commit -m "feat(review): add ReviewSummary results component"
```

---

### 任务 7：ReviewView — 状态机容器 + 样式

**文件：**
- 创建：`src/components/Content/ReviewView.tsx`
- 创建：`src/components/Content/ReviewView.css`

- [ ] **步骤 1：创建 ReviewView 容器组件**

```tsx
import { useState, useEffect } from "react";
import type { ReviewItem, ReviewResult } from "../../types";
import { api } from "../../api/client";
import ReviewList from "./ReviewList";
import ReviewSession from "./ReviewSession";
import ReviewSummary from "./ReviewSummary";
import "./ReviewView.css";

type Phase = "list" | "session" | "summary";

export default function ReviewView() {
  const [phase, setPhase] = useState<Phase>("list");
  const [items, setItems] = useState<ReviewItem[]>([]);
  const [selectedIds, setSelectedIds] = useState<string[]>([]);
  const [results, setResults] = useState<ReviewResult[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadItems = async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await api.learning.getDueReviewsWithKp();
      setItems(data);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadItems();
  }, []);

  const handleStart = (ids: string[]) => {
    setSelectedIds(ids);
    setPhase("session");
  };

  const handleComplete = (sessionResults: ReviewResult[]) => {
    setResults(sessionResults);
    setPhase("summary");
  };

  const handleBack = () => {
    setPhase("list");
    loadItems(); // refresh due list
  };

  if (loading) {
    return (
      <div className="review-view">
        <div className="review-loading">加载中...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="review-view">
        <div className="review-error">
          <p>加载失败：{error}</p>
          <button className="btn-primary" onClick={loadItems}>重试</button>
        </div>
      </div>
    );
  }

  return (
    <div className="review-view">
      {phase === "list" && (
        <ReviewList items={items} onStart={handleStart} />
      )}
      {phase === "session" && (
        <ReviewSession kpIds={selectedIds} onComplete={handleComplete} />
      )}
      {phase === "summary" && (
        <ReviewSummary results={results} onBack={handleBack} />
      )}
    </div>
  );
}
```

- [ ] **步骤 2：创建 ReviewView 样式**

```css
.review-view {
  max-width: 720px;
  margin: 0 auto;
  padding: 24px;
}

.review-loading,
.review-error {
  text-align: center;
  padding: 48px 24px;
  color: var(--color-text-secondary);
}

.review-error .btn-primary {
  margin-top: 16px;
}

/* ── ReviewList ── */
.review-empty {
  text-align: center;
  padding: 64px 24px;
  color: var(--color-text-secondary);
}

.review-empty h3 {
  font-size: 20px;
  margin-bottom: 8px;
}

.review-list-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.review-list-header h2 {
  font-size: 22px;
  font-weight: 700;
}

.btn-link {
  background: none;
  border: none;
  color: var(--color-accent);
  cursor: pointer;
  font-size: 14px;
  text-decoration: underline;
}

.review-items {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin-bottom: 24px;
}

.review-item-card {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 14px 16px;
  border: 1px solid var(--color-border);
  border-radius: 10px;
  cursor: pointer;
  transition: background 0.15s, border-color 0.15s;
}

.review-item-card:hover {
  background: rgba(0, 0, 0, 0.03);
}

.review-item-card.selected {
  border-color: var(--color-accent);
  background: var(--color-sidebar-active, rgba(124, 111, 247, 0.08));
}

.review-item-card input[type="checkbox"] {
  width: 18px;
  height: 18px;
  accent-color: var(--color-accent);
  flex-shrink: 0;
}

.review-item-info {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}

.review-item-title {
  font-size: 15px;
  font-weight: 600;
}

.review-item-meta {
  font-size: 13px;
  color: var(--color-text-secondary);
}

/* ── ReviewSession ── */
.review-session-header {
  margin-bottom: 16px;
}

.review-progress {
  font-size: 14px;
  color: var(--color-text-secondary);
  font-weight: 500;
}

.review-session-loading,
.review-session-error {
  text-align: center;
  padding: 48px 24px;
  color: var(--color-text-secondary);
}

.review-session-error .btn-primary {
  margin-top: 16px;
}

/* ── ReviewSummary ── */
.review-summary-header {
  text-align: center;
  margin-bottom: 28px;
}

.review-summary-header h2 {
  font-size: 24px;
  margin-bottom: 8px;
}

.review-summary-score {
  font-size: 32px;
  font-weight: 700;
  color: var(--color-accent);
}

.review-summary-items {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-bottom: 28px;
}

.review-summary-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 16px;
  border-radius: 8px;
  font-size: 15px;
}

.review-summary-item.correct {
  background: rgba(34, 197, 94, 0.08);
}

.review-summary-item.incorrect {
  background: rgba(239, 68, 68, 0.08);
}

.review-summary-icon {
  font-size: 18px;
  flex-shrink: 0;
}

.review-summary-title {
  flex: 1;
  font-weight: 500;
}

.review-summary-next {
  font-size: 13px;
  color: var(--color-text-secondary);
}
```

- [ ] **步骤 3：验证编译**

```bash
cd /home/ubuntu/Lexio && npx tsc --noEmit 2>&1 | head -20
```
预期：无类型错误。

- [ ] **步骤 4：Commit**

```bash
git add src/components/Content/ReviewView.tsx src/components/Content/ReviewView.css
git commit -m "feat(review): add ReviewView state machine container and styles"
```

---

### 任务 8：端到端验证

- [ ] **步骤 1：编译检查前后端**

```bash
cd /home/ubuntu/Lexio && cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5
npx tsc --noEmit 2>&1 | tail -5
```
预期：两者均无错误。

- [ ] **步骤 2：检查 Sidebar import 一致性**

确认 `src/components/Sidebar.tsx` 顶部导入了 `useEffect`、`useState`（已有 `useState`，可能需要追加 `useEffect`）：

```typescript
import { useState, useEffect } from "react";
import { api } from "../api/client";
```

- [ ] **步骤 3：最终 commit**

```bash
git add -A
git commit -m "feat(review): complete review system implementation"
```

---

## 依赖关系

```
任务 1（后端API）  ──→ 任务 3 可以并行开始
任务 2（类型+客户端）──┤
                        ├──→ 任务 4（ReviewList）
                        │       └──→ 任务 7（ReviewView 容器）
                        ├──→ 任务 5（ReviewSession）───┘
                        └──→ 任务 6（ReviewSummary）───┘
                                                          └──→ 任务 8（验证）
```

任务 4、5、6 可以并行（各自独立组件），但都需要 2、3 完成。
