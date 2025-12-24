"""增强接口 - Phase M1 实现"""

from fastapi import APIRouter, HTTPException
from pydantic import BaseModel

from app.core.pipeline import enhance_pipeline
from app.schemas.request import EnhanceRequest
from app.schemas.response import EnhanceResponse
from app.utils.metrics import metrics
from app.logging import get_logger

logger = get_logger(__name__)
router = APIRouter()


@router.post("/", response_model=EnhanceResponse)
async def enhance(request: EnhanceRequest):
    """
    增强文本接口

    - **content**: 需要增强的文本
    - **mode**: 增强模式 (basic, i_plus_1)
    - **user_id**: 用户ID（可选，用于获取 Profile）
    """
    try:
        logger.info(f"Enhance request - mode: {request.mode}")

        # 调用核心流程
        result = enhance_pipeline(request.content, request.mode)

        # 记录指标
        if request.user_id:
            metrics.track_enhance(request.user_id, request.mode, result)

        return EnhanceResponse(
            original=result["original"],
            enhanced=result["enhanced"],
            mode=result["mode"],
            metadata={"status": result["status"]}
        )
    except Exception as e:
        logger.error(f"Enhance error: {e}")
        raise HTTPException(status_code=500, detail=str(e))


@router.get("/")
async def enhance_root():
    """增强接口信息"""
    return {
        "endpoint": "/api/v1/enhance",
        "method": "POST",
        "description": "文本增强服务",
        "modes": ["basic", "i_plus_1"]
    }