# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

Lexio 是一个基于 FastAPI 的语言学习增强服务。它通过分析用户输入的文本内容，结合用户语言模型（词汇量、耐压能力），智能地增强文本以促进语言学习（i+1 理论）。

**当前状态**: 项目骨架已初始化，所有源文件为空，处于开发初期阶段（Phase M0）。

## 项目架构

```
Lexio/
├── app/                          # 主应用目录
│   ├── main.py                   # FastAPI 入口点
│   ├── config.py                 # 全局配置
│   ├── logging.py                # 日志配置
│   │
│   ├── api/                      # API 层（纯 I/O）
│   │   ├── deps.py               # 依赖注入
│   │   ├── router.py             # 路由聚合
│   │   └── v1/                   # API v1 版本
│   │       ├── enhance.py        # 增强接口
│   │       ├── preview.py        # 预览接口
│   │       └── profile.py        # 用户语言模型接口
│   │
│   ├── core/                     # 核心业务逻辑（最重要）
│   │   ├── pipeline.py           # 内容 → 增强 主流程
│   │   ├── boundary.py           # 边界评估引擎
│   │   ├── stress.py             # 压力模型
│   │   └── strategy.py           # 增强策略选择
│   │
│   ├── parsing/                  # 内容解析层
│   │   ├── sentence.py           # 分句
│   │   ├── semantic.py           # 语义单元识别
│   │   └── difficulty.py         # 语言难度评估
│   │
│   ├── profile/                  # 用户语言模型
│   │   ├── schema.py             # 数据结构
│   │   ├── updater.py            # 行为驱动更新
│   │   └── repository.py         # Redis / DB 存取
│   │
│   ├── llm/                      # LLM 适配层（可替换）
│   │   ├── base.py               # 抽象接口
│   │   ├── openai.py             # OpenAI 实现
│   │   └── local.py              # 本地模型
│   │
│   ├── storage/                  # 持久化层
│   │   ├── redis.py
│   │   └── postgres.py
│   │
│   ├── schemas/                  # Pydantic 模型（API 边界）
│   │   ├── request.py
│   │   └── response.py
│   │
│   └── utils/                    # 工具
│       └── metrics.py            # 指标埋点
│
├── scripts/                      # 运维 / 本地工具
│   ├── init_db.py
│   └── dev_server.sh
│
└── tests/                        # 测试
    ├── test_pipeline.py
    └── test_boundary.py
```

## 核心设计原则

1. **分层架构**: API → Core → Parsing/Profile/LLM → Storage
2. **核心不依赖 FastAPI**: `app/core/` 目录下的代码应保持纯 Python 逻辑，不依赖 FastAPI
3. **边界清晰**: 使用 Pydantic schemas 定义 API 边界
4. **可替换性**: LLM 层设计为抽象接口，支持 OpenAI/本地模型切换

## 开发命令

### 环境管理
```bash
# 激活虚拟环境（如果需要）
source .venv/bin/activate  # Linux/Mac
# .venv\Scripts\activate    # Windows

# 安装依赖
uv sync

# 添加新依赖
uv add <package>
```

### 运行服务
```bash
# 启动开发服务器
uvicorn app.main:app --reload --host 0.0.0.0 --port 8000

# 或使用脚本
./scripts/dev_server.sh
```

### 测试
```bash
# 运行所有测试
pytest

# 运行特定测试
pytest tests/test_pipeline.py

# 运行测试并显示覆盖率
pytest --cov=app --cov-report=html
```

### 代码质量
```bash
# 格式化代码（需要安装 ruff/black）
ruff format app/ tests/

# 类型检查（需要安装 mypy）
mypy app/
```

## 开发阶段说明

根据 README.md，项目分为以下阶段：

- **Phase M0**: 工程初始化（当前阶段）
  - M0-01: 初始化 uv + FastAPI 项目骨架
  - M0-02: 创建标准后端目录结构
  - M0-03: 配置统一 config / env / logging

- **Phase M1**: 最小增强闭环
  - 实现 core.pipeline 最小可运行版本
  - 实现 parsing.v0（分句 + mock 语义）
  - 实现固定规则增强策略（无 LLM）
  - 实现 /api/v1/enhance 接口

- **Phase M2**: 边界 & 压力模型 v0
  - 定义用户语言模型 schema
  - 实现压力模型 v0
  - 实现边界评估与策略降级
  - 行为埋点与 profile 更新

- **Phase M3**: LLM 接入
  - 实现 llm 抽象接口
  - 接入 OpenAI / 本地模型
  - 实现 i+1 增强策略

- **Phase M4**: 稳定性与保护机制
  - 实现 stress spike 自动降级
  - LLM 熔断与 fallback
  - 性能与成本优化

- **Phase M5**: Demo & 灰度
  - Demo 场景数据准备
  - 基础数据看板
  - 技术债与 v2 清单整理

## 重要注意事项

1. **先完成 Phase M1/M2 再进入 M3**: 不要过早引入 LLM
2. **每个 Phase 必须有可验证结果**: 必须有 Demo 或测试验证
3. **同一时间最多进行 3-4 个 Issue**: 避免过度并发
4. **import 方向**: API → Core → LLM/Storage，Core 不依赖 FastAPI

## 环境变量配置

参考 `.env.example`（当前为空），需要配置：
- 数据库连接（Redis/PostgreSQL）
- OpenAI API Key（Phase M3 开始需要）
- 日志级别
- 服务配置

## 测试策略

- 核心逻辑（core/）必须有单元测试
- API 层使用集成测试
- 边界情况必须覆盖
- 性能关键路径需要基准测试
