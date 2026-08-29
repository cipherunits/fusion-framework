"""Method-level response header mutators (add / delete).

Pass header name strings, helper maps (e.g. ``header.cache_control(...)``),
or kwargs. Applied after the handler returns so they override middleware.

Example::

    from fusion_framework.header import LOCATION, cache_control
    from fusion_framework.middleware import add_header, delete_header

    @delete_header("X-Powered-By", LOCATION)
    @add_header(cache_control("no-store"), **{"X-Custom": "1"})
    def get(self):
        return self.response({"ok": True})
"""

from __future__ import annotations

import inspect
from typing import Any, Callable, Mapping, Sequence, TypeVar

F = TypeVar("F", bound=Callable[..., Any])


def _normalize_name(name: Any) -> str:
    return str(name).strip()


def _flatten_header_maps(*items: Any, **extra: str) -> dict[str, str]:
    """Resolve decorator args into a header name → value map."""
    out: dict[str, str] = {}
    # Allow @add_header(LOCATION, "/path", "X-Demo", "1") pairs of strings.
    str_buf: list[str] = []
    for item in items:
        if item is None:
            continue
        if isinstance(item, Mapping):
            for key, value in item.items():
                out[_normalize_name(key)] = "" if value is None else str(value)
        elif isinstance(item, (list, tuple)) and len(item) == 2 and not isinstance(
            item[0], (list, tuple, Mapping)
        ):
            out[_normalize_name(item[0])] = "" if item[1] is None else str(item[1])
        elif isinstance(item, str):
            str_buf.append(item)
        elif callable(item) and not isinstance(item, type):
            continue
    if len(str_buf) >= 2 and len(str_buf) % 2 == 0:
        for i in range(0, len(str_buf), 2):
            out[_normalize_name(str_buf[i])] = str(str_buf[i + 1])
    for key, value in extra.items():
        name = key.replace("_", "-") if "_" in key and "-" not in key else key
        out[_normalize_name(name)] = "" if value is None else str(value)
    return out


def _flatten_header_names(*items: Any) -> list[str]:
    names: list[str] = []
    seen: set[str] = set()

    def add(name: str) -> None:
        key = name.lower()
        if key and key not in seen:
            seen.add(key)
            names.append(name)

    for item in items:
        if item is None:
            continue
        if isinstance(item, Mapping):
            for key in item.keys():
                add(_normalize_name(key))
        elif isinstance(item, str):
            add(_normalize_name(item))
        elif isinstance(item, (list, tuple)):
            if len(item) == 2 and not isinstance(item[0], (list, tuple, Mapping)):
                add(_normalize_name(item[0]))
            else:
                for nested in item:
                    if isinstance(nested, str):
                        add(_normalize_name(nested))
                    elif isinstance(nested, Mapping):
                        for key in nested.keys():
                            add(_normalize_name(key))
    return names


def _ensure_envelope(result: Any) -> dict[str, Any]:
    if isinstance(result, dict) and "status" in result:
        return dict(result)
    return {"status": 200, "body": result}


def _apply_add(result: Any, extra: Mapping[str, str]) -> Any:
    if not extra:
        return result
    out = _ensure_envelope(result)
    headers = out.get("headers")
    if not isinstance(headers, dict):
        headers = {}
    merged = {**headers, **extra}  # add_header wins over handler
    out["headers"] = merged
    return out


def _apply_delete(result: Any, names: Sequence[str]) -> Any:
    if not names:
        return result
    out = _ensure_envelope(result)
    headers = out.get("headers")
    if isinstance(headers, dict):
        drop = {n.lower() for n in names}
        out["headers"] = {k: v for k, v in headers.items() if str(k).lower() not in drop}
    suppressed = out.get("suppress_headers")
    if not isinstance(suppressed, list):
        suppressed = []
    existing = {str(s).lower() for s in suppressed}
    for name in names:
        if name.lower() not in existing:
            suppressed.append(name)
            existing.add(name.lower())
    out["suppress_headers"] = suppressed
    return out


def _wrap_handler(fn: F, after: Callable[[Any], Any]) -> F:
    if inspect.iscoroutinefunction(fn):

        async def async_wrapped(*args: Any, **kwargs: Any) -> Any:
            result = await fn(*args, **kwargs)
            return after(result)

        async_wrapped.__name__ = getattr(fn, "__name__", "wrapped")
        async_wrapped.__qualname__ = getattr(fn, "__qualname__", async_wrapped.__name__)
        async_wrapped.__doc__ = fn.__doc__
        async_wrapped.__module__ = fn.__module__
        async_wrapped.__annotations__ = getattr(fn, "__annotations__", {})
        # Preserve Fusion metadata (http routes, previous header ops, …).
        for key, value in vars(fn).items():
            if key.startswith("__fusion"):
                setattr(async_wrapped, key, value)
        async_wrapped.__wrapped__ = fn  # type: ignore[attr-defined]
        return async_wrapped  # type: ignore[return-value]

    def sync_wrapped(*args: Any, **kwargs: Any) -> Any:
        result = fn(*args, **kwargs)
        if inspect.isawaitable(result):

            async def _await_after() -> Any:
                return after(await result)

            return _await_after()
        return after(result)

    sync_wrapped.__name__ = getattr(fn, "__name__", "wrapped")
    sync_wrapped.__qualname__ = getattr(fn, "__qualname__", sync_wrapped.__name__)
    sync_wrapped.__doc__ = fn.__doc__
    sync_wrapped.__module__ = fn.__module__
    sync_wrapped.__annotations__ = getattr(fn, "__annotations__", {})
    for key, value in vars(fn).items():
        if key.startswith("__fusion"):
            setattr(sync_wrapped, key, value)
    sync_wrapped.__wrapped__ = fn  # type: ignore[attr-defined]
    return sync_wrapped  # type: ignore[return-value]


def add_header(*items: Any, **extra: str) -> Callable[[F], F]:
    """Merge headers into the handler response (wins over middleware defaults)."""
    headers = _flatten_header_maps(*items, **extra)

    def decorator(fn: F) -> F:
        existing = getattr(fn, "__fusion_add_headers__", None)
        merged = {**(existing or {}), **headers}
        wrapped = _wrap_handler(fn, lambda result: _apply_add(result, merged))
        wrapped.__fusion_add_headers__ = merged  # type: ignore[attr-defined]
        return wrapped

    return decorator


def delete_header(*items: Any) -> Callable[[F], F]:
    """Strip headers from the handler response and suppress wire fingerprint re-add."""
    names = _flatten_header_names(*items)

    def decorator(fn: F) -> F:
        existing = list(getattr(fn, "__fusion_delete_headers__", []) or [])
        merged = _flatten_header_names(*(existing + names))
        wrapped = _wrap_handler(fn, lambda result: _apply_delete(result, merged))
        wrapped.__fusion_delete_headers__ = merged  # type: ignore[attr-defined]
        return wrapped

    return decorator


AddHeader = add_header
DeleteHeader = delete_header

__all__ = [
    "add_header",
    "delete_header",
    "AddHeader",
    "DeleteHeader",
]
