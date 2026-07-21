# Lexio: AI 知识学习 Agent 设计规格

> 2026-07-21 | 状态: 设计中

## 1. 产品定位

**Lexio**（源自 "lexicon"——词汇、知识典）是一个以**学习教练为核心的 AI 知识 agent**桌面应用。

一句话描述：用户把资料放进来，告诉它想学什么，它帮你研究、整理、教授、测验，直到你真正掌握。

### 核心循环

```
搜集 → 整理 → 学习 → 测验 → 追踪
  ↑                              ↓
  └──────── 复习（间隔重复）←─────┘
```

### 三大能力塔

- **学习教练**（核心）：测验生成/批改、SM-2 间隔复习、掌握度追踪
- **自动整理者**（辅助）：资料归类、打标签、概念关联
- **研究助手**（辅助）：联网搜索、提炼摘要、生成结构化报告

## 2. 整体架构

```
┌─────────────────────────────────────────┐
│  React 前端 (Tauri WebView / 浏览器)     │
│  ┌─────────┐ ┌──────────┐ ┌──────────┐ │
│  │  对话区  │ │  文档区   │ │  知识图谱  │ │
│  │(聊天交互)│ │(笔记/报告)│ │(可视化关联)│ │
│  └─────────┘ └──────────┘ └──────────┘ │
└───────────────┬─────────────────────────┘
                │ HTTP / Tauri IPC
┌───────────────▼─────────────────────────┐
│  Rust 后端 (Axum + Tauri Commands)       │
│  ┌─────────┐ ┌──────────┐ ┌──────────┐ │
│  │ AI 网关  │ │ 知识引擎  │ │ 学习引擎  │ │
│  │(模型调用)│ │(存储/检索)│ │(测验/复习)│ │
│  └────┬────┘ └────┬─────┘ └──────────┘ │
└───────┼───────────┼─────────────────────┘
        │           │
   ┌────▼────┐ ┌───▼──────┐
   │ LLM API │ │ SQLite   │
   │(兼容    │ │(FTS5     │
   │ OpenAI/ │ │ 全文检索) │
   │ DeepSeek)│ └──────────┘
   └─────────┘
```

### 核心模块

| 模块 | 职责 | 技术 |
|------|------|------|
| AI 网关 | 统一管理 LLM 调用（对话、总结、出题、搜索提炼）| reqwest + streaming |
| 知识引擎 | 存储笔记/文件、全文检索、标签管理、概念关联 | SQLite + FTS5 |
| 学习引擎 | 知识点管理、测验生成/批改、SM-2 间隔复习算法 | Rust 纯逻辑 |

## 3. 核心数据模型

```
Source (资料)
├── id: uuid
├── title: string
├── type: "url" | "text" | "file"
├── content: text
├── tags: string[]
├── origin: "user" | "ai_search"
├── source_url: string?          (AI 搜索来的必有)
├── hidden: bool = false         (暂时屏蔽，不参与后续处理)
└── created_at: datetime

KnowledgePoint (知识点)           ← 核心实体
├── id: uuid
├── title: string
├── summary: string              (一句话概括)
├── content: text                (详细说明)
├── tags: string[]
├── source_ids: uuid[]
└── created_at: datetime

Relation (知识点关联)
├── id: uuid
├── from_kp_id: uuid
├── to_kp_id: uuid
├── relation_type: "prerequisite" | "related" | "extension"
└── created_at: datetime

QuizQuestion (测验题)
├── id: uuid
├── kp_id: uuid
├── type: "multiple_choice" | "fill_blank" | "analysis"
├── question: text
├── options: string[]?           (选择题才有)
├── answer: string
└── explanation: string

QuizAttempt (作答记录)
├── id: uuid
├── question_id: uuid
├── user_answer: string
├── is_correct: bool
└── attempted_at: datetime

MasteryRecord (掌握度追踪，SM-2 算法数据)
├── id: uuid
├── kp_id: uuid (unique)
├── ease_factor: float           (初始 2.5)
├── interval_days: int           (复习间隔天数)
├── repetitions: int             (连续正确次数)
├── next_review_at: datetime
└── last_reviewed_at: datetime

LearningPlan (学习计划)
├── id: uuid
├── title: string
├── goal: text
├── kp_ids: uuid[]
├── status: "active" | "completed" | "paused"
└── created_at: datetime
```

### 数据流

```
用户提交资料 ──→ Source ──(AI 提取)──→ KnowledgePoint[] ──→ QuizQuestion[]
                                               │
用户提主题 ──(AI 搜索+提炼)────→─────────────────┘
                                               │
                                         Relation (关联图谱)
                                               │
                                         LearningPlan ──→ MasteryRecord (追踪)
```

## 4. 核心用户流程

### 流程 1：新建学习主题

1. 用户输入"我想系统学习 Rust 的所有权和借用"
2. AI 判断为新学习主题，启动研究+规划流程
3. **研究阶段**：AI 搜索网络，筛选高质量资料，存为 Source (origin: ai_search)，每份资料展示给用户预览
4. 用户可隐藏不需要的资料 (hidden=true)
5. **整理阶段**：AI 从资料中提取知识点卡片，建立前置/关联关系，生成学习计划（如：所有权 → 引用 → 借用 → 生命周期）
6. **对话确认**：AI 展示计划，用户可调整顺序，确认后开始学习

### 流程 2：学习与测验循环

1. AI 以对话形式讲解知识点（markdown + 代码块），用户可随时追问
2. 用户表示"懂了"后，AI 出题测验（从题库抽取或即时生成）
3. AI 批改答案，解释正确/错误原因
4. 记录 QuizAttempt + 更新 MasteryRecord（SM-2 算法）

### 流程 3：日常复习提醒

1. 系统基于 MasteryRecord.next_review_at 检查到期知识点
2. 主动提醒："今天有 3 个知识点到期复习"
3. 用户选择复习 → AI 针对性出题（不重复旧题）
4. 根据结果更新 MasteryRecord，重新安排下次复习

### 流程 4：随手扔资料（轻量入口）

1. 用户粘贴链接/文字/文件
2. AI 识别内容并给出选项：存着以后看 / 提取知识点 / 和当前学习主题关联
3. 选提取 → AI 提取知识点 → 自动打标签 → 关联已有知识图谱

## 5. MVP 范围

### MVP（第 1 版）——验证核心闭环

| 能力 | 包含 | 不包含 |
|------|------|--------|
| 学习教练 | 新建主题→AI 搜资料→提取知识点→出题测验→SM-2 复习提醒 | 复杂的进度可视化 |
| 知识管理 | Source 存取、标签、搜索、隐藏 | 知识图谱可视化 |
| 研究助手 | 给主题→AI 搜索→生成结构化报告 | 多轮深度研究 |
| 界面 | 对话式为主，侧栏管理资料和知识点 | 知识图谱页面、文档编辑器 |

**MVP 验收标准**：用户说"我想学 X"，AI 搜资料→整理知识点→出题→批改→安排下次复习——全流程跑通。

### 第 2 版

- 知识图谱可视化（D3.js / vis.js）
- 学习进度仪表盘
- 多轮深度研究（分步搜索、交叉验证）
- 批量导入（PDF、Markdown 文件）

### 第 3 版

- 多人协作（共享知识库）
- 语音交互
- 自定义 LLM 模型切换
- 导出（Anki / Markdown）

## 6. 技术选型

| 层级 | 技术 | 说明 |
|------|------|------|
| 桌面框架 | Tauri v2 | 沿用现项目 |
| 后端 | Axum 0.8 + Tokio | 沿用现项目 |
| 前端 | React 19 + TypeScript + Vite 7 | 沿用现项目 |
| 数据库 | SQLite (rusqlite) + FTS5 | 本地存储，全文检索 |
| LLM 调用 | reqwest + SSE streaming | 兼容 OpenAI/DeepSeek |
| 间隔复习 | SM-2 算法 | Rust 实现，轻量有效 |
| 网络搜索 | tavily / searxng / duckduckgo | LLM 搜索外部资料 |
| 前端 Markdown | react-markdown + rehype | 渲染 AI 回复 |
| 知识图谱 (v2) | D3.js 或 vis.js | v2 引入 |

## 7. 非功能需求

- **离线可用**：核心学习+测验功能不依赖网络，知识库和 SM-2 算法本地运行。AI 搜索和 LLM 调用需要网络。
- **数据隐私**：所有用户资料和知识库存储在本地 SQLite，不上传。LLM API 只发送必要的 prompt 上下文。
- **增量加载**：知识点和资料列表支持分页/懒加载，不一次性加载全部。
- **错误处理**：LLM 调用失败时有降级提示，不丢失用户已输入的上下文。
