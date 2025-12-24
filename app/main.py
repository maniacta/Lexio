"""Lexio 主应用入口"""

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from app.api.router import api_router
from app.config import settings
from app.logging import setup_logging

# 设置日志
setup_logging()

# 创建 FastAPI 应用
app = FastAPI(
    title="Lexio",
    description="语言学习增强服务 - 通过智能增强促进语言习得 (i+1)",
    version="0.1.0",
)

# CORS 中间件
app.add_middleware(
    CORSMiddleware,
    allow_origins=settings.allowed_origins_list,
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# 注册 API 路由
app.include_router(api_router, prefix="/api")

@app.get("/")
async def root():
    """健康检查接口"""
    return {
        "service": "Lexio",
        "version": "0.1.0",
        "status": "running",
        "docs": "/docs",
    }


@app.get("/health")
async def health():
    """健康检查"""
    return {"status": "healthy"}