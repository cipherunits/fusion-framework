"""Tests for custom HTTP method routes (@http_get / HttpGet)."""

from fusion_framework._fusion import clear_routes
from fusion_framework.api import FusionBaseApi
from fusion_framework.http_route import http_get
from fusion_framework.route import route


def setup_function():
    clear_routes()


def teardown_function():
    clear_routes()


def test_custom_http_get_with_action_token():
    @route("/api/[module]")
    class UserModule(FusionBaseApi):
        @http_get("test/[action]")
        def UserAction(self):
            return {"ok": True}

    from fusion_framework._fusion import openapi_spec

    spec = openapi_spec()
    assert "/api/user/test/user" in spec["paths"]
    assert "get" in spec["paths"]["/api/user/test/user"]
    assert spec["paths"]["/api/user/test/user"]["get"]["operationId"] == "UserModule_UserAction"


def test_convention_and_custom_routes_coexist():
    @route("/api/[module]")
    class ProductModule(FusionBaseApi):
        def get(self):
            return {"mode": "convention"}

        @http_get("catalog/[action]")
        def ListAction(self):
            return {"mode": "custom"}

    from fusion_framework._fusion import openapi_spec

    spec = openapi_spec()
    assert "/api/product" in spec["paths"]
    assert "get" in spec["paths"]["/api/product"]
    assert "/api/product/catalog/list" in spec["paths"]


def test_custom_http_inherits_class_tags():
    @route("/api/[module]", tags=["products"])
    class ProductModule(FusionBaseApi):
        def get(self):
            return {"mode": "convention"}

        @http_get("catalog/[action]", title="Product catalog")
        def CatalogAction(self):
            return {"items": []}

        @http_get("admin/[action]", tags=["admin"])
        def AdminAction(self):
            return {"ok": True}

    from fusion_framework._fusion import openapi_spec

    spec = openapi_spec()
    convention = spec["paths"]["/api/product"]["get"]
    catalog = spec["paths"]["/api/product/catalog/catalog"]["get"]
    admin = spec["paths"]["/api/product/admin/admin"]["get"]
    assert convention["tags"] == ["products"]
    assert catalog["tags"] == ["products"]
    assert catalog["summary"] == "Product catalog"
    assert admin["tags"] == ["admin"]
