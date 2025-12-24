"""语义单元识别 - Phase M1 实现"""

from typing import List, Dict


def identify_semantic_units(text: str) -> List[Dict]:
    """
    语义单元识别（占位实现）

    Args:
        text: 文本

    Returns:
        语义单元列表
    """
    # Phase M1: Mock 实现
    return [
        {
            "type": "phrase",
            "text": text,
            "start": 0,
            "end": len(text)
        }
    ]