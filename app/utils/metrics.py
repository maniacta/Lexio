"""指标埋点 - Phase M2 实现"""

from typing import Dict, Any
from datetime import datetime


class MetricsCollector:
    """指标收集器（占位实现）"""

    def __init__(self):
        self._events = []

    def track_enhance(self, user_id: str, mode: str, result: Dict[str, Any]):
        """跟踪增强事件"""
        event = {
            "type": "enhance",
            "user_id": user_id,
            "mode": mode,
            "timestamp": datetime.utcnow(),
            "data": result
        }
        self._events.append(event)

    def track_stress(self, user_id: str, stress_level: float):
        """跟踪压力事件"""
        event = {
            "type": "stress",
            "user_id": user_id,
            "stress_level": stress_level,
            "timestamp": datetime.utcnow()
        }
        self._events.append(event)

    def track_interaction(self, user_id: str, action: str, metadata: Dict[str, Any]):
        """跟踪用户交互"""
        event = {
            "type": "interaction",
            "user_id": user_id,
            "action": action,
            "timestamp": datetime.utcnow(),
            "metadata": metadata
        }
        self._events.append(event)


# 全局实例
metrics = MetricsCollector()