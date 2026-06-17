# Lexio

基于 Tauri v2 + React + Axum 的桌面应用。

## 环境要求

- [Rust](https://www.rust-lang.org/tools/install)（stable，MSVC 工具链）
- [Node.js](https://nodejs.org/) >= 18
- **Windows**：需安装 [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)，勾选「使用 C++ 的桌面开发」工作负载

## 安装

```bash
# 安装前端依赖
npm install
```

## 开发

```bash
# 启动完整桌面应用（Tauri + React 开发服务器 + Axum 后端）
npm run tauri dev
```

- 前端开发服务器：`http://localhost:1420`
- Axum API：`http://127.0.0.1:3001`
- 健康检查：`http://127.0.0.1:3001/api/health`

## 构建

```bash
# 生产构建
npm run tauri build
```

## 技术栈

| 层级 | 技术 |
|------|------|
| 桌面框架 | Tauri v2 |
| 后端 | Axum 0.8 + Tokio |
| 前端 | React 19 + TypeScript + Vite 7 |
| 包管理 | npm |
