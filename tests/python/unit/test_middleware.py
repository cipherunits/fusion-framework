"""Unit tests for middleware chain (no server required)."""

import asyncio
import inspect

from fusion_framework.middleware import (
    bearer_jwt,
    clear_active_global,
    cors,
    dispatch_route,
    framework_headers,
    request_id,
    require_roles,
    set_active_global,
)


def _handler(request):
    return {"status": 200, "body": {"state": request.get("state", {})}}


async def _async_handler(request):
    return {"status": 200, "body": {"ok": True, "path": request.get("path")}}


def _sync_invoker_like(request):
    """Mirrors PyO3 HandlerInvoker: sync ``__call__`` that may return a coroutine."""
    return _async_handler(request)


def test_require_roles_allows_matching_role():
    request = {"headers": {}, "state": {"jwt": {"roles": ["admin"]}}}
    chain = [require_roles("admin", "super_admin")]
    result = dispatch_route(request, _handler, chain)
    assert result["status"] == 200


def test_require_roles_blocks_missing_role():
    request = {"headers": {}, "state": {"jwt": {"roles": ["user"]}}}
    chain = [require_roles("admin")]
    result = dispatch_route(request, _handler, chain)
    assert result["status"] == 403


def test_bearer_jwt_populates_state():
    token = "eyJhbGciOiJub25lIiwidHlwIjoiSldUIn0.eyJzdWIiOiIxIiwicm9sZXMiOlsiYWRtaW4iXX0."
    request = {"headers": {"Authorization": f"Bearer {token}"}}
    set_active_global([bearer_jwt()])
    result = dispatch_route(request, _handler, [])
    assert result["status"] == 200
    assert result["body"]["state"]["jwt"]["sub"] == "1"


def test_no_middleware_by_default():
    """FusionApp does not inject framework headers unless explicitly added."""
    set_active_global([])
    request = {"path": "/", "headers": {}, "method": "GET"}
    result = dispatch_route(request, _handler, [])
    assert result["status"] == 200
    headers = {str(k).lower(): v for k, v in (result.get("headers") or {}).items()}
    assert "x-powered-by" not in headers


def test_request_id_header():
    set_active_global([request_id()])
    request = {"path": "/", "headers": {}, "method": "GET"}
    result = dispatch_route(request, _handler, [])
    headers = {str(k).lower(): v for k, v in (result.get("headers") or {}).items()}
    assert "x-request-id" in headers
    assert result["body"]["state"]["request_id"] == headers["x-request-id"]


def test_cors_options_preflight():
    set_active_global([cors()])
    request = {
        "path": "/api",
        "headers": {"Origin": "https://example.com"},
        "method": "OPTIONS",
    }
    result = dispatch_route(request, _handler, [])
    assert result["status"] == 204
    headers = {str(k).lower(): v for k, v in (result.get("headers") or {}).items()}
    assert headers.get("access-control-allow-origin")


def test_framework_headers_awaits_async_handler():
    """Regression: sync framework_headers must not stringify coroutine bodies."""
    set_active_global([framework_headers()])
    request = {"path": "/membership", "headers": {}, "method": "GET"}
    result = dispatch_route(request, _sync_invoker_like, [])
    assert inspect.isawaitable(result), "async handler result must stay awaitable for Rust"
    resolved = asyncio.run(result)
    assert resolved["status"] == 200
    assert resolved["body"] == {"ok": True, "path": "/membership"}
    headers = {str(k).lower(): v for k, v in (resolved.get("headers") or {}).items()}
    assert "x-powered-by" in headers
    body = resolved.get("body")
    assert not (isinstance(body, str) and body.startswith("<coroutine"))


def test_sync_middleware_propagates_async_handler_coroutine():
    request = {"path": "/x", "headers": {}, "state": {"jwt": {"roles": ["admin"]}}}
    result = dispatch_route(request, _sync_invoker_like, [require_roles("admin")])
    assert inspect.isawaitable(result)
    resolved = asyncio.run(result)
    assert resolved["body"]["ok"] is True
