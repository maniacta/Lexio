"""用户语言模型接口 - Phase M2 实现"""

from fastapi import APIRouter, HTTPException

from app.profile.schema import UserLanguageProfile, ProfileUpdate
from app.profile.repository import profile_repo
from app.profile.updater import update_profile
from app.schemas.request import ProfileUpdateRequest
from app.logging import get_logger

logger = get_logger(__name__)
router = APIRouter()


@router.get("/{user_id}")
async def get_profile(user_id: str):
    """
    获取用户 Profile

    - **user_id**: 用户ID
    """
    try:
        profile = await profile_repo.get(user_id)
        if not profile:
            # 创建默认 Profile
            profile = UserLanguageProfile(user_id=user_id)

        return {
            "user_id": profile.user_id,
            "known_vocab_count": len(profile.known_vocab),
            "unknown_vocab_count": len(profile.unknown_vocab),
            "stress_tolerance": profile.stress_tolerance,
            "current_level": profile.current_level,
            "updated_at": profile.updated_at
        }
    except Exception as e:
        logger.error(f"Get profile error: {e}")
        raise HTTPException(status_code=500, detail=str(e))


@router.post("/")
async def update_profile_endpoint(request: ProfileUpdateRequest):
    """
    更新用户 Profile

    - **user_id**: 用户ID
    - **known_words**: 已知词汇
    - **unknown_words**: 未知词汇
    - **stress_events**: 压力事件列表
    """
    try:
        # 获取或创建 Profile
        profile = await profile_repo.get(request.user_id)
        if not profile:
            profile = UserLanguageProfile(user_id=request.user_id)

        # 更新 Profile
        update_data = ProfileUpdate(
            known_words=request.known_words,
            unknown_words=request.unknown_words,
            stress_events=request.stress_events
        )
        updated_profile = update_profile(profile, update_data)

        # 保存
        await profile_repo.save(updated_profile)

        logger.info(f"Profile updated for user: {request.user_id}")

        return {
            "status": "success",
            "user_id": updated_profile.user_id,
            "stress_tolerance": updated_profile.stress_tolerance
        }
    except Exception as e:
        logger.error(f"Update profile error: {e}")
        raise HTTPException(status_code=500, detail=str(e))


@router.get("/")
async def profile_root():
    """Profile 接口信息"""
    return {
        "endpoint": "/api/v1/profile",
        "methods": ["GET /{user_id}", "POST /"],
        "description": "用户语言模型管理"
    }