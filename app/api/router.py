"""API 路由聚合"""

from fastapi import APIRouter

from app.api.v1 import enhance, preview, profile

api_router = APIRouter()

# v1 路由
api_router.include_router(enhance.router, prefix="/v1/enhance", tags=["v1/enhance"])
api_router.include_router(preview.router, prefix="/v1/preview", tags=["v1/preview"])
api_router.include_router(profile.router, prefix="/v1/profile", tags=["v1/profile"])