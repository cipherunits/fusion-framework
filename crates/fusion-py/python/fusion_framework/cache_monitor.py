"""Built-in process-wide cache monitor panel (HTML + JSON).

Enabled only when ``cache.monitor.enabled`` is true in ``fusion.<env>.json``.
When disabled, routes are not registered (security: no endpoints, not just UI).
"""

from __future__ import annotations

from typing import Any

from fusion_framework.template import FusionBaseTemplate


class CacheMonitorPanel(FusionBaseTemplate):
    """Default cache monitoring page using Fusion Tera components."""

    template = "fusion/cache_monitor.html"

    def context(self) -> dict[str, Any]:
        """Live entries + recent set/delete/clear events for the panel."""
        from fusion_framework import cache

        return dict(cache.panel_context())


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
    path = str(raw or "/__fusion/cache").strip() or "/__fusion/cache"
    if not path.startswith("/"):
        path = f"/{path}"
    return path.rstrip("/") or "/__fusion/cache"


def mount_cache_monitor(engine, settings) -> bool:
    """Register HTML + ``/json`` routes when ``cache.monitor.enabled`` is true.

    Returns whether routes were mounted.
    """
    from fusion_framework import cache

    if not _truthy_enabled(settings.get("cache.monitor.enabled", default=False), default=False):
        return False

    # Apply settings so monitor_path / max_events match the env file.
    cache.configure(settings)
    path = _normalize_monitor_path(
        settings.get("cache.monitor.path", default="/__fusion/cache")
    )

    def html_handler(req: dict) -> Any:
        """Serve the monitor HTML (or JSON when Accept prefers JSON)."""
        return CacheMonitorPanel(req).get()

    def json_handler(_req: dict) -> Any:
        """Serve the raw monitor snapshot JSON."""
        return cache.snapshot()

    engine.route("GET", path, html_handler)
    if path != "/":
        engine.route("GET", f"{path}/", html_handler)
    engine.route("GET", f"{path}/json", json_handler)
    return True


__all__ = ["CacheMonitorPanel", "mount_cache_monitor"]
