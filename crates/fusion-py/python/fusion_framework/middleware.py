"""Global and route middleware for Fusion Framework."""

from __future__ import annotations

import base64
import inspect
import json
from pathlib import Path
from typing import Any, Awaitable, Callable, Iterable, Union

Middleware = Callable[..., Any]
RequestDict = dict[str, Any]
MaybeAwaitable = Union[Any, Awaitable[Any]]

_active_global: list[Middleware] = []


def use(middleware: Middleware) -> Middleware:
    """Register global middleware on the active app (called by ``FusionApp.use``)."""
    _active_global.append(middleware)
    return middleware


def set_active_global(middlewares: Iterable[Middleware]) -> None:
    global _active_global
    _active_global = list(middlewares)


def get_active_global() -> list[Middleware]:
    return list(_active_global)


def clear_active_global() -> None:
    _active_global.clear()


async def _maybe_await(value: MaybeAwaitable) -> Any:
    if inspect.isawaitable(value):
        return await value
    return value


def _ensure_state(request: RequestDict) -> dict[str, Any]:
    state = request.get("state")
    if not isinstance(state, dict):
        state = {}
        request["state"] = state
    return state


def _is_response(value: Any) -> bool:
    return isinstance(value, dict) and "status" in value


def _middleware_is_async(middleware: Middleware) -> bool:
    return inspect.iscoroutinefunction(middleware)


def _chain_needs_async(middlewares: list[Middleware], handler: Callable[..., Any]) -> bool:
    if inspect.iscoroutinefunction(handler):
        return True
    return any(_middleware_is_async(mw) for mw in middlewares)


def _run_chain_sync(
    request: RequestDict,
    middlewares: list[Middleware],
    handler: Callable[[RequestDict], Any],
) -> Any:
    _ensure_state(request)

    def dispatch(index: int, req: RequestDict) -> Any:
        if index >= len(middlewares):
            return handler(req)
        middleware = middlewares[index]

        def call_next(next_req: RequestDict) -> Any:
            return dispatch(index + 1, next_req)

        result = middleware(req, call_next)
        # Sync middleware may return a coroutine when the route handler is
        # ``async def`` (HandlerInvoker is sync but yields an awaitable).
        # Propagate to Rust so ``async_runtime.submit`` can await it.
        if inspect.isawaitable(result):
            return result
        if _is_response(result):
            return result
        return result

    return dispatch(0, request)


async def run_chain(
    request: RequestDict,
    middlewares: list[Middleware],
    handler: Callable[[RequestDict], MaybeAwaitable],
) -> Any:
    _ensure_state(request)

    async def dispatch(index: int, req: RequestDict) -> Any:
        if index >= len(middlewares):
            return await _maybe_await(handler(req))
        middleware = middlewares[index]

        async def call_next(next_req: RequestDict) -> Any:
            return await dispatch(index + 1, next_req)

        result = middleware(req, call_next)
        result = await _maybe_await(result)
        if _is_response(result):
            return result
        return result

    return await dispatch(0, request)


def require_permissions(*checks: Callable[[RequestDict], Any]) -> Middleware:
    """Route middleware: run custom permission checks; any falsy result → 403.

    Each check receives the request dict (``method``, ``path``, ``headers``,
    ``body``, ``state``, …). Omit checks to allow any caller (default).
    """

    def middleware(request: RequestDict, call_next: Callable[[RequestDict], Any]) -> Any:
        for check in checks:
            if not check(request):
                return {"status": 403, "body": {"detail": "Forbidden"}}
        return call_next(request)

    return middleware


def require_roles(
    *roles: str,
    claim: str = "roles",
    state_key: str = "jwt",
) -> Middleware:
    """Route middleware: allow only requests whose JWT/state payload includes a role."""
    allowed = {str(r) for r in roles}

    def middleware(request: RequestDict, call_next: Callable[[RequestDict], Any]) -> Any:
        state = _ensure_state(request)
        payload = state.get(state_key)
        if payload is None:
            return {"status": 401, "body": {"detail": "Authentication required"}}

        user_roles = payload.get(claim) if isinstance(payload, dict) else None
        if user_roles is None:
            return {"status": 403, "body": {"detail": f"Missing '{claim}' claim"}}

        if isinstance(user_roles, str):
            user_roles = [user_roles]
        if not isinstance(user_roles, (list, tuple, set)):
            return {"status": 403, "body": {"detail": f"Invalid '{claim}' claim"}}

        if not allowed.intersection({str(r) for r in user_roles}):
            return {
                "status": 403,
                "body": {
                    "detail": "Insufficient permissions",
                    "required": sorted(allowed),
                },
            }
        return call_next(request)

    return middleware


def bearer_jwt(
    *,
    state_key: str = "jwt",
    header: str = "Authorization",
    verify: Callable[[str], dict[str, Any] | None] | None = None,
) -> Middleware:
    """Extract Bearer token and store decoded payload in ``request['state']``."""

    def _decode_unverified(token: str) -> dict[str, Any] | None:
        try:
            parts = token.split(".")
            if len(parts) != 3:
                return None
            payload_b64 = parts[1] + "=" * (-len(parts[1]) % 4)
            raw = base64.urlsafe_b64decode(payload_b64.encode("ascii"))
            parsed = json.loads(raw.decode("utf-8"))
            return parsed if isinstance(parsed, dict) else None
        except (ValueError, json.JSONDecodeError, UnicodeDecodeError):
            return None

    header_keys = {header, header.lower(), header.upper()}

    def middleware(request: RequestDict, call_next: Callable[[RequestDict], Any]) -> Any:
        headers = request.get("headers") or {}
        auth = None
        for key, value in headers.items():
            if str(key) in header_keys:
                auth = value
                break

        if not auth or not str(auth).lower().startswith("bearer "):
            return {"status": 401, "body": {"detail": "Missing bearer token"}}

        token = str(auth)[7:].strip()
        payload = verify(token) if verify else _decode_unverified(token)
        if payload is None:
            return {"status": 401, "body": {"detail": "Invalid token"}}

        _ensure_state(request)[state_key] = payload
        return call_next(request)

    return middleware


def _merge_response_headers(result: Any, extra: dict[str, str]) -> Any:
    if not isinstance(result, dict):
        return {"status": 200, "body": result, "headers": dict(extra)}
    headers = result.get("headers")
    if not isinstance(headers, dict):
        headers = {}
    merged = {**extra, **headers}  # handler wins on conflict
    out = dict(result)
    out["headers"] = merged
    return out


def _call_next_merge_headers(
    call_next: Callable[[RequestDict], Any],
    request: RequestDict,
    extra: dict[str, str],
) -> Any:
    """Merge headers after ``call_next``, awaiting async handler results."""
    result = call_next(request)
    if inspect.isawaitable(result):

        async def _await_merge() -> Any:
            return _merge_response_headers(await result, extra)

        return _await_merge()
    return _merge_response_headers(result, extra)


def framework_headers() -> Middleware:
    """Optional identity middleware: advertise Fusion on responses.

    Adds ``X-Powered-By``, ``X-Framework``, and ``X-Fusion-Version``.
    Not enabled by default — add with ``app.use(framework_headers())``.
    Wire-level injection is off unless ``fingerprint.enabled: true`` in settings.
    """
    try:
        from fusion_framework._fusion import fingerprint_headers as _fp

        extra = dict(_fp())
    except Exception:
        try:
            from fusion_framework import header as hdr

            extra = {
                getattr(hdr, "X_POWERED_BY", "X-Powered-By"): getattr(
                    hdr, "FRAMEWORK_POWERED_BY", "Fusion Framework"
                ),
                getattr(hdr, "X_FRAMEWORK", "X-Framework"): getattr(hdr, "FRAMEWORK_ID", "Fusion"),
                getattr(hdr, "X_FUSION_VERSION", "X-Fusion-Version"): "1.2.6",
            }
        except Exception:
            extra = {
                "X-Powered-By": "Fusion Framework",
                "X-Framework": "Fusion",
                "X-Fusion-Version": "1.2.6",
            }

    def middleware(request: RequestDict, call_next: Callable[[RequestDict], Any]) -> Any:
        return _call_next_merge_headers(call_next, request, extra)

    return middleware


def _get_header(request: RequestDict, name: str) -> str | None:
    headers = request.get("headers") or {}
    target = name.lower()
    for key, value in headers.items():
        if str(key).lower() == target:
            return str(value)
    return None


def security_headers(
    *,
    content_type_options: str = "nosniff",
    frame_options: str = "DENY",
    referrer_policy: str = "strict-origin-when-cross-origin",
    permissions_policy: str = "camera=(), microphone=(), geolocation=(), payment=()",
    coop: str = "same-origin",
    corp: str = "same-origin",
    csp: str | None = None,
    hsts: str | None = None,
) -> Middleware:
    """Add common security response headers."""
    extra: dict[str, str] = {
        "X-Content-Type-Options": content_type_options,
        "X-Frame-Options": frame_options,
        "Referrer-Policy": referrer_policy,
        "Permissions-Policy": permissions_policy,
        "Cross-Origin-Opener-Policy": coop,
        "Cross-Origin-Resource-Policy": corp,
    }
    if csp:
        extra["Content-Security-Policy"] = csp
    if hsts:
        extra["Strict-Transport-Security"] = hsts

    def middleware(request: RequestDict, call_next: Callable[[RequestDict], Any]) -> Any:
        return _call_next_merge_headers(call_next, request, extra)

    return middleware


def cache_headers(*, default: str = "no-store") -> Middleware:
    """Set ``Cache-Control`` on responses that do not already define it."""
    extra = {"Cache-Control": default}

    def middleware(request: RequestDict, call_next: Callable[[RequestDict], Any]) -> Any:
        return _call_next_merge_headers(call_next, request, extra)

    return middleware


def request_id(
    *,
    header: str = "X-Request-Id",
    incoming: bool = True,
) -> Middleware:
    """Attach a request id to ``state`` and echo it on the response."""
    import uuid

    def middleware(request: RequestDict, call_next: Callable[[RequestDict], Any]) -> Any:
        state = _ensure_state(request)
        rid = _get_header(request, header) if incoming else None
        if not rid:
            rid = str(uuid.uuid4())
        state["request_id"] = rid
        return _call_next_merge_headers(call_next, request, {header: rid})

    return middleware


def cors(
    *,
    allow_origins: Iterable[str] | str = "*",
    allow_methods: Iterable[str] | None = None,
    allow_headers: Iterable[str] | None = None,
    expose_headers: Iterable[str] | None = None,
    allow_credentials: bool = False,
    max_age: int = 600,
) -> Middleware:
    """CORS middleware; short-circuits ``OPTIONS`` preflight with 204."""
    origins = (
        [str(allow_origins)]
        if isinstance(allow_origins, str)
        else [str(o) for o in allow_origins]
    )
    methods = (
        ["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS", "HEAD"]
        if allow_methods is None
        else [str(m).upper() for m in allow_methods]
    )
    headers = (
        ["Authorization", "Content-Type", "Accept", "Origin", "X-Request-Id"]
        if allow_headers is None
        else [str(h) for h in allow_headers]
    )
    expose = ["X-Request-Id"] if expose_headers is None else [str(h) for h in expose_headers]
    allow_all = "*" in origins

    def _cors_headers(origin: str | None) -> dict[str, str]:
        chosen = "*"
        if not allow_all:
            if origin and origin in origins:
                chosen = origin
            elif origins:
                chosen = origins[0]
        out = {
            "Access-Control-Allow-Origin": chosen,
            "Access-Control-Allow-Methods": ", ".join(methods),
            "Access-Control-Allow-Headers": ", ".join(headers),
            "Access-Control-Expose-Headers": ", ".join(expose),
            "Access-Control-Max-Age": str(max_age),
            "Vary": "Origin",
        }
        if allow_credentials and chosen != "*":
            out["Access-Control-Allow-Credentials"] = "true"
        return out

    def middleware(request: RequestDict, call_next: Callable[[RequestDict], Any]) -> Any:
        origin = _get_header(request, "Origin")
        extra = _cors_headers(origin)
        if str(request.get("method", "GET")).upper() == "OPTIONS":
            return {"status": 204, "body": "", "headers": extra}
        return _call_next_merge_headers(call_next, request, extra)

    return middleware


_STATIC_MIME_TYPES: dict[str, str] = {
    ".css": "text/css; charset=utf-8",
    ".gif": "image/gif",
    ".htm": "text/html; charset=utf-8",
    ".html": "text/html; charset=utf-8",
    ".ico": "image/x-icon",
    ".jpeg": "image/jpeg",
    ".jpg": "image/jpeg",
    ".js": "text/javascript; charset=utf-8",
    ".json": "application/json",
    ".map": "application/json",
    ".png": "image/png",
    ".svg": "image/svg+xml",
    ".txt": "text/plain; charset=utf-8",
    ".webp": "image/webp",
    ".woff": "font/woff",
    ".woff2": "font/woff2",
}


def _guess_static_content_type(path: Path) -> str:
    """Map a file extension to a Content-Type, defaulting to octet-stream."""
    return _STATIC_MIME_TYPES.get(path.suffix.lower(), "application/octet-stream")


def static_files(
    root: str | Path = "static",
    *,
    prefix: str = "/static",
    max_age: int | None = 3600,
    fallthrough: bool | None = None,
) -> Middleware:
    """Serve local files under ``root`` for URLs starting with ``prefix`` (WhiteNoise-style).

    - ``root``: folder on disk that holds the files (e.g. ``\"static\"`` or
      ``Path(__file__).parent / \"templates\" / \"home\"``).
    - ``prefix``: URL path prefix browsers request (e.g. ``\"/static\"`` so
      ``static/logo.png`` is served at ``/static/logo.png``). Use ``\"/\"`` to
      serve files at the site root (``templates/home/a.png`` → ``/a.png``).

    On ``FusionApp.listen()``, matching files are mounted as real GET/HEAD routes
    (middleware alone cannot see unmatched paths). Only ``GET`` / ``HEAD``.
    Path traversal is rejected. The returned middleware also short-circuits when
    invoked on an already-mounted path.
    """
    root_path = Path(root).expanduser()
    normalized = "/" + str(prefix).strip("/") if str(prefix).strip("/") else "/"
    allow_fallthrough = (normalized == "/") if fallthrough is None else bool(fallthrough)
    cfg = {
        "root": root_path,
        "prefix": normalized,
        "max_age": max_age,
        "fallthrough": allow_fallthrough,
    }

    def middleware(request: RequestDict, call_next: Callable[[RequestDict], Any]) -> Any:
        return _serve_static_or_next(cfg, request, call_next)

    middleware.__fusion_static__ = cfg  # type: ignore[attr-defined]
    return middleware


def _serve_static_or_next(
    cfg: dict[str, Any],
    request: RequestDict,
    call_next: Callable[[RequestDict], Any],
) -> Any:
    """Try to serve a file for this request; otherwise call the next handler."""
    method = str(request.get("method", "GET")).upper()
    if method not in ("GET", "HEAD"):
        return call_next(request)

    req_path = str(request.get("path") or "/")
    normalized = str(cfg["prefix"])
    if normalized == "/":
        relative = req_path.lstrip("/")
        if not relative or relative.endswith("/"):
            return call_next(request)
    else:
        if not (req_path == normalized or req_path.startswith(normalized + "/")):
            return call_next(request)
        relative = req_path[len(normalized) :].lstrip("/")
        if not relative:
            return call_next(request)

    base = Path(cfg["root"]).resolve()
    candidate = (base / relative).resolve()
    try:
        candidate.relative_to(base)
    except ValueError:
        return {"status": 403, "body": {"detail": "Forbidden"}}

    if not candidate.is_file():
        if cfg["fallthrough"]:
            return call_next(request)
        return {"status": 404, "body": {"detail": "Not found"}}

    return _static_file_response(candidate, method, cfg.get("max_age"))


def _static_file_response(path: Path, method: str, max_age: int | None) -> dict[str, Any]:
    """Build a 200 response envelope for a file on disk."""
    size = path.stat().st_size
    headers = {
        "content-type": _guess_static_content_type(path),
        "content-length": str(size),
    }
    if max_age is not None:
        headers["cache-control"] = f"public, max-age={int(max_age)}"
    body: Any = b"" if method.upper() == "HEAD" else path.read_bytes()
    return {"status": 200, "body": body, "headers": headers}


def mount_static_files(engine: Any, middlewares: Iterable[Middleware]) -> None:
    """Register GET/HEAD routes for every file under each ``static_files`` mount."""
    for middleware in middlewares:
        cfg = getattr(middleware, "__fusion_static__", None)
        if not isinstance(cfg, dict):
            continue
        root = Path(cfg["root"]).expanduser().resolve()
        if not root.is_dir():
            continue
        prefix = str(cfg["prefix"])
        max_age = cfg.get("max_age")
        for file_path in root.rglob("*"):
            if not file_path.is_file():
                continue
            rel = file_path.relative_to(root).as_posix()
            url = f"/{rel}" if prefix == "/" else f"{prefix}/{rel}"
            path_for_get = file_path
            path_for_head = file_path

            def _get(_req: RequestDict, p: Path = path_for_get, age: int | None = max_age) -> dict[str, Any]:
                return _static_file_response(p, "GET", age)

            def _head(_req: RequestDict, p: Path = path_for_head, age: int | None = max_age) -> dict[str, Any]:
                return _static_file_response(p, "HEAD", age)

            engine.route("GET", url, _get)
            engine.route("HEAD", url, _head)


def dispatch_route(
    request: RequestDict,
    handler: Callable[[RequestDict], Any],
    route_middleware: list[Middleware],
) -> Any:
    """Entry from Rust: global middleware → route middleware → API handler."""
    chain = get_active_global() + list(route_middleware)

    if _chain_needs_async(chain, handler):
        return run_chain(request, chain, handler)
    return _run_chain_sync(request, chain, handler)
