"""日志配置"""

import logging
import sys
from app.config import settings


def setup_logging():
    """设置日志配置"""

    # 日志格式
    formatter = logging.Formatter(
        fmt="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
        datefmt="%Y-%m-%d %H:%M:%S"
    )

    # 控制台处理器
    console_handler = logging.StreamHandler(sys.stdout)
    console_handler.setFormatter(formatter)
    console_handler.setLevel(settings.LOG_LEVEL)

    # 根日志记录器
    root_logger = logging.getLogger()
    root_logger.setLevel(settings.LOG_LEVEL)

    # 清除已有处理器
    root_logger.handlers.clear()
    root_logger.addHandler(console_handler)

    # 应用特定日志记录器
    app_logger = logging.getLogger("app")
    app_logger.setLevel(settings.LOG_LEVEL)

    # 数据库日志记录器
    db_logger = logging.getLogger("sqlalchemy")
    db_logger.setLevel(logging.WARNING)

    print(f"✅ 日志配置完成 - 级别: {settings.LOG_LEVEL}")


def get_logger(name: str):
    """获取日志记录器"""
    return logging.getLogger(f"app.{name}")


# 快捷访问
logger = get_logger(__name__)