"""用户语言模型数据结构 - Phase M2 实现"""

from typing import Set, Dict
from pydantic import BaseModel, Field
from datetime import datetime


class UserLanguageProfile(BaseModel):
    """用户语言模型"""

    user_id: str
    known_vocab: Set[str] = Field(default_factory=set)
    unknown_vocab: Set[str] = Field(default_factory=set)
    stress_tolerance: float = 0.5  # 0.0-1.0
    current_level: str = "intermediate"
    updated_at: datetime = Field(default_factory=datetime.utcnow)

    class Config:
        json_encoders = {
            set: lambda v: list(v),
            datetime: lambda v: v.isoformat()
        }


class ProfileUpdate(BaseModel):
    """Profile 更新数据"""

    known_words: list = []
    unknown_words: list = []
    stress_events: list = []