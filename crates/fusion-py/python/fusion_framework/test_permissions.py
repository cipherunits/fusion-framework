"""Route permission checks and Swagger security metadata."""

from fusion_framework._fusion import openapi_spec
from fusion_framework.api import FusionBaseApi
from fusion_framework.middleware import clear_active_global, dispatch_route, require_permissions
from fusion_framework.route import clear_registry, route


def _handler(request):
    return {"status": 200, "body": {"ok": True}}


def setup_function():
    clear_registry()
    clear_active_global()


def teardown_function():
    clear_registry()
    clear_active_global()


def test_require_permissions_blocks():
    chain = [require_permissions(lambda req: False)]
    result = dispatch_route({"headers": {}, "method": "GET"}, _handler, chain)
    assert result["status"] == 403


def test_require_permissions_allows():
    chain = [require_permissions(lambda req: True)]
    result = dispatch_route({"headers": {}, "method": "GET"}, _handler, chain)
    assert result["status"] == 200


def test_openapi_marks_protected_routes():
    def is_admin(_request):
        return True

    @route("/api/admin", permissions=[is_admin])
    class Admin(FusionBaseApi):
        def get(self):
            return {"ok": True}

    @route("/api/public")
    class Public(FusionBaseApi):
        def get(self):
            return {"ok": True}

    spec = openapi_spec()
    admin_op = spec["paths"]["/api/admin"]["get"]
    public_op = spec["paths"]["/api/public"]["get"]

    assert "security" in admin_op
    assert admin_op["security"] == [{"FusionPermissions": []}]
    assert "security" not in public_op
    assert "FusionPermissions" in spec["components"]["securitySchemes"]
