"""响应模型"""

from pydantic import BaseModel, Field
from typing import Dict, Any, Optional
from datetime import datetime


class EnhanceResponse(BaseModel):
    """增强响应"""

    original: str
    enhanced: str
    mode: str
    metadata: Dict[str, Any] = Field(default_factory=dict)
    timestamp: datetime = Field(default_factory=datetime.utcnow)


class PreviewResponse(BaseModel):
    """预览响应"""

    preview: str
    changes: list
    mode: str


class ProfileResponse(BaseModel):
    """Profile 响应"""

    user_id: str
    known_vocab_count: int
    unknown_vocab_count: int
    stress_tolerance: float
    current_level: str
    updated_at: datetime


class HealthResponse(BaseModel):
    """健康检查响应"""

    status: str
    version: str
    timestamp: datetime