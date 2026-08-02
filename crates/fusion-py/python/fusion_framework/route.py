from __future__ import annotations

import inspect
from typing import Any, Callable, Type

from fusion_framework._fusion import (
    HTTP_METHODS,
    api_resource_name as _api_resource_name,
    coerce_param as _coerce_param,
    resolve_route_path as _resolve_route_path,
)
from fusion_framework.api import FusionBaseApi

_REGISTRY: list[tuple[str, Type[FusionBaseApi]]] = []


def api_resource_name(cls: Type[Any] | str) -> str:
    name = cls if isinstance(cls, str) else cls.__name__
    return _api_resource_name(name)


def resolve_route_path(path: str, cls: Type[Any]) -> str:
    return _resolve_route_path(path, cls.__name__)


def router(path: str) -> Callable[[Type[FusionBaseApi]], Type[FusionBaseApi]]:
    """Register a ``FusionBaseApi`` subclass. Path resolution is done in Rust."""

    def decorator(cls: Type[FusionBaseApi]) -> Type[FusionBaseApi]:
        if not issubclass(cls, FusionBaseApi):
            raise TypeError(f"{cls.__name__} must inherit from FusionBaseApi")
        resolved = resolve_route_path(path, cls)
        cls.__fusion_path__ = resolved  # type: ignore[attr-defined]
        cls.__fusion_path_template__ = path  # type: ignore[attr-defined]
        _REGISTRY.append((resolved, cls))
        return cls

    return decorator


def registered_routes() -> list[tuple[str, Type[FusionBaseApi]]]:
    return list(_REGISTRY)


def clear_registry() -> None:
    _REGISTRY.clear()


def _annotation_kind(annotation: Any) -> str:
    if annotation is inspect.Parameter.empty or annotation is str or annotation is Any:
        return "str"
    if annotation is int:
        return "int"
    if annotation is float:
        return "float"
    if annotation is bool:
        return "bool"
    name = getattr(annotation, "__name__", None)
    return name.lower() if isinstance(name, str) else "str"


def _bind_kwargs(method: Callable[..., Any], params: dict[str, str]) -> dict[str, Any]:
    signature = inspect.signature(method)
    kwargs: dict[str, Any] = {}
    for name, parameter in signature.parameters.items():
        if name == "self":
            continue
        if name not in params:
            if parameter.default is inspect.Parameter.empty:
                raise TypeError(f"missing path param '{name}' for {method.__qualname__}")
            continue
        kind = _annotation_kind(parameter.annotation)
        kwargs[name] = _coerce_param(params[name], kind)
    return kwargs


def invoke_api_method(api_cls: Type[FusionBaseApi], method_name: str, request: dict[str, Any]) -> Any:
    """Call the API method. May return a value or an awaitable (handled in Rust)."""
    instance = api_cls(request)
    method = getattr(instance, method_name)
    kwargs = _bind_kwargs(method, dict(request.get("params") or {}))
    return method(**kwargs)


# Re-export for callers that used the constant from this module historically.
__all__ = [
    "HTTP_METHODS",
    "api_resource_name",
    "resolve_route_path",
    "router",
    "registered_routes",
    "clear_registry",
    "invoke_api_method",
]
