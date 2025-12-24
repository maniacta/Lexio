"""预览接口 - Phase M1 实现"""

from fastapi import APIRouter, HTTPException

from app.core.pipeline import enhance_pipeline
from app.schemas.request import PreviewRequest
from app.schemas.response import PreviewResponse
from app.logging import get_logger

logger = get_logger(__name__)
router = APIRouter()


@router.post("/", response_model=PreviewResponse)
async def preview(request: PreviewRequest):
    """
    预览增强效果

    - **content**: 需要预览的文本
    - **mode**: 增强模式
    """
    try:
        logger.info(f"Preview request - mode: {request.mode}")

        result = enhance_pipeline(request.content, request.mode)

        # 简单的变更追踪（占位）
        changes = []
        if result["enhanced"] != result["original"]:
            changes.append({
                "type": "enhanced",
                "original": result["original"],
                "enhanced": result["enhanced"]
            })

        return PreviewResponse(
            preview=result["enhanced"],
            changes=changes,
            mode=result["mode"]
        )
    except Exception as e:
        logger.error(f"Preview error: {e}")
        raise HTTPException(status_code=500, detail=str(e))


@router.get("/")
async def preview_root():
    """预览接口信息"""
    return {
        "endpoint": "/api/v1/preview",
        "method": "POST",
        "description": "增强效果预览"
    }