"""LLM 抽象接口 - Phase M3 实现"""

from abc import ABC, abstractmethod
from typing import Dict, Any


class LLMInterface(ABC):
    """LLM 抽象接口"""

    @abstractmethod
    async def enhance_text(self, text: str, context: Dict[str, Any]) -> Dict[str, Any]:
        """增强文本"""
        pass

    @abstractmethod
    async def suggest_replacements(self, word: str, context: Dict[str, Any]) -> list:
        """建议替换词"""
        pass


class MockLLM(LLMInterface):
    """Mock LLM 用于测试"""

    async def enhance_text(self, text: str, context: Dict[str, Any]) -> Dict[str, Any]:
        return {"enhanced": text, "provider": "mock"}

    async def suggest_replacements(self, word: str, context: Dict[str, Any]) -> list:
        return [word]