# Lexio

Desktop application built with Tauri v2, React, and Axum.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (stable, MSVC toolchain)
- [Node.js](https://nodejs.org/) >= 18
- **Windows**: [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with "Desktop development with C++" workload

## Install

```bash
# Install frontend dependencies
npm install
```

## Development

```bash
# Start full desktop app (Tauri + React dev server + Axum backend)
npm run tauri dev
```

- Frontend dev server: `http://localhost:1420`
- Axum API: `http://127.0.0.1:3001`
- Health check: `http://127.0.0.1:3001/api/health`

## Build

```bash
# Build for production
npm run tauri build
```

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop | Tauri v2 |
| Backend | Axum 0.8 + Tokio |
| Frontend | React 19 + TypeScript + Vite 7 |
| Package Manager | npm |
