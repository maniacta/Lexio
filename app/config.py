"""全局配置 - 基于环境变量"""

from pydantic_settings import BaseSettings
from typing import List
from functools import lru_cache


class Settings(BaseSettings):
    """应用配置"""

    # 环境
    ENVIRONMENT: str = "development"
    HOST: str = "0.0.0.0"
    PORT: int = 8000

    # CORS - 使用字符串解析，逗号分隔
    ALLOWED_ORIGINS: str = "http://localhost:3000,http://localhost:5173"

    # 日志
    LOG_LEVEL: str = "DEBUG"

    # OpenAI (Phase M3)
    OPENAI_API_KEY: str = ""
    OPENAI_MODEL: str = "gpt-4o-mini"

    # 数据库 (Phase M2)
    REDIS_URL: str = "redis://localhost:6379/0"
    POSTGRES_DSN: str = ""

    # 应用配置
    MAX_INJECTION: int = 3
    DEFAULT_MODE: str = "basic"

    @property
    def allowed_origins_list(self) -> List[str]:
        """获取 CORS 原始列表"""
        return [origin.strip() for origin in self.ALLOWED_ORIGINS.split(",") if origin.strip()]

    class Config:
        env_file = ".env"
        env_file_encoding = "utf-8"
        case_sensitive = False


@lru_cache()
def get_settings() -> Settings:
    """获取配置实例（缓存）"""
    return Settings()


# 全局配置实例
settings = get_settings()