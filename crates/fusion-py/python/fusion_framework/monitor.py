"""Built-in Fusion monitor panel (cache + background tasks).

Enabled when ``monitor.enabled`` is true in ``fusion.<env>.json``
(legacy ``cache.monitor.enabled`` still works). When disabled, routes are
not registered.
"""

from __future__ import annotations

from typing import Any

from fusion_framework.template import FusionBaseTemplate


class MonitorPanel(FusionBaseTemplate):
    """Default Fusion monitoring page (cache entries, events, tasks)."""

    template = "fusion/monitor.html"

    def context(self) -> dict[str, Any]:
        """Live cache + task snapshot for the panel."""
        from fusion_framework import cache

        return dict(cache.panel_context())


# Backward-compatible alias.
CacheMonitorPanel = MonitorPanel


def _truthy_enabled(value: Any, default: bool = False) -> bool:
    """Interpret settings booleans (same rules as swagger.enabled)."""
    if value is None:
        return default
    if value in (False, 0, "false", "False", "0", "off", "no"):
        return False
    if value in (True, 1, "true", "True", "1", "on", "yes"):
        return True
    return bool(value)


def _normalize_monitor_path(raw: Any) -> str:
    """Normalize monitor URL path (leading slash, no trailing slash)."""
    path = str(raw or "/__fusion/monitor").strip() or "/__fusion/monitor"
    if not path.startswith("/"):
        path = f"/{path}"
    return path.rstrip("/") or "/__fusion/monitor"


def _monitor_enabled(settings) -> bool:
    """Prefer top-level monitor.enabled; fall back to legacy cache.monitor.enabled."""
    top = settings.get("monitor.enabled", default=None)
    if top is not None:
        return _truthy_enabled(top, default=False)
    return _truthy_enabled(
        settings.get("cache.monitor.enabled", default=False), default=False
    )


def _monitor_path(settings) -> str:
    """Prefer monitor.path; fall back to legacy cache.monitor.path."""
    top = settings.get("monitor.path", default=None)
    if top is not None and str(top).strip():
        return _normalize_monitor_path(top)
    return _normalize_monitor_path(
        settings.get("cache.monitor.path", default="/__fusion/monitor")
    )


def mount_monitor(engine, settings) -> bool:
    """Register HTML + ``/json`` routes when ``monitor.enabled`` is true.

    Returns whether routes were mounted.
    """
    from fusion_framework import cache

    if not _monitor_enabled(settings):
        return False

    # Apply cache + monitor settings so snapshot/path match the env file.
    cache.configure(settings)
    path = _monitor_path(settings)

    def html_handler(req: dict) -> Any:
        """Serve the monitor HTML (or JSON when Accept prefers JSON)."""
        return MonitorPanel(req).get()

    def json_handler(_req: dict) -> Any:
        """Serve the raw monitor snapshot JSON."""
        return cache.snapshot()

    engine.route("GET", path, html_handler)
    if path != "/":
        engine.route("GET", f"{path}/", html_handler)
    engine.route("GET", f"{path}/json", json_handler)
    return True


# Backward-compatible alias.
mount_cache_monitor = mount_monitor


__all__ = [
    "MonitorPanel",
    "CacheMonitorPanel",
    "mount_monitor",
    "mount_cache_monitor",
]
