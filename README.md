# Lexio

本地优先的 AI 学习教练。把主题或资料交给它，它帮你研究、整理成知识点，再通过测验与间隔复习帮你真正掌握。

```
研究 → 整理 → 学习 → 测验 → 间隔复习
```

## 功能

- **主题研究**：对话发起学习主题，生成资料、知识点与学习计划
- **知识库**：管理知识点与资料，支持侧栏浏览与学习页阅读
- **测验**：按知识点生成题目并批改
- **复习**：基于 SM-2 的到期复习队列与会话流程
- **模型配置**：按厂商适配；当前已接入 [DeepSeek](https://api-docs.deepseek.com/zh-cn/)（`deepseek-v4-flash` / `deepseek-v4-pro`）
- **本地安全**：API Key AES-GCM 加密存储；本地 API 使用 Token 鉴权

## 技术栈

| 层级 | 技术 |
|------|------|
| 桌面壳 | Tauri v2 |
| 后端 | Rust · Axum · Tokio · SQLite |
| 前端 | React 19 · TypeScript · Vite 7 |
| AI | 厂商独立适配器（DeepSeek Chat Completions） |

默认窗口尺寸：`1280×720`。

## 环境要求

- [Rust](https://www.rust-lang.org/tools/install)（stable）
- [Node.js](https://nodejs.org/) >= 18
- **Windows**：安装 [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools)，勾选「使用 C++ 的桌面开发」
- **macOS**：`xcode-select --install`
- **Linux**：Tauri 系统依赖，例如：

```bash
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev \
  librsvg2-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev
```

### 国内镜像（可选）

npm：

```bash
npm config set registry https://registry.npmmirror.com
```

Cargo（`~/.cargo/config.toml`）：

```toml
[source.crates-io]
replace-with = "tuna"

[source.tuna]
registry = "sparse+https://mirrors.tuna.tsinghua.edu.cn/crates.io-index/"

[net]
git-fetch-with-cli = true
```

## 快速开始

```bash
npm install
```

### 桌面模式（推荐）

```bash
npm run tauri dev
```

启动后在 **设置 → 模型厂商** 填写 DeepSeek API Key，即可开始对话与研究。

### Web 模式（仅前端联调）

```bash
# 终端 1：后端
cargo run --manifest-path src-tauri/Cargo.toml --bin server

# 终端 2：前端
npm run dev
```

浏览器打开 `http://localhost:14200`。侧栏底部会显示运行模式（Desktop / Web）。

## 服务地址

| 服务 | 地址 |
|------|------|
| 前端（Vite） | `http://localhost:14200` |
| 后端（Axum） | `http://127.0.0.1:3001` |
| 健康检查 | `http://127.0.0.1:3001/api/health` |

前端通过 Vite 代理将 `/api/*` 转发到后端。

## 构建

```bash
npm run tauri build
```

产物在 `src-tauri/target/release/bundle/`。

## 项目结构

```
Lexio/
├── src/                      # React 前端
│   ├── api/                  # HTTP 客户端（含本地 Token）
│   ├── components/           # 布局、聊天、学习、复习、设置
│   └── hooks/                # 数据与交互 hooks
├── src-tauri/                # Tauri + Axum
│   └── src/
│       ├── ai/               # LLM 适配器、研究、出题
│       ├── api/              # HTTP 路由
│       ├── learning/         # SM-2 等学习逻辑
│       ├── repo/             # SQLite 仓储
│       ├── crypto.rs         # API Key 加密
│       └── server.rs         # 路由装配
└── docs/                     # 设计规格与实现计划
```

## 配置说明

1. 打开 **设置 → 模型厂商**，为 DeepSeek 保存 API Key（本地加密）。
2. 在官方模型目录中启用 `deepseek-v4-flash` / `deepseek-v4-pro`，并设默认模型。
3. 可选：在 **任务模型** 中为对话 / 测验指定不同模型。

数据与主密钥默认存放在应用本地数据目录（SQLite + `.lexio-master.key`）。请勿把密钥文件提交到版本库。

## 许可证

私有项目，未声明开源许可。
