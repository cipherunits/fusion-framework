"""Thin route registration — all dispatch logic lives in Rust."""

from __future__ import annotations

from typing import Callable, Optional, Type

from fusion_framework._fusion import (
    HTTP_METHODS,
    api_resource_name,
    clear_routes,
    register_route,
    resolve_route_path,
)
from fusion_framework.api import FusionBaseApi

__all__ = [
    "HTTP_METHODS",
    "api_resource_name",
    "resolve_route_path",
    "route",
    "router",
    "clear_registry",
]


def route(
    path: str,
    *,
    tags: Optional[list[str]] = None,
    desc: Optional[str] = None,
    title: Optional[str] = None,
    version: Optional[str] = None,
    deprecated: bool = False,
) -> Callable[[Type[FusionBaseApi]], Type[FusionBaseApi]]:
    """Register a ``FusionBaseApi`` subclass with Swagger/OpenAPI metadata."""

    def decorator(cls: Type[FusionBaseApi]) -> Type[FusionBaseApi]:
        register_route(
            path,
            cls,
            tags or [],
            desc,
            title,
            version,
            deprecated,
        )
        return cls

    return decorator


def router(path: str) -> Callable[[Type[FusionBaseApi]], Type[FusionBaseApi]]:
    """Backward-compatible alias of `route(path)` with no Swagger metadata."""
    return route(path)


def clear_registry() -> None:
    clear_routes()
