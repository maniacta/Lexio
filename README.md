# Lexio

```plain
Lexio/
├── pyproject.toml # uv / 项目依赖配置
├── uv.lock # 锁定依赖
├── README.md
├── .env.example # 环境变量模板
├── .gitignore
│
├── app/ # 主应用目录
│ ├── main.py # FastAPI 入口
│ ├── config.py # 全局配置
│ ├── logging.py # 日志配置
│ │
│ ├── api/ # API 层（纯 I/O）
│ │ ├── __init__.py
│ │ ├── deps.py # 依赖注入
│ │ ├── router.py # 路由聚合
│ │ └── v1/
│ │ ├── __init__.py
│ │ ├── enhance.py # 增强接口
│ │ ├── preview.py # 预览接口
│ │ └── profile.py # 用户语言模型接口
│ │
│ ├── core/ # 核心业务逻辑（最重要）
│ │ ├── __init__.py
│ │ ├── pipeline.py # 内容 → 增强 主流程
│ │ ├── boundary.py # 边界评估引擎
│ │ ├── stress.py # 压力模型
│ │ └── strategy.py # 增强策略选择
│ │
│ ├── parsing/ # 内容解析层
│ │ ├── __init__.py
│ │ ├── sentence.py # 分句
│ │ ├── semantic.py # 语义单元识别
│ │ └── difficulty.py # 语言难度评估
│ │
│ ├── profile/ # 用户语言模型
│ │ ├── __init__.py
│ │ ├── schema.py # 数据结构
│ │ ├── updater.py # 行为驱动更新
│ │ └── repository.py # Redis / DB 存取
│ │
│ ├── llm/ # LLM 适配层（可替换）
│ │ ├── __init__.py
│ │ ├── base.py # 抽象接口
│ │ ├── openai.py # OpenAI 实现
│ │ └── local.py # 本地模型
│ │
│ ├── storage/ # 持久化层
│ │ ├── __init__.py
│ │ ├── redis.py
│ │ └── postgres.py
│ │
│ ├── schemas/ # Pydantic 模型（API 边界）
│ │ ├── __init__.py
│ │ ├── request.py
│ │ └── response.py
│ │
│ └── utils/ # 工具
│ ├── __init__.py
│ └── metrics.py # 指标埋点
│
├── scripts/ # 运维 / 本地工具
│ ├── init_db.py
│ └── dev_server.sh
│
└── tests/
├── __init__.py
├── test_boundary.py
└── test_pipeline.py
```

## 拆解清单（Backend）

> 使用方式建议：
>
> * 每一条 = 1 个 GitHub Issue
> * Issue 标题可直接复制
> * Label 建议：`phase/*` `area/*` `priority/*`

---

## 🧱 Phase M0｜工程初始化（Week 0）

### Issue M0-01｜初始化 uv + FastAPI 项目骨架

* **Labels**: phase/M0, area/infra, priority/P0
* **目标**：项目可启动
* **任务**：

  * 初始化 uv 项目
  * 固定 Python 3.11
  * FastAPI hello world
* **验收标准**：

  * `uv run uvicorn app.main:app` 可访问

---

### Issue M0-02｜创建标准后端目录结构

* **Labels**: phase/M0, area/infra, priority/P0
* **任务**：

  * 创建 app/core/parsing/profile/llm 等目录
  * 保证 core 不依赖 FastAPI
* **验收标准**：

  * import 方向清晰，无循环依赖

---

### Issue M0-03｜配置统一 config / env / logging

* **Labels**: phase/M0, area/infra, priority/P1
* **任务**：

  * .env.example
  * config.py
  * logging.py
* **验收标准**：

  * 可按环境切换配置

---

## 🔁 Phase M1｜最小增强闭环（Week 1–2）

### Issue M1-01｜实现 core.pipeline 最小可运行版本

* **Labels**: phase/M1, area/core, priority/P0
* **任务**：

  * 定义 pipeline 接口
  * 串联 parsing → strategy → output
* **验收标准**：

  * 输入文本必有输出

---

### Issue M1-02｜实现 parsing.v0（分句 + mock 语义）

* **Labels**: phase/M1, area/parsing, priority/P0
* **任务**：

  * 中文分句
  * mock semantic unit
* **验收标准**：

  * 分句稳定

---

### Issue M1-03｜实现固定规则增强策略（无 LLM）

* **Labels**: phase/M1, area/core, priority/P0
* **任务**：

  * 关键词英文替换（白名单）
* **验收标准**：

  * 输出可预测

---

### Issue M1-04｜实现 /api/v1/enhance 接口

* **Labels**: phase/M1, area/api, priority/P0
* **任务**：

  * POST enhance
  * 支持 mode
* **验收标准**：

  * curl 请求成功

---

### Issue M1-05｜实现原文回退机制

* **Labels**: phase/M1, area/core, priority/P1
* **任务**：

  * original / enhanced 切换
* **验收标准**：

  * 无异常回退

---

## 📏 Phase M2｜边界 & 压力模型 v0（Week 3–4）

### Issue M2-01｜定义用户语言模型 schema

* **Labels**: phase/M2, area/profile, priority/P0
* **任务**：

  * known / unknown vocab
  * stress_tolerance

---

### Issue M2-02｜实现压力模型 v0

* **Labels**: phase/M2, area/core, priority/P0
* **任务**：

  * 单词压力计算
  * 累计 stress

---

### Issue M2-03｜实现边界评估与策略降级

* **Labels**: phase/M2, area/core, priority/P0
* **任务**：

  * max_injection
  * downgrade logic

---

### Issue M2-04｜行为埋点与 profile 更新

* **Labels**: phase/M2, area/profile, priority/P1
* **任务**：

  * 停留
  * 回退
  * 模式切换

---

## 🤖 Phase M3｜LLM 接入（Week 5–6）

### Issue M3-01｜实现 llm 抽象接口

* **Labels**: phase/M3, area/llm, priority/P0

---

### Issue M3-02｜接入 OpenAI / 本地模型

* **Labels**: phase/M3, area/llm, priority/P0

---

### Issue M3-03｜实现 i+1 增强策略

* **Labels**: phase/M3, area/core, priority/P0

---

## 🔒 Phase M4｜稳定性与保护机制（Week 7–8）

### Issue M4-01｜实现 stress spike 自动降级

* **Labels**: phase/M4, area/core, priority/P0

---

### Issue M4-02｜LLM 熔断与 fallback

* **Labels**: phase/M4, area/llm, priority/P0

---

### Issue M4-03｜性能与成本优化

* **Labels**: phase/M4, area/infra, priority/P1

---

## 🎬 Phase M5｜Demo & 灰度（Week 9–10）

### Issue M5-01｜Demo 场景数据准备

* **Labels**: phase/M5, area/demo, priority/P0

---

### Issue M5-02｜基础数据看板

* **Labels**: phase/M5, area/metrics, priority/P1

---

### Issue M5-03｜技术债与 v2 清单整理

* **Labels**: phase/M5, area/meta, priority/P1

---

## ✅ 使用建议（非常重要）

* **同一时间最多进行 3–4 个 Issue**
* M1/M2 不完成，绝不进入 M3
* 每个 Phase 必须有 Demo 或可验证结果


