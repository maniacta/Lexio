# Lexio

基于 Tauri v2 + React + Axum 的桌面应用，支持**双端运行**（Web 模式 + Tauri 桌面客户端）。

## 环境要求

- [Rust](https://www.rust-lang.org/tools/install)（stable）
- [Node.js](https://nodejs.org/) >= 18
- **Linux**：需安装 Tauri 系统依赖
  ```bash
  sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev \
    librsvg2-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev
  ```
- **Windows**：需安装 [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools)，勾选「使用 C++ 的桌面开发」工作负载
- **macOS**：Xcode Command Line Tools（`xcode-select --install`）

### Cargo 镜像（中国大陆用户）

```bash
mkdir -p ~/.cargo
cat > ~/.cargo/config.toml << 'EOF'
[source.crates-io]
replace-with = 'tuna'

[source.tuna]
registry = "sparse+https://mirrors.tuna.tsinghua.edu.cn/crates.io-index/"

[net]
git-fetch-with-cli = true
EOF
```

## 安装

```bash
npm install
```

## 开发

### Web 模式（云服务器 / 纯前端开发）

无需 Tauri 环境，直接在浏览器中运行：

```bash
npm run dev
```

访问 `http://localhost:14200`，侧栏底部显示 **Web** 标签。

### 桌面模式（本地开发）

启动完整 Tauri 桌面应用（React + Axum 后端）：

```bash
npm run tauri dev
```

侧栏底部显示 **Desktop** 标签。

## 服务地址

| 服务 | 地址 |
|------|------|
| 前端 (Vite) | `http://localhost:14200` |
| 后端 (Axum) | `http://127.0.0.1:3001` |
| 健康检查 | `http://127.0.0.1:3001/api/health` |

## 项目结构

```
Lexio/
├── index.html              # 入口 HTML
├── src/                    # 前端源码
│   ├── main.tsx            # React 入口
│   ├── App.tsx             # 根组件
│   ├── App.css             # 全局样式 & 设计令牌
│   ├── components/         # UI 组件
│   │   ├── Layout.tsx      # 页面布局（侧栏 + 内容）
│   │   ├── Sidebar.tsx     # 侧栏导航
│   │   └── Content.tsx     # 主内容区
│   └── utils/
│       └── tauri.ts        # 环境检测（Web / Desktop）
├── src-tauri/              # Rust 后端 (Tauri + Axum)
│   ├── src/
│   │   ├── main.rs         # 程序入口
│   │   └── lib.rs          # Tauri 命令 & Axum 路由
│   ├── Cargo.toml
│   └── tauri.conf.json
├── vite.config.ts
└── tsconfig.json
```

## 构建

```bash
npm run tauri build
```

## 技术栈

| 层级 | 技术 |
|------|------|
| 桌面框架 | Tauri v2 |
| 后端 | Axum 0.8 + Tokio |
| 前端 | React 19 + TypeScript + Vite 7 |
| 包管理 | npm |
