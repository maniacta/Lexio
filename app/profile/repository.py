"""Profile 存储 - Redis/DB 存取 - Phase M2 实现"""

from typing import Optional
from app.profile.schema import UserLanguageProfile


class ProfileRepository:
    """Profile 仓库（占位实现）"""

    def __init__(self):
        # Phase M2: 实现 Redis/PostgreSQL
        self._storage = {}

    async def get(self, user_id: str) -> Optional[UserLanguageProfile]:
        """获取用户 Profile"""
        data = self._storage.get(user_id)
        if data:
            return UserLanguageProfile(**data)
        return None

    async def save(self, profile: UserLanguageProfile) -> bool:
        """保存用户 Profile"""
        self._storage[profile.user_id] = profile.model_dump()
        return True

    async def delete(self, user_id: str) -> bool:
        """删除用户 Profile"""
        self._storage.pop(user_id, None)
        return True


# 全局实例
profile_repo = ProfileRepository()