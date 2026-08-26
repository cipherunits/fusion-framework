"""Thin route registration — dispatch and middleware orchestration live in Rust/Python."""

from __future__ import annotations

from typing import Callable, Optional, Sequence, Type

from fusion_framework._fusion import (
    HTTP_METHODS,
    api_resource_name,
    clear_routes,
    register_route,
    resolve_route_path,
)
from fusion_framework.api import FusionBaseApi
from fusion_framework.http_route import (
    HttpDelete,
    HttpGet,
    HttpHead,
    HttpOptions,
    HttpPatch,
    HttpPost,
    HttpPut,
    http_delete,
    http_get,
    http_head,
    http_options,
    http_patch,
    http_post,
    http_put,
)
from fusion_framework.middleware import require_roles

__all__ = [
    "HTTP_METHODS",
    "HttpDelete",
    "HttpGet",
    "HttpHead",
    "HttpOptions",
    "HttpPatch",
    "HttpPost",
    "HttpPut",
    "api_resource_name",
    "clear_registry",
    "http_delete",
    "http_get",
    "http_head",
    "http_options",
    "http_patch",
    "http_post",
    "http_put",
    "resolve_route_path",
    "route",
    "router",
]


def route(
    path: str,
    *,
    tags: Optional[list[str]] = None,
    desc: Optional[str] = None,
    title: Optional[str] = None,
    version: Optional[str] = None,
    deprecated: bool = False,
    middleware: Optional[Sequence] = None,
    roles: Optional[Sequence[str]] = None,
) -> Callable[[Type[FusionBaseApi]], Type[FusionBaseApi]]:
    """Register a ``FusionBaseApi`` subclass.

    ``middleware`` — callables ``(request, call_next) -> response | call_next(...)``.
    ``roles`` — shorthand that appends a ``require_roles(...)`` route middleware.
    ``version`` — API prefix such as ``v1`` (path becomes ``/v1/...``). Each
    version gets its own OpenAPI spec and appears in the Swagger navbar.
    """

    def decorator(cls: Type[FusionBaseApi]) -> Type[FusionBaseApi]:
        route_middleware = list(middleware or [])
        if roles:
            route_middleware.append(require_roles(*roles))

        register_route(
            path,
            cls,
            tags or [],
            desc,
            title,
            version,
            deprecated,
            route_middleware,
        )
        return cls

    return decorator


def router(path: str) -> Callable[[Type[FusionBaseApi]], Type[FusionBaseApi]]:
    """Backward-compatible alias of ``route(path)`` with no Swagger metadata."""
    return route(path)


def clear_registry() -> None:
    clear_routes()
