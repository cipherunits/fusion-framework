"""Unit tests for middleware chain (no server required)."""

from fusion_framework.middleware import bearer_jwt, dispatch_route, require_roles, set_active_global


def _handler(request):
    return {"status": 200, "body": {"state": request.get("state", {})}}


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
