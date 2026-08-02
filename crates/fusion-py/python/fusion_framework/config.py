from __future__ import annotations

import importlib
import json
import os
import sys
from pathlib import Path
from types import ModuleType
from typing import Any, Mapping


def _resolve_value(value: Any) -> Any:
    """Resolve ALL_CAPS string placeholders from environment variables."""
    if isinstance(value, str) and value.isupper() and value.isidentifier() and len(value) > 1:
        return os.environ.get(value, value)
    return value


def _normalize_key(key: str) -> str:
    return key.strip().replace("-", "_").lower()


class Settings:
    """Runtime settings loaded from ``fusion.<env>.json`` and/or a Python module.

    Usage::

        from fusion_framework import settings

        debug = settings.get("debug", default=True)
        secret = settings.get("secret_key")
    """

    def __init__(self) -> None:
        self._config: dict[str, Any] = {}
        self._raw: dict[str, Any] = {}
        self._env: str = os.environ.get("FUSION_ENV", "dev")
        self._loaded_from: list[str] = []
        self._auto_loaded = False

    def get(self, key: str, default: Any = None, *, defualt: Any = None) -> Any:
        """Return a config value from JSON ``config`` (and module overlays).

        Supports dotted paths, e.g. ``settings.get("commands.run")``.
        ``defualt`` is accepted as an alias for ``default``.
        """
        self._ensure_loaded()
        if defualt is not None and default is None:
            default = defualt

        if "." in key:
            return self._get_dotted(key, default)

        normalized = _normalize_key(key)
        for candidate in (key, normalized, key.upper(), key.lower()):
            if candidate in self._config:
                return _resolve_value(self._config[candidate])

        for store_key, value in self._config.items():
            if _normalize_key(str(store_key)) == normalized:
                return _resolve_value(value)

        return default

    def __getitem__(self, key: str) -> Any:
        self._ensure_loaded()
        normalized = _normalize_key(key)
        if not any(_normalize_key(str(k)) == normalized for k in self._config):
            if "." in key:
                value = self._get_dotted(key, None)
                if value is not None:
                    return value
            raise KeyError(key)
        return self.get(key)

    def __contains__(self, key: object) -> bool:
        if not isinstance(key, str):
            return False
        self._ensure_loaded()
        normalized = _normalize_key(key)
        return any(_normalize_key(str(k)) == normalized for k in self._config)

    @property
    def env(self) -> str:
        self._ensure_loaded()
        return self._env

    @property
    def config(self) -> dict[str, Any]:
        self._ensure_loaded()
        return dict(self._config)

    @property
    def raw(self) -> dict[str, Any]:
        self._ensure_loaded()
        return dict(self._raw)

    @property
    def host(self) -> str:
        return str(self.get("host", default="127.0.0.1"))

    @property
    def port(self) -> int:
        return int(self.get("port", default=3000))

    @property
    def debug(self) -> bool:
        return bool(self.get("debug", default=False))

    def load_json(self, path: str | Path | None = None, *, env: str | None = None) -> "Settings":
        """Load a ``fusion.<env>.json`` file (or an explicit path)."""
        if path is None:
            env_name = env or os.environ.get("FUSION_ENV", self._env or "dev")
            path = self._find_json_file(env_name)
            if path is None:
                self._auto_loaded = True
                return self
            self._env = env_name
        else:
            path = Path(path)
            if not path.is_file():
                raise FileNotFoundError(f"settings json not found: {path}")

        data = json.loads(path.read_text(encoding="utf-8"))
        if not isinstance(data, dict):
            raise ValueError(f"settings json must be an object: {path}")

        self._raw = data
        if isinstance(data.get("env"), str):
            self._env = data["env"]

        config = data.get("config")
        if isinstance(config, dict):
            self._merge(config)
        else:
            skip = {"env", "commands"}
            self._merge({k: v for k, v in data.items() if k not in skip})

        if isinstance(data.get("commands"), dict):
            self._config.setdefault("commands", data["commands"])

        self._loaded_from.append(str(path.resolve()))
        self._auto_loaded = True
        return self

    def load_module(self, module: str | ModuleType = "settings") -> "Settings":
        """Load UPPERCASE attributes from a Python settings module into the store."""
        if isinstance(module, str):
            module = importlib.import_module(module)

        extras: dict[str, Any] = {}
        for name, value in vars(module).items():
            if name.startswith("_") or not name.isupper():
                continue
            extras[_normalize_key(name)] = value
            extras[name] = value

        self._merge(extras)
        self._loaded_from.append(getattr(module, "__file__", repr(module)))
        self._auto_loaded = True
        return self

    def configure(self, **values: Any) -> "Settings":
        self._ensure_loaded()
        normalized: dict[str, Any] = {}
        for key, value in values.items():
            normalized[key] = value
            normalized[_normalize_key(key)] = value
        self._merge(normalized)
        return self

    def clear(self) -> None:
        self._config.clear()
        self._raw.clear()
        self._loaded_from.clear()
        self._auto_loaded = False
        self._env = os.environ.get("FUSION_ENV", "dev")

    def _ensure_loaded(self) -> None:
        if self._auto_loaded:
            return
        self.load_json()

    def _merge(self, values: Mapping[str, Any]) -> None:
        for key, value in values.items():
            self._config[key] = value
            self._config[_normalize_key(str(key))] = value

    def _get_dotted(self, key: str, default: Any) -> Any:
        parts = key.split(".")

        def dig(root: Mapping[str, Any]) -> Any:
            cursor: Any = root
            for part in parts:
                if not isinstance(cursor, Mapping):
                    return None
                if part in cursor:
                    cursor = cursor[part]
                    continue
                match = next(
                    (
                        cursor[k]
                        for k in cursor
                        if _normalize_key(str(k)) == _normalize_key(part)
                    ),
                    None,
                )
                if match is None:
                    return None
                cursor = match
            return cursor

        for root in (self._raw, self._config):
            found = dig(root)
            if found is not None:
                return _resolve_value(found)
        return default

    def _find_json_file(self, env_name: str) -> Path | None:
        filename = f"fusion.{env_name}.json"
        for root in self._search_roots():
            candidate = root / filename
            if candidate.is_file():
                return candidate
        for root in self._search_roots():
            matches = sorted(root.glob("fusion.*.json"))
            if matches:
                return matches[0]
        return None

    def _search_roots(self) -> list[Path]:
        roots: list[Path] = [Path.cwd(), *Path.cwd().parents]
        main = sys.modules.get("__main__")
        main_file = getattr(main, "__file__", None)
        if main_file:
            roots.append(Path(main_file).resolve().parent)

        seen: set[Path] = set()
        unique: list[Path] = []
        for root in roots:
            resolved = root.resolve()
            if resolved in seen:
                continue
            seen.add(resolved)
            unique.append(resolved)
        return unique

    def __repr__(self) -> str:
        keys = sorted({_normalize_key(str(k)) for k in self._config})
        return f"<Settings env={self._env!r} keys={keys}>"


settings = Settings()


def configure(**values: Any) -> Settings:
    return settings.configure(**values)


def load_settings_module(module: str | ModuleType = "settings") -> Settings:
    """Load JSON (if present) then overlay a Python settings module."""
    settings.load_json()
    settings.load_module(module)
    return settings


def get_settings() -> Settings:
    settings._ensure_loaded()
    return settings
