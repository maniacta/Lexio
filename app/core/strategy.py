"""增强策略选择 - Phase M1/M3 实现"""

from typing import Dict, Any


def select_strategy(mode: str, profile: Dict[str, Any] = None) -> Dict[str, Any]:
    """
    选择增强策略（占位实现）

    Args:
        mode: 模式
        profile: 用户语言模型（Phase M2+）

    Returns:
        策略配置
    """
    strategies = {
        "basic": {
            "name": "basic",
            "description": "基础增强 - 关键词替换",
            "requires_llm": False,
            "max_injection": 1
        },
        "i_plus_1": {
            "name": "i_plus_1",
            "description": "i+1 增强 - 智能推荐",
            "requires_llm": True,
            "max_injection": 3
        }
    }

    return strategies.get(mode, strategies["basic"])