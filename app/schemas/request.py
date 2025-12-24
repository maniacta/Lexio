"""请求模型"""

from pydantic import BaseModel, Field
from typing import Optional


class EnhanceRequest(BaseModel):
    """增强请求"""

    content: str = Field(..., description="需要增强的文本内容")
    mode: str = Field(default="basic", description="增强模式: basic, i_plus_1")
    user_id: Optional[str] = Field(None, description="用户ID（用于获取 Profile）")


class PreviewRequest(BaseModel):
    """预览请求"""

    content: str
    mode: str = "basic"


class ProfileUpdateRequest(BaseModel):
    """Profile 更新请求"""

    user_id: str
    known_words: list = []
    unknown_words: list = []
    stress_events: list = []