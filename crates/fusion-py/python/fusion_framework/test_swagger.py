"""OpenAPI specs are split per `@route(version=...)` for Swagger navbar switching."""

from fusion_framework._fusion import has_unversioned_routes, openapi_spec, route_versions
from fusion_framework.api import FusionBaseApi
from fusion_framework.app import UNVERSIONED_SWAGGER_NAME, _swagger_version_urls
from fusion_framework.route import clear_registry, route


def setup_function():
    clear_registry()


def teardown_function():
    clear_registry()


def test_route_versions_and_filtered_specs():
    @route("/hello", version="v1")
    class V1Hello(FusionBaseApi):
        def get(self):
            return {"v": 1}

    @route("/hello", version="v2")
    class V2Hello(FusionBaseApi):
        def get(self):
            return {"v": 2}

    @route("/health")
    class Health(FusionBaseApi):
        def get(self):
            return {"ok": True}

    assert route_versions() == ["v1", "v2"]
    assert has_unversioned_routes() is True

    v1 = openapi_spec("v1")
    v2 = openapi_spec("v2")
    unversioned = openapi_spec(UNVERSIONED_SWAGGER_NAME)
    combined = openapi_spec()

    assert "/v1/hello" in v1["paths"]
    assert "/v2/hello" not in v1["paths"]
    assert "/health" not in v1["paths"]

    assert "/v2/hello" in v2["paths"]
    assert "/v1/hello" not in v2["paths"]

    assert "/health" in unversioned["paths"]
    assert "/v1/hello" not in unversioned["paths"]

    assert "/v1/hello" in combined["paths"]
    assert "/v2/hello" in combined["paths"]
    assert "/health" in combined["paths"]


def test_navbar_urls_list_each_version():
    @route("/hello", version="v1")
    class V1Hello(FusionBaseApi):
        def get(self):
            return {"v": 1}

    @route("/hello", version="v2")
    class V2Hello(FusionBaseApi):
        def get(self):
            return {"v": 2}

    @route("/health")
    class Health(FusionBaseApi):
        def get(self):
            return {"ok": True}

    urls = _swagger_version_urls("/swagger")
    assert urls == [
        {"url": "/swagger/v1/openapi.json", "name": "v1"},
        {"url": "/swagger/v2/openapi.json", "name": "v2"},
        {"url": "/swagger/default/openapi.json", "name": "default"},
    ]
