# Lexio：复习系统 设计规格

> 2025-08-05 | 状态: 已批准

## 1. 概述

为 Lexio 实现完整的复习闭环——把"测验→SM-2 追踪→到期提醒→复习答题→更新掌握度"的循环做完。这是 MVP 核心闭环最后缺失的一环。

### 核心需求

- **入口**：侧栏新增「复习」tab，红点数字显示到期知识点数量
- **待复习列表**：展示所有 `next_review_at <= now` 的知识点，含上次复习时间和间隔信息
- **复习答题**：选中知识点后逐个出题，优先从已有题库抽取，不足时 AI 即时生成
- **结果总结**：完成后展示对错统计 + 下次复习时间
- **SM-2 更新**：每次答题后更新 mastery record

---

## 2. 路由 & 入口

现有 `View` 类型扩展：

```typescript
export type View = "chat" | "learning" | "settings" | "review";
```

侧栏布局变更：

```
┌──────────┐
│ 知识点    │  ← tab
│ 资料      │  ← tab
│ 🔔 复习 3 │  ← tab（红点数字 = 到期数）
│           │
│ ─────────│
│    ⚙      │
└──────────┘
```

- 红点数字在侧栏挂载时调用 `GET /api/learning/reviews/due` 获取
- 桌面应用单用户，无需轮询，进入复习视图后刷新即可
- 点击进入 `review` 视图 → Content 区渲染 `ReviewView`

---

## 3. 复习视图：三阶段状态机

```
list ──(开始复习)──→ session ──(全部完成)──→ summary ──(返回)──→ list
```

### 3.1 阶段 1：待复习列表（ReviewList）

```
┌──────────────────────────────────────┐
│  📋 待复习                            │
│                                      │
│  ┌──────────────────────────────┐    │
│  │ 📌 Rust 所有权               │    │
│  │ 上次复习：3 天前  间隔：1 天   │    │
│  │ 连续正确：1 次               │    │
│  └──────────────────────────────┘    │
│  ┌──────────────────────────────┐    │
│  │ 📌 借用与引用                │    │
│  │ 上次复习：7 天前  间隔：6 天   │    │
│  └──────────────────────────────┘    │
│                                      │
│  [ 开始全部复习 ]   [ 选择知识点 ]      │
└──────────────────────────────────────┘
```

- 调用 `GET /api/learning/reviews/due?with_kp=true`
- 显示知识点标题、上次复习时间、当前间隔、连续正确次数
- 用户可单独勾选或全选，点击按钮进入答题阶段
- 若到期列表为空，显示「暂无待复习知识点 🎉」

### 3.2 阶段 2：答题循环（ReviewSession）

- 逐个对选中的知识点出题
- 每题逻辑：
  1. 调用 `GET /api/quiz/kp/:id` 查已有题库
  2. 从中筛选未被本次复习使用过的题目（去重策略见 §8）
  3. 若无未用过的题，调用 `POST /api/ai/generate-quiz` 动态生成
  4. 复用现有 `QuizCard` 组件展示题目
  5. 用户提交 → `POST /api/quiz/submit` 获取结果 → `POST /api/ai/update-mastery` 更新 SM-2
  6. 显示本道题结果 → 点击「下一题」continue

- 进度条：`第 2/5 题`

### 3.3 阶段 3：结果总结（ReviewSummary）

```
┌──────────────────────────────────────┐
│  ✅ 复习完成！                        │
│                                      │
│  正确：3 / 5                         │
│  ┌──────────────────────────────┐    │
│  │ ✅ Rust 所有权    下次：6 天后  │    │
│  │ ❌ 借用与引用     下次：明天    │    │
│  │ ✅ 生命周期       下次：15 天后 │    │
│  └──────────────────────────────┘    │
│                                      │
│  [ 返回待复习列表 ]                    │
└──────────────────────────────────────┘
```

- 每题结果本地收集（在 ReviewSession 中累积），无需额外 API
- 下次复习时间来自 `POST /api/ai/update-mastery` 的 SM-2 输出
- 点击返回 → 重新拉取到期列表（已完成的可能不再出现）

---

## 4. 数据流

```
ReviewView 挂载
  → GET /api/learning/reviews/due?with_kp=true
  → 渲染阶段 1：ReviewList
  → 用户选中 KP，点击「开始复习」
  → 过渡到阶段 2：ReviewSession
      → 对每个 KP：
          → GET /api/quiz/kp/:id  查已有题
          → 若不足 → POST /api/ai/generate-quiz
          → 展示 QuizCard
          → 用户作答 → POST /api/quiz/submit
          → POST /api/ai/update-mastery
          → 记录结果到本地数组
  → 全部完成 → 过渡到阶段 3：ReviewSummary
  → 用户点击「返回」→ 重新 GET reviews/due，回到阶段 1
```

---

## 5. API

### 5.1 已有 API（无需改动）

| 端点 | 用途 |
|------|------|
| `GET /api/learning/reviews/due` | 获取到期复习记录 |
| `POST /api/quiz/submit` | 提交答案 |
| `POST /api/ai/update-mastery` | 更新 SM-2 掌握度 |
| `GET /api/quiz/kp/:id` | 获取知识点已有题目 |
| `POST /api/ai/generate-quiz` | AI 生成新题 |

### 5.2 新增/修改 API

**`GET /api/learning/reviews/due?with_kp=true`**

修改现有端点：新增可选参数 `with_kp`。为 `true` 时，返回聚合数据：

```json
[
  {
    "mastery": {
      "id": "...",
      "kp_id": "...",
      "ease_factor": 2.5,
      "interval_days": 1,
      "repetitions": 1,
      "next_review_at": "2025-08-04T...",
      "last_reviewed_at": "2025-08-03T..."
    },
    "knowledge_point": {
      "id": "...",
      "title": "Rust 所有权",
      "summary": "...",
      "content": "...",
      "tags": ["rust"],
      "source_ids": [...],
      "created_at": "..."
    }
  }
]
```

### 5.3 后端改动

`src-tauri/src/api/learning.rs`：解析 `with_kp` query param，传递到 repo 层。

`src-tauri/src/repo/learning.rs`：新增 `get_due_reviews_with_kp()` 函数，JOIN `mastery_records` 和 `knowledge_points` 表。

---

## 6. 新增/修改类型定义

```typescript
// ReviewItem: 到期复习条目的聚合数据（API 返回）
export interface ReviewItem {
  mastery: MasteryRecord;
  knowledge_point: KnowledgePoint;
}

// ReviewResult: 单次复习答题的结果（前端本地累积，不持久化）
export interface ReviewResult {
  kp_id: string;
  kp_title: string;
  is_correct: boolean;
  next_review_at: string;  // 来自 update-mastery 响应的 SM-2 输出
}
```

## 7. 前端组件结构

### 7.1 文件清单

| 文件 | 操作 | 说明 |
|------|------|------|
| `src/types.ts` | 修改 | 新增 `ReviewItem` 类型 |
| `src/api/client.ts` | 修改 | 新增 `learning.getDueReviewsWithKp()` |
| `src/components/Layout.tsx` | 修改 | View 类型扩展 |
| `src/components/Content.tsx` | 修改 | review 分发 |
| `src/components/Sidebar.tsx` | 修改 | 复习 tab + 红点数字 |
| `src/components/Sidebar.css` | 修改 | 复习 tab 样式 |
| `src/components/Content/ReviewView.tsx` | 新增 | 三阶段状态机容器 |
| `src/components/Content/ReviewView.css` | 新增 | 复习视图样式 |
| `src/components/Content/ReviewList.tsx` | 新增 | 待复习列表 |
| `src/components/Content/ReviewSession.tsx` | 新增 | 答题循环 |
| `src/components/Content/ReviewSummary.tsx` | 新增 | 结果总结 |

### 7.2 组件职责

| 组件 | 职责 | 输入 | 输出 |
|------|------|------|------|
| `ReviewView` | 三阶段状态机，管理选中 KP 集合和结果数组 | 无 | 无 |
| `ReviewList` | 展示到期列表，勾选操作 | `items: ReviewItem[]` | `onStart(ids[])` |
| `ReviewSession` | 逐个出题，调用 API，累积结果 | `kpIds: string[]` | `onComplete(results[])` |
| `ReviewSummary` | 展示对错统计 + 下次复习时间 | `results: ReviewResult[]` | `onBack()` |

### 7.3 状态管理

不引入新 hook，`ReviewView` 内用 `useState` 管理局部状态：

```typescript
type Phase = "list" | "session" | "summary";
```

---

## 8. 题目去重策略

知识点可能有多道已有题目，复习时避免重复出题：

- `ReviewSession` 维护一个 `usedQuestionIds: Set<string>`，本地追踪已用题目
- 每次从 `GET /api/quiz/kp/:id` 返回的题目中筛选 `id not in usedQuestionIds` 的题
- 若无未用题目，调用 `POST /api/ai/generate-quiz` 生成新题
- 若 AI 生成也失败，跳过该知识点（错误处理覆盖）

## 9. 错误处理

| 场景 | 处理 |
|------|------|
| 到期列表为空 | 显示「暂无待复习知识点 🎉」 |
| 加载到期列表失败 | 显示错误提示 + 重试按钮 |
| AI 生成题目失败 | 提示「出题失败，请稍后重试」，跳过该知识点继续下一个 |
| 提交答案失败 | 提示「提交失败」，允许重试当前题 |
| 整个复习会话中网络断连 | Toast 提示，保留已完成的答案记录，允许从断点继续 |

---

## 10. 非功能需求

- **性能**：到期列表通常不会超过数十个知识点，无分页需求
- **离线**：若已有题库充足（非 AI 生成），核心答题流程可离线运行
- **状态保持**：ReviewSession 中的结果累积在组件 state 中，切换路由会丢失（可接受——下次回来重做）
