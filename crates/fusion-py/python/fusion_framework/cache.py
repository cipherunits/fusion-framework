"""Process-wide application cache (default driver: moka).

Configure via ``fusion.<env>.json``::

    "cache": {
      "driver": "moka",
      "max_capacity": 10000,
      "default_ttl": null,
      "connection_string": null,
      "host": "127.0.0.1",
      "port": 6379,
      "username": null,
      "password": null,
      "db": 0,
      "monitor": {
        "enabled": true,
        "path": "/__fusion/cache",
        "max_events": 50
      }
    }

``default_ttl``:

- ``null`` (scaffold default) — entries live forever unless you pass ``ttl=``
- a number (seconds) — used when ``set`` / ``get_or_set`` omit ``ttl``

Sync::

    from fusion_framework import cache

    cache.set("user:1", {"name": "Ada"})           # forever (if default_ttl is null)
    cache.set("user:1", {"name": "Ada"}, ttl=60)   # expire in 60s
    cache.get("user:1")
    cache.clear()

Async::

    await cache.aset("user:1", {"name": "Ada"}, ttl=60)
    await cache.aget("user:1")
    await cache.aget_or_set("user:1", fetch_user)
    await cache.aclear()
"""

from __future__ import annotations

import asyncio
import inspect
from typing import Any, Awaitable, Callable, Optional, Union

from fusion_framework._fusion import (
    cache_clear as _cache_clear,
    cache_configure as _cache_configure,
    cache_configure_driver as _cache_configure_driver,
    cache_delete as _cache_delete,
    cache_delete_or_set as _cache_delete_or_set,
    cache_driver as _cache_driver,
    cache_exists as _cache_exists,
    cache_exists_or_set as _cache_exists_or_set,
    cache_get as _cache_get,
    cache_get_or_set as _cache_get_or_set,
    cache_panel_context as _cache_panel_context,
    cache_reset as _cache_reset,
    cache_set as _cache_set,
    cache_snapshot as _cache_snapshot,
)

_configured = False

DefaultFactory = Union[Any, Callable[[], Any], Callable[[], Awaitable[Any]]]


def _ensure() -> None:
    """Load settings once and apply ``cache.*`` (falls back to default moka)."""
    global _configured
    if _configured:
        return
    from fusion_framework.config import get_settings

    settings = get_settings()
    _cache_configure(settings)
    _configured = True


def configure(settings: Any = None) -> None:
    """Apply cache settings from a Settings object (or reload from get_settings)."""
    global _configured
    if settings is None:
        from fusion_framework.config import get_settings

        settings = get_settings()
    _cache_configure(settings)
    _configured = True


def configure_driver(
    driver: str = "moka",
    *,
    max_capacity: Optional[int] = None,
    default_ttl: Optional[float] = None,
) -> None:
    """Install a driver explicitly (tests / advanced use)."""
    global _configured
    _cache_configure_driver(driver, max_capacity, default_ttl)
    _configured = True


def set(key: str, value: Any, ttl: Optional[float] = None) -> None:
    """Store ``value`` under ``key`` (JSON-compatible).

    ``ttl`` is seconds. If omitted, uses ``cache.default_ttl`` from settings;
    when that is ``null``, the entry does not expire.
    """
    _ensure()
    _cache_set(key, value, ttl)


def get(key: str) -> Any:
    """Return the cached value, or ``None`` if missing/expired."""
    _ensure()
    return _cache_get(key)


def delete(key: str) -> bool:
    """Remove ``key``; returns whether it existed."""
    _ensure()
    return bool(_cache_delete(key))


def exists(key: str) -> bool:
    """True when ``key`` is present and not expired."""
    _ensure()
    return bool(_cache_exists(key))


def get_or_set(
    key: str,
    default: Union[Any, Callable[[], Any]],
    ttl: Optional[float] = None,
) -> Any:
    """Return cached value, or evaluate/store ``default`` and return it."""
    _ensure()
    return _cache_get_or_set(key, default, ttl)


def delete_or_set(key: str, value: Any, ttl: Optional[float] = None) -> Any:
    """Delete ``key`` (if any), then set ``value``; returns the stored value."""
    _ensure()
    return _cache_delete_or_set(key, value, ttl)


def exists_or_set(key: str, value: Any, ttl: Optional[float] = None) -> bool:
    """If ``key`` exists return ``True``; otherwise set ``value`` and return ``False``."""
    _ensure()
    return bool(_cache_exists_or_set(key, value, ttl))


def clear() -> None:
    """Remove every entry from the process-wide cache."""
    _ensure()
    _cache_clear()


def driver() -> str:
    """Active cache driver name (e.g. ``\"moka\"``)."""
    _ensure()
    return str(_cache_driver())


def snapshot() -> dict[str, Any]:
    """JSON snapshot of live entries and recent cache mutations (for the monitor)."""
    _ensure()
    return dict(_cache_snapshot())


def panel_context() -> dict[str, Any]:
    """Template context for the built-in ``fusion/cache_monitor.html`` panel."""
    _ensure()
    return dict(_cache_panel_context())


def reset() -> None:
    """Drop the global cache instance (tests)."""
    global _configured
    _cache_reset()
    _configured = False


async def _maybe_await(value: Any) -> Any:
    """Await ``value`` when it is awaitable; otherwise return it unchanged."""
    if inspect.isawaitable(value):
        return await value
    return value


async def _run_sync(fn: Callable[[], Any]) -> Any:
    """Run a sync cache op off the event loop when a loop is running."""
    try:
        asyncio.get_running_loop()
    except RuntimeError:
        return fn()
    return await asyncio.to_thread(fn)


async def aset(key: str, value: Any, ttl: Optional[float] = None) -> None:
    """Async ``set``."""
    await _run_sync(lambda: set(key, value, ttl))


async def aget(key: str) -> Any:
    """Async ``get``."""
    return await _run_sync(lambda: get(key))


async def adelete(key: str) -> bool:
    """Async ``delete``."""
    return await _run_sync(lambda: delete(key))


async def aexists(key: str) -> bool:
    """Async ``exists``."""
    return await _run_sync(lambda: exists(key))


async def aget_or_set(
    key: str,
    default: DefaultFactory,
    ttl: Optional[float] = None,
) -> Any:
    """Async ``get_or_set``; ``default`` may be sync/async callable or a value."""
    _ensure()
    if await _run_sync(lambda: exists(key)):
        return await _run_sync(lambda: get(key))
    if callable(default):
        value = await _maybe_await(default())
    else:
        value = default
    await _run_sync(lambda: set(key, value, ttl))
    return value


async def adelete_or_set(key: str, value: Any, ttl: Optional[float] = None) -> Any:
    """Async ``delete_or_set``."""
    return await _run_sync(lambda: delete_or_set(key, value, ttl))


async def aexists_or_set(key: str, value: Any, ttl: Optional[float] = None) -> bool:
    """Async ``exists_or_set``."""
    return await _run_sync(lambda: exists_or_set(key, value, ttl))


async def aclear() -> None:
    """Async ``clear``."""
    await _run_sync(clear)


__all__ = [
    "configure",
    "configure_driver",
    "set",
    "get",
    "delete",
    "exists",
    "get_or_set",
    "delete_or_set",
    "exists_or_set",
    "clear",
    "driver",
    "snapshot",
    "panel_context",
    "reset",
    "aset",
    "aget",
    "adelete",
    "aexists",
    "aget_or_set",
    "adelete_or_set",
    "aexists_or_set",
    "aclear",
]
