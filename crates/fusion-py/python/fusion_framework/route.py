from __future__ import annotations

import asyncio
import inspect
from typing import Any, Callable, Type

from fusion_framework.api import FusionBaseApi

_REGISTRY: list[tuple[str, Type[FusionBaseApi]]] = []

# Reserved placeholder: replaced with the class name without a trailing "Api".
_CLASS_NAME_TOKEN = "[name]"


def api_resource_name(cls: Type[Any] | str) -> str:
    """``MyFirstApi`` → ``MyFirst``; leaves names without an ``Api`` suffix unchanged."""
    name = cls if isinstance(cls, str) else cls.__name__
    if name.endswith("Api") and len(name) > 3:
        return name[: -len("Api")]
    if name.endswith("API") and len(name) > 3:
        return name[: -len("API")]
    return name


def resolve_route_path(path: str, cls: Type[Any]) -> str:
    """Expand ``[name]`` in a route template using the API class name."""
    return path.replace(_CLASS_NAME_TOKEN, api_resource_name(cls))


def router(path: str) -> Callable[[Type[FusionBaseApi]], Type[FusionBaseApi]]:
    """Register a ``FusionBaseApi`` subclass on ``path``.

    ``[name]`` is replaced with the class name without the ``Api`` suffix
    (e.g. ``MyFirstApi`` + ``/api/[name]/{id}`` → ``/api/MyFirst/{id}``).
    Other ``{param}`` / ``[param]`` segments remain path parameters.
    """

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


def _coerce(value: str, annotation: Any) -> Any:
    if annotation is inspect.Parameter.empty or annotation is str or annotation is Any:
        return value
    if annotation is int:
        return int(value)
    if annotation is float:
        return float(value)
    if annotation is bool:
        return value.lower() in {"1", "true", "yes", "on"}
    try:
        return annotation(value)
    except Exception:
        return value


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
        kwargs[name] = _coerce(params[name], parameter.annotation)
    return kwargs


def invoke_api_method(api_cls: Type[FusionBaseApi], method_name: str, request: dict[str, Any]) -> Any:
    instance = api_cls(request)
    method = getattr(instance, method_name)
    kwargs = _bind_kwargs(method, dict(request.get("params") or {}))
    result = method(**kwargs)
    if inspect.isawaitable(result):
        result = asyncio.run(result)
    return result
