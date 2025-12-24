"""行为驱动更新 - Phase M2 实现"""

from typing import Dict, Any
from app.profile.schema import UserLanguageProfile, ProfileUpdate


def update_profile(profile: UserLanguageProfile, update: ProfileUpdate) -> UserLanguageProfile:
    """
    更新用户语言模型

    Args:
        profile: 原始 profile
        update: 更新数据

    Returns:
        更新后的 profile
    """
    # 更新已知词汇
    for word in update.known_words:
        profile.known_vocab.add(word)
        if word in profile.unknown_vocab:
            profile.unknown_vocab.remove(word)

    # 更新未知词汇
    for word in update.unknown_words:
        profile.unknown_vocab.add(word)
        if word in profile.known_vocab:
            profile.known_vocab.remove(word)

    # 更新压力容忍度（基于压力事件）
    if update.stress_events:
        avg_stress = sum(update.stress_events) / len(update.stress_events)
        # 根据平均压力调整容忍度
        if avg_stress > 0.7:
            profile.stress_tolerance = min(1.0, profile.stress_tolerance + 0.05)
        elif avg_stress < 0.3:
            profile.stress_tolerance = max(0.1, profile.stress_tolerance - 0.02)

    profile.updated_at = __import__('datetime').datetime.utcnow()
    return profile