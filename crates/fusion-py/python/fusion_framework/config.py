"""Thin Python façade over fusion-core Settings.

JSON discovery, env placeholders, and key lookup live in Rust.
Only the Python ``settings.py`` module overlay stays here (importlib).
"""

from __future__ import annotations

import importlib
import sys
from pathlib import Path
from types import ModuleType
from typing import Any

from fusion_framework._fusion import Settings, settings

__all__ = [
    "Settings",
    "settings",
    "configure",
    "get_settings",
    "load_settings_module",
]


def _main_dir() -> list[str]:
    main = sys.modules.get("__main__")
    main_file = getattr(main, "__file__", None)
    if main_file:
        return [str(Path(main_file).resolve().parent)]
    return []


def configure(**values: Any) -> Settings:
    settings.configure(**values)
    return settings


def get_settings() -> Settings:
    settings.ensure_loaded(_main_dir())
    return settings


def load_settings_module(module: str | ModuleType = "settings") -> Settings:
    """Load JSON via Rust, then overlay UPPERCASE attrs from a Python module."""
    settings.load_json(None, None, _main_dir())

    if isinstance(module, str):
        module = _import_settings_module(module)

    extras: dict[str, Any] = {}
    for name, value in vars(module).items():
        if name.startswith("_") or not name.isupper():
            continue
        extras[name.lower()] = value
        extras[name] = value

    if extras:
        settings.merge(extras)

    # Apply cache.* from the loaded env JSON (default driver: moka).
    try:
        from fusion_framework.cache import configure as configure_cache

        configure_cache(settings)
    except Exception:
        # Cache is optional at import time; first use will ensure_configured.
        pass

    return settings


def _import_settings_module(name: str) -> ModuleType:
    candidates = [name]
    if name == "settings":
        candidates.append("core.settings")
    elif "." not in name:
        candidates.append(f"core.{name}")

    errors: list[ModuleNotFoundError] = []
    for candidate in candidates:
        try:
            return importlib.import_module(candidate)
        except ModuleNotFoundError as exc:
            errors.append(exc)
    raise errors[0]
