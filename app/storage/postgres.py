"""PostgreSQL 存储 - Phase M2 实现"""

from typing import Optional, Any


class PostgresStorage:
    """PostgreSQL 存储（占位实现）"""

    def __init__(self, dsn: str):
        self.dsn = dsn
        self._conn = None  # Phase M2: 实际连接

    async def query(self, sql: str, params: tuple = None) -> list:
        """执行查询"""
        return []

    async def execute(self, sql: str, params: tuple = None) -> bool:
        """执行命令"""
        return True