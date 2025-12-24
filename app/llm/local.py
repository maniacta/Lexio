"""本地 LLM 实现 - Phase M3 实现"""

from typing import Dict, Any
from app.llm.base import LLMInterface


class LocalLLM(LLMInterface):
    """本地 LLM 实现（占位）"""

    def __init__(self, model_path: str):
        self.model_path = model_path

    async def enhance_text(self, text: str, context: Dict[str, Any]) -> Dict[str, Any]:
        # Phase M3 实现
        return {"enhanced": text, "provider": "local"}

    async def suggest_replacements(self, word: str, context: Dict[str, Any]) -> list:
        # Phase M3 实现
        return [word]