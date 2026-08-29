"""Built-in security / CORS / cache / request-id middleware (settings-driven).

Public import path::

    from fusion_framework.middleware import security_headers, cors, cache_headers, request_id
"""

from __future__ import annotations

import uuid
from typing import Any, Callable, Mapping

from fusion_framework.middleware import Middleware, RequestDict, _call_next_merge_headers


def _as_dict(value: Any) -> dict[str, Any]:
    return dict(value) if isinstance(value, Mapping) else {}


def _as_list(value: Any) -> list[str]:
    if value is None:
        return []
    if isinstance(value, str):
        return [p.strip() for p in value.split(",") if p.strip()]
    if isinstance(value, (list, tuple, set)):
        return [str(v).strip() for v in value if str(v).strip()]
    return [str(value)]


def _truthy(value: Any, default: bool = False) -> bool:
    if value is None:
        return default
    if isinstance(value, bool):
        return value
    if isinstance(value, (int, float)):
        return value != 0
    if isinstance(value, str):
        return value.strip().lower() not in ("", "0", "false", "off", "no")
    return bool(value)


def _section(settings: Any, *keys: str) -> dict[str, Any]:
    if settings is None:
        return {}
    for key in keys:
        try:
            raw = settings.get(key)
        except Exception:
            raw = None
        if isinstance(raw, Mapping):
            return dict(raw)
    return {}


def _header_get(headers: Mapping[str, Any], name: str) -> str | None:
    target = name.lower()
    for key, value in headers.items():
        if str(key).lower() == target:
            return None if value is None else str(value)
    return None


def security_headers(config: Mapping[str, Any] | None = None) -> Middleware:
    """Modern browser security headers (enable HSTS only in production HTTPS)."""
    cfg = _as_dict(config)
    headers: dict[str, str] = {
        "X-Content-Type-Options": str(
            cfg.get("content_type_options") or cfg.get("x_content_type_options") or "nosniff"
        ),
        "X-Frame-Options": str(cfg.get("frame_options") or cfg.get("x_frame_options") or "DENY"),
        "Referrer-Policy": str(cfg.get("referrer_policy") or "strict-origin-when-cross-origin"),
        "Permissions-Policy": str(
            cfg.get("permissions_policy")
            or "camera=(), microphone=(), geolocation=(), payment=()"
        ),
        # Modern browsers ignore X-XSS-Protection; set 0 to disable legacy XSS auditor.
        "X-XSS-Protection": str(cfg.get("xss_protection") or "0"),
        "Cross-Origin-Opener-Policy": str(cfg.get("coop") or "same-origin"),
        "Cross-Origin-Resource-Policy": str(cfg.get("corp") or "same-origin"),
    }

    csp = cfg.get("csp") or cfg.get("content_security_policy")
    if csp:
        headers["Content-Security-Policy"] = str(csp)

    hsts = _as_dict(cfg.get("hsts"))
    if _truthy(hsts.get("enabled"), False):
        max_age = int(hsts.get("max_age") or 31536000)
        parts = [f"max-age={max_age}"]
        if _truthy(hsts.get("include_subdomains"), True):
            parts.append("includeSubDomains")
        if _truthy(hsts.get("preload"), False):
            parts.append("preload")
        headers["Strict-Transport-Security"] = "; ".join(parts)

    for key, value in _as_dict(cfg.get("headers")).items():
        if value is None:
            headers.pop(str(key), None)
        else:
            headers[str(key)] = str(value)

    def middleware(request: RequestDict, call_next: Callable[[RequestDict], Any]) -> Any:
        return _call_next_merge_headers(call_next, request, headers)

    return middleware


def cors(config: Mapping[str, Any] | None = None) -> Middleware:
    """CORS middleware — configure ``middleware.cors`` in ``fusion.<env>.json``."""
    cfg = _as_dict(config)
    allow_origins = _as_list(cfg.get("allow_origins") or cfg.get("origins") or ["*"])
    allow_methods = _as_list(
        cfg.get("allow_methods")
        or ["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS", "HEAD"]
    )
    allow_headers = _as_list(
        cfg.get("allow_headers")
        or ["Authorization", "Content-Type", "Accept", "Origin", "X-Request-Id"]
    )
    expose_headers = _as_list(cfg.get("expose_headers") or ["X-Request-Id"])
    allow_credentials = _truthy(cfg.get("allow_credentials"), False)
    max_age = int(cfg.get("max_age") or 600)

    def _cors_headers(origin: str | None) -> dict[str, str] | None:
        if not allow_origins:
            return None
        wildcard = "*" in allow_origins
        if origin and (wildcard or origin in allow_origins):
            allow = "*" if (wildcard and not allow_credentials) else origin
        elif wildcard and not allow_credentials and origin is None:
            allow = "*"
        elif wildcard and not allow_credentials:
            allow = "*"
        else:
            return None

        headers = {
            "Access-Control-Allow-Origin": allow,
            "Access-Control-Allow-Methods": ", ".join(allow_methods),
            "Access-Control-Allow-Headers": ", ".join(allow_headers),
            "Access-Control-Max-Age": str(max_age),
            "Vary": "Origin",
        }
        if expose_headers:
            headers["Access-Control-Expose-Headers"] = ", ".join(expose_headers)
        if allow_credentials:
            headers["Access-Control-Allow-Credentials"] = "true"
        return headers

    def middleware(request: RequestDict, call_next: Callable[[RequestDict], Any]) -> Any:
        origin = _header_get(request.get("headers") or {}, "Origin")
        cors_hdrs = _cors_headers(origin)
        method = str(request.get("method") or "").upper()
        if method == "OPTIONS":
            if cors_hdrs is None:
                return {"status": 403, "body": {"detail": "CORS origin not allowed"}}
            return {"status": 204, "body": "", "headers": cors_hdrs}
        if cors_hdrs is None:
            return call_next(request)
        return _call_next_merge_headers(call_next, request, cors_hdrs)

    return middleware


def cache_headers(config: Mapping[str, Any] | None = None) -> Middleware:
    """Default ``Cache-Control`` for API responses (override with ``@add_header``)."""
    cfg = _as_dict(config)
    value = str(cfg.get("default") or cfg.get("cache_control") or "no-store")
    extra = {"Cache-Control": value}
    pragma = cfg.get("pragma")
    if pragma:
        extra["Pragma"] = str(pragma)
    elif value == "no-store":
        extra["Pragma"] = "no-cache"

    def middleware(request: RequestDict, call_next: Callable[[RequestDict], Any]) -> Any:
        return _call_next_merge_headers(call_next, request, extra)

    return middleware


def request_id(config: Mapping[str, Any] | None = None) -> Middleware:
    """Propagate or generate ``X-Request-Id`` (also on ``request['state']['request_id']``)."""
    cfg = _as_dict(config)
    header_name = str(cfg.get("header") or "X-Request-Id")
    accept_incoming = _truthy(cfg.get("incoming"), True)
    state_key = str(cfg.get("state_key") or "request_id")

    def middleware(request: RequestDict, call_next: Callable[[RequestDict], Any]) -> Any:
        headers = request.get("headers") or {}
        rid = _header_get(headers, header_name) if accept_incoming else None
        if not rid:
            rid = uuid.uuid4().hex
        state = request.get("state")
        if not isinstance(state, dict):
            state = {}
            request["state"] = state
        state[state_key] = rid
        return _call_next_merge_headers(call_next, request, {header_name: rid})

    return middleware


def default_builtin_middleware(settings: Any) -> list[Middleware]:
    """Build built-in middleware from ``middleware.*`` settings (opt-in).

    Register with ``app.use()`` — nothing is installed automatically::

        for mw in default_builtin_middleware(settings):
            app.use(mw)

    When this helper is used, defaults if a key is omitted: security + request_id on;
    cors + cache off.
    """
    root = _section(settings, "middleware")

    def cfg_and_enabled(name: str, default_enabled: bool) -> tuple[dict[str, Any], bool]:
        cfg = _as_dict(root.get(name)) if root else _section(settings, name)
        if "enabled" in cfg:
            return cfg, _truthy(cfg.get("enabled"), default_enabled)
        if root:
            # middleware block present but this key omitted → use default
            return cfg, default_enabled if name not in root else _truthy(
                cfg.get("enabled"), default_enabled
            )
        return cfg, default_enabled

    out: list[Middleware] = []

    cfg, on = cfg_and_enabled("security", True)
    if on:
        out.append(security_headers(cfg))

    cfg, on = cfg_and_enabled("cors", False)
    if on:
        out.append(cors(cfg))

    cfg, on = cfg_and_enabled("cache", False)
    if on:
        out.append(cache_headers(cfg))

    cfg, on = cfg_and_enabled("request_id", True)
    if on:
        out.append(request_id(cfg))

    return out


__all__ = [
    "security_headers",
    "cors",
    "cache_headers",
    "request_id",
    "default_builtin_middleware",
]
