"""Fusion Framework — thin Python interface over fusion-core."""

from fusion_framework.config import settings
from fusion_framework.http import HTTPException
from fusion_framework.middleware import (
    bearer_jwt,
    cache_headers,
    cors,
    framework_headers,
    request_id,
    require_permissions,
    require_roles,
    security_headers,
    use,
)
from fusion_framework.pagination import PaginationParams, paginated_body, parse_pagination
from fusion_framework.template import FusionBaseTemplate, render_template
from . import header, status

__all__ = [
    "settings",
    "status",
    "header",
    "HTTPException",
    "bearer_jwt",
    "cache_headers",
    "cors",
    "framework_headers",
    "request_id",
    "require_permissions",
    "require_roles",
    "security_headers",
    "use",
    "PaginationParams",
    "parse_pagination",
    "paginated_body",
    "FusionBaseTemplate",
    "render_template",
]
