"""Built-in Fusion UI surfaces: component gallery at ``/__fusion/component``.

Serves ``components.html`` (gallery) and static assets from the fusion-core
templates folder when that directory is available (editable / repo installs).
"""

from __future__ import annotations

import mimetypes
from pathlib import Path
from typing import Any

DEFAULT_COMPONENT_PATH = "/__fusion/component"


def _normalize_path(raw: Any, default: str = DEFAULT_COMPONENT_PATH) -> str:
    """Normalize a URL path (leading slash, no trailing slash)."""
    path = str(raw or default).strip() or default
    if not path.startswith("/"):
        path = f"/{path}"
    return path.rstrip("/") or default


def resolve_fusion_ui_assets() -> Path | None:
    """Locate ``crates/fusion-core/assets/templates/fusion`` when present on disk."""
    here = Path(__file__).resolve()
    candidates: list[Path] = []
    # Editable install: .../crates/fusion-py/python/fusion_framework/ui.py
    if len(here.parents) >= 4:
        candidates.append(
            here.parents[3] / "fusion-core" / "assets" / "templates" / "fusion"
        )
    # Repo checkout from examples/ or CWD
    cwd = Path.cwd()
    candidates.append(cwd / "crates" / "fusion-core" / "assets" / "templates" / "fusion")
    candidates.append(
        cwd.parent / "crates" / "fusion-core" / "assets" / "templates" / "fusion"
    )
    for path in candidates:
        if (path / "components.html").is_file():
            return path
    return None


def _content_type(path: Path) -> str:
    """Guess a Content-Type for a static UI asset."""
    guessed, _ = mimetypes.guess_type(str(path))
    if guessed:
        return guessed
    if path.suffix == ".js":
        return "application/javascript; charset=utf-8"
    if path.suffix == ".css":
        return "text/css; charset=utf-8"
    if path.suffix == ".html":
        return "text/html; charset=utf-8"
    return "application/octet-stream"


def mount_component_gallery(engine, settings=None) -> bool:
    """Register ``/__fusion/component`` HTML + static assets when files exist.

    Returns whether routes were mounted.
    """
    root = resolve_fusion_ui_assets()
    if root is None:
        return False

    path = DEFAULT_COMPONENT_PATH
    if settings is not None:
        raw = settings.get("ui.component_path", default=None)
        if raw is not None and str(raw).strip():
            path = _normalize_path(raw)

    gallery = root / "components.html"
    if not gallery.is_file():
        return False

    def html_handler(_req: dict) -> Any:
        """Serve the component gallery HTML."""
        return {
            "status": 200,
            "headers": {"content-type": "text/html; charset=utf-8"},
            "body": gallery.read_bytes(),
        }

    engine.route("GET", path, html_handler)
    if path != "/":
        engine.route("GET", f"{path}/", html_handler)

    # Register each asset under /__fusion/component/...
    for file_path in root.rglob("*"):
        if not file_path.is_file():
            continue
        rel = file_path.relative_to(root).as_posix()
        if rel in ("components.html", "index.html", "monitor.html", "cache_monitor.html"):
            continue
        url = f"{path}/{rel}"

        def _make_handler(target: Path):
            def asset_handler(_req: dict, p: Path = target) -> Any:
                """Serve one gallery static file."""
                return {
                    "status": 200,
                    "headers": {"content-type": _content_type(p)},
                    "body": p.read_bytes(),
                }

            return asset_handler

        engine.route("GET", url, _make_handler(file_path))

    return True


__all__ = [
    "DEFAULT_COMPONENT_PATH",
    "resolve_fusion_ui_assets",
    "mount_component_gallery",
]
