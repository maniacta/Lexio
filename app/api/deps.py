"""依赖注入 - FastAPI 依赖项定义"""

from typing import Generator


def get_db() -> Generator:
    """数据库连接依赖（占位，Phase M2 实现）"""
    yield None


def get_redis() -> Generator:
    """Redis 连接依赖（占位，Phase M2 实现）"""
    yield None