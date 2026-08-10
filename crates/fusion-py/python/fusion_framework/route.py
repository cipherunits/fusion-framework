from __future__ import annotations

import inspect
import json
from typing import Any, Callable, Mapping, Type, get_args, get_origin, Union

from fusion_framework._fusion import (
    HTTP_METHODS,
    api_resource_name as _api_resource_name,
    coerce_param as _coerce_param,
    resolve_route_path as _resolve_route_path,
)
from fusion_framework.api import FusionBaseApi
from fusion_framework.http import HTTPException

_REGISTRY: list[tuple[str, Type[FusionBaseApi]]] = []

_BODY_METHODS = frozenset({"POST", "PUT", "PATCH"})


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


def _unwrap_optional(annotation: Any) -> tuple[Any, bool]:
    """Return (inner_type, is_optional) for ``T | None`` / ``Optional[T]``."""
    if annotation is inspect.Parameter.empty:
        return annotation, False
    origin = get_origin(annotation)
    if origin is Union:
        args = [a for a in get_args(annotation)]
        non_none = [a for a in args if a is not type(None)]
        if type(None) in args and len(non_none) == 1:
            return non_none[0], True
    return annotation, False


def _annotation_kind(annotation: Any) -> str:
    annotation, _ = _unwrap_optional(annotation)
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


def _parse_json_object(body: Any) -> dict[str, Any] | None:
    if body is None:
        return None
    if isinstance(body, dict):
        return body
    text = str(body).strip()
    if not text:
        return None
    try:
        parsed = json.loads(text)
    except json.JSONDecodeError:
        return None
    return parsed if isinstance(parsed, dict) else None


def _coerce_value(raw: Any, kind: str) -> Any:
    """Coerce a path/query string or JSON body scalar to the annotated kind."""
    if kind == "int":
        if isinstance(raw, bool):
            raise HTTPException(400, {"detail": "expected int, got bool"})
        if isinstance(raw, int):
            return raw
        return _coerce_param(str(raw), "int")
    if kind == "float":
        if isinstance(raw, bool):
            raise HTTPException(400, {"detail": "expected float, got bool"})
        if isinstance(raw, (int, float)) and not isinstance(raw, bool):
            return float(raw)
        return _coerce_param(str(raw), "float")
    if kind == "bool":
        if isinstance(raw, bool):
            return raw
        return _coerce_param(str(raw), "bool")
    if raw is None:
        return ""
    if isinstance(raw, str):
        return raw
    return str(raw)


def _bind_kwargs(method: Callable[..., Any], request: Mapping[str, Any]) -> dict[str, Any]:
    """Bind handler args: path first, then query (read) or JSON body (write)."""
    signature = inspect.signature(method)
    path_params = dict(request.get("params") or {})
    query_params = dict(request.get("query") or {})
    http_method = str(request.get("method", "")).upper()
    body_fields = (
        _parse_json_object(request.get("body")) if http_method in _BODY_METHODS else None
    )

    kwargs: dict[str, Any] = {}
    for name, parameter in signature.parameters.items():
        if name == "self":
            continue

        _, is_optional = _unwrap_optional(parameter.annotation)
        kind = _annotation_kind(parameter.annotation)
        has_default = parameter.default is not inspect.Parameter.empty
        source: str | None = None
        raw: Any = None

        if name in path_params:
            source = "path"
            raw = path_params[name]
        elif http_method in _BODY_METHODS:
            if body_fields is not None and name in body_fields:
                source = "body"
                raw = body_fields[name]
        elif name in query_params:
            source = "query"
            raw = query_params[name]

        if source is None:
            if has_default:
                continue
            # Missing query/body: pass None so the handler can raise HTTPException
            # with a custom message (e.g. if not id: raise HTTPException(...)).
            kwargs[name] = None
            continue

        if raw is None and is_optional:
            kwargs[name] = None
            continue

        kwargs[name] = _coerce_value(raw, kind)

    return kwargs


async def _await_http_safe(awaitable: Any) -> Any:
    try:
        return await awaitable
    except HTTPException as exc:
        return exc.to_response()


def invoke_api_method(api_cls: Type[FusionBaseApi], method_name: str, request: dict[str, Any]) -> Any:
    """Call the API method. May return a value or an awaitable (handled in Rust)."""
    instance = api_cls(request)
    method = getattr(instance, method_name)
    try:
        kwargs = _bind_kwargs(method, request)
        result = method(**kwargs)
    except HTTPException as exc:
        return exc.to_response()

    if inspect.isawaitable(result):
        return _await_http_safe(result)
    return result


__all__ = [
    "HTTP_METHODS",
    "HTTPException",
    "api_resource_name",
    "resolve_route_path",
    "router",
    "registered_routes",
    "clear_registry",
    "invoke_api_method",
]
