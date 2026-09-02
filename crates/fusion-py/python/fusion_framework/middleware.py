"""Global and route middleware for Fusion Framework."""

from __future__ import annotations

import base64
import inspect
import json
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
