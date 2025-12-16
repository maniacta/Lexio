```
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