"""Tests for built-in middleware and add/delete header ops."""

import asyncio
import inspect

from fusion_framework.middleware import request_id, security_headers
from fusion_framework.header import cache_control
from fusion_framework.header_ops import add_header, delete_header
from fusion_framework.middleware import (
    clear_active_global,
    dispatch_route,
    framework_headers,
    set_active_global,
)


def _handler(_request):
    return {"status": 200, "body": {"ok": True}, "headers": {"X-Handler": "1"}}


def test_security_headers_merge():
    clear_active_global()
    set_active_global([security_headers()])
    try:
        result = dispatch_route({"headers": {}, "method": "GET"}, _handler, [])
        headers = {str(k).lower(): v for k, v in (result.get("headers") or {}).items()}
        assert headers["x-content-type-options"] == "nosniff"
        assert headers["x-frame-options"] == "DENY"
        assert headers["referrer-policy"] == "strict-origin-when-cross-origin"
        assert headers["x-handler"] == "1"
    finally:
        clear_active_global()


def test_request_id_sets_state_and_header():
    clear_active_global()
    set_active_global([request_id()])

    def handler(request):
        return {
            "status": 200,
            "body": {"rid": request["state"]["request_id"]},
        }

    try:
        result = dispatch_route(
            {"headers": {"X-Request-Id": "abc123"}, "method": "GET", "state": {}},
            handler,
            [],
        )
        assert result["body"]["rid"] == "abc123"
        headers = {str(k).lower(): v for k, v in (result.get("headers") or {}).items()}
        assert headers["x-request-id"] == "abc123"
    finally:
        clear_active_global()


def test_delete_header_strips_and_suppresses():
    @delete_header("X-Powered-By")
    def get(_self=None):
        return {"status": 200, "body": "ok", "headers": {"X-Powered-By": "Fusion Framework"}}

    result = get()
    headers = result.get("headers") or {}
    assert "X-Powered-By" not in headers
    assert "X-Powered-By" in (result.get("suppress_headers") or [])


def test_add_header_merges_maps_and_pairs():
    @add_header(cache_control("no-store"), "X-Demo", "yes")
    def get(_self=None):
        return {"status": 200, "body": "ok"}

    result = get()
    headers = result.get("headers") or {}
    assert headers["Cache-Control"] == "no-store"
    assert headers["X-Demo"] == "yes"


def test_add_header_async():
    @add_header(**{"X-Async": "1"})
    async def get():
        return {"status": 200, "body": "ok"}

    assert inspect.iscoroutinefunction(get)
    result = asyncio.run(get())
    assert result["headers"]["X-Async"] == "1"


def test_delete_header_survives_framework_middleware():
    clear_active_global()
    set_active_global([framework_headers(), security_headers()])

    @delete_header("X-Powered-By")
    def get(_request):
        return {"status": 200, "body": {"ok": True}}

    try:
        result = dispatch_route({"headers": {}, "method": "GET"}, get, [])
        headers = {str(k).lower(): v for k, v in (result.get("headers") or {}).items()}
        assert "x-powered-by" not in headers
        assert "x-content-type-options" in headers
        assert "X-Powered-By" in (result.get("suppress_headers") or [])
    finally:
        clear_active_global()
