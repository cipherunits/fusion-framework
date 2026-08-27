"""Pagination tests."""

from fusion_framework._fusion import clear_routes
from fusion_framework.api import FusionBaseApi
from fusion_framework.http import HTTPException
from fusion_framework.pagination import paginated_body, parse_pagination
from fusion_framework.route import route


def setup_function():
    clear_routes()


def teardown_function():
    clear_routes()


def test_parse_pagination_defaults():
    params = parse_pagination({})
    assert params.page == 1
    assert params.page_size == 20
    assert params.offset == 0


def test_parse_pagination_page_and_size():
    params = parse_pagination({"page": "3", "page_size": "10"})
    assert params.page == 3
    assert params.page_size == 10
    assert params.offset == 20


def test_parse_pagination_handler_override():
    params = parse_pagination({"page": "1"}, page=2, page_size=5)
    assert params.page == 2
    assert params.page_size == 5
    assert params.offset == 5


def test_paginated_body_shape():
    params = parse_pagination({"page": "2", "page_size": "10"})
    body = paginated_body([1, 2, 3], 25, params)
    assert body["items"] == [1, 2, 3]
    meta = body["pagination"]
    assert meta["total"] == 25
    assert meta["total_pages"] == 3
    assert meta["has_next"] is True
    assert meta["has_prev"] is True


def test_fusion_base_api_paginated_response():
    @route("/api/[module]")
    class ProductModule(FusionBaseApi):
        def get(self, page: int = 1, page_size: int = 10):
            items = list(range(1, 6))
            return self.paginated(items, total=25, page=page, page_size=page_size)

    from fusion_framework._fusion import openapi_spec

    spec = openapi_spec()
    assert "/api/product" in spec["paths"]
    get_op = spec["paths"]["/api/product"]["get"]
    param_names = {p["name"] for p in get_op.get("parameters", [])}
    assert {"page", "page_size"}.issubset(param_names)


def test_invalid_page_raises_http_exception():
    try:
        parse_pagination({"page": "0"})
        assert False, "expected HTTPException"
    except HTTPException as exc:
        assert exc.status == 400
